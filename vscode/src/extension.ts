import * as vscode from "vscode";

import {
  spawn,
  ChildProcessWithoutNullStreams,
  ChildProcess,
} from "node:child_process";

import { analyze, isAlive } from "./api/request";
import { zWorkspace, zCollectedData, zInfer, zRange } from "./api/schemas";
import { selectLocal } from "./analyze";
import {
  commonRanges,
  eliminatedRanges,
  rangeToRange,
  excludeRanges,
} from "./range";

const DOCKER_CONTAINER_NAME = "rustowl-server";

export function activate(context: vscode.ExtensionContext) {
  console.log("rustowl activated");

  const lifetimeDecorationType = vscode.window.createTextEditorDecorationType({
    textDecoration: "underline solid 3px hsla(125, 80%, 60%, 0.8)",
  });
  const moveDecorationType = vscode.window.createTextEditorDecorationType({
    textDecoration: "underline solid 3px hsla(35, 80%, 60%, 0.8)",
  });
  const imBorrowDecorationType = vscode.window.createTextEditorDecorationType({
    textDecoration: "underline solid 3px hsla(230, 80%, 60%, 0.8)",
  });
  const mBorrowDecorationType = vscode.window.createTextEditorDecorationType({
    textDecoration: "underline solid 3px hsla(300, 80%, 60%, 0.8)",
  });
  const outLiveDecorationType = vscode.window.createTextEditorDecorationType({
    textDecoration: "underline solid 3px hsla(0, 80%, 60%, 0.8)",
  });
  const emptyDecorationType = vscode.window.createTextEditorDecorationType({});

  let analyzed: zInfer<typeof zWorkspace> | undefined = undefined;

  // update decoration
  const updateDecoration = (editor: vscode.TextEditor) => {
    let filepath = editor.document.fileName;
    const wsPath = vscode.workspace.workspaceFolders![0].uri.path;
    if (filepath.startsWith(wsPath)) {
      filepath = filepath.slice(wsPath.length + 1);
    }
    const thisFile = analyzed?.[filepath];
    if (!thisFile) {
      return;
    }
    console.log(filepath);
    type DecoInfo = {
      range: zInfer<typeof zRange>;
      hoverMessage?: string;
    };
    let lifetime: DecoInfo[] = [];
    let moves: DecoInfo[] = [];
    let imBorrows: DecoInfo[] = [];
    let mBorrows: DecoInfo[] = [];
    let outLives: DecoInfo[] = [];
    let messages: DecoInfo[] = [];
    //clearDecoration(editor);
    const cursor = editor.document.offsetAt(editor.selection.active);
    for (const itemId in thisFile.items) {
      const mir = thisFile.items[itemId];
      const getDeclFromLocal = (local: number) =>
        mir.decls.filter((v) => v.local_index === local).at(0);

      const locals = selectLocal(cursor, mir);
      const decls = mir.decls;
      const selectedDecls = decls
        .filter((v) => locals.includes(v.local_index))
        .map((v) => ({
          ...v,
          lives: eliminatedRanges(v.lives),
          must_live_at: eliminatedRanges(v.must_live_at),
        }));

      console.log(selectedDecls);

      const selectedLiveDecos = selectedDecls
        .map((v) =>
          //v.must_live_at.map((w) => ({
          v.lives
            .map((w) => ({
              range: w,
              hoverMessage:
                v.type === "user"
                  ? `lifetime of variable \`${v.name}\``
                  : undefined,
            }))
            .flat(),
        )
        .flat();
      lifetime = lifetime.concat(selectedLiveDecos);

      const outliveList = selectedDecls
        .map((v) =>
          v.must_live_at
            .map((w) =>
              // if there are drop range,
              // it's type may be implemented `Drop`
              v.drop
                ? excludeRanges(w, v.drop_range)
                : excludeRanges(w, v.lives),
            )
            .flat()
            .map((w) => ({
              ...v,
              outLives: w,
            }))
            .map((v) => ({
              range: v.outLives,
              hoverMessage:
                v.type === "user"
                  ? `variable \`${v.name}\` outlives it's lifetime`
                  : "out of it's lifetime",
            })),
        )
        .flat();
      outLives = outLives.concat(outliveList.map((v) => ({ range: v.range })));
      messages = messages.concat(outliveList);

      // start generating decorations for basic blocks
      for (const bb of mir.basic_blocks) {
        // start generating decorations for statements
        for (const stmt of bb.statements) {
          if (stmt.type === "assign") {
            if (
              stmt.rval &&
              (locals.includes(stmt.target_local_index) ||
                locals.includes(stmt.rval.target_local_index))
            ) {
              if (stmt.rval.type === "move") {
                const movedFrom = getDeclFromLocal(
                  stmt.rval.target_local_index,
                );
                const movedTo = getDeclFromLocal(stmt.target_local_index);
                moves.push({
                  range: stmt.rval.range,
                  hoverMessage:
                    "ownership moved" +
                    (movedFrom?.type === "user"
                      ? ` from \`${movedFrom.name}\``
                      : "") +
                    (movedTo?.type === "user" ? ` to \`${movedTo.name}\`` : ""),
                });
              } else if (stmt.rval.type === "borrow") {
                const borrowFrom = getDeclFromLocal(
                  stmt.rval.target_local_index,
                );
                if (stmt.rval.mutable) {
                  mBorrows.push({
                    range: stmt.rval.range,
                    hoverMessage:
                      "mutable borrow" +
                      (borrowFrom?.type === "user"
                        ? ` of \`${borrowFrom.name}\``
                        : ""),
                  });
                } else {
                  imBorrows.push({
                    range: stmt.rval.range,
                    hoverMessage:
                      "immutable borrow" +
                      (borrowFrom?.type === "user"
                        ? ` of \`${borrowFrom.name}\``
                        : ""),
                  });
                }
              }
            }
          }
          // start terminator
          if (bb.terminator) {
            if (
              bb.terminator.type === "call" &&
              locals.includes(bb.terminator.destination_local_index)
            ) {
              const dest = getDeclFromLocal(
                bb.terminator.destination_local_index,
              );
              moves.push({
                range: bb.terminator.fn_span,
                hoverMessage:
                  "value from function call" +
                  (dest?.type === "user" ? ` to \`${dest.name}\`` : ""),
              });
            }
          }
          // end terminator
        }
        // end statements
      }
      // end basic blocks
    }
    messages = messages.concat(lifetime);
    lifetime = lifetime
      .map((v) =>
        excludeRanges(
          v.range,
          outLives
            .map((w) => w.range)
            .concat(
              mBorrows.map((w) => w.range),
              imBorrows.map((w) => w.range),
              moves.map((w) => w.range),
            ),
        ).map((w) => ({ range: w })),
      )
      .flat();
    messages = messages.concat(moves);
    moves = moves
      .map((v) =>
        excludeRanges(
          v.range,
          outLives
            .map((w) => w.range)
            .concat(
              mBorrows.map((w) => w.range),
              imBorrows.map((w) => w.range),
            ),
        ).map((w) => ({ range: w })),
      )
      .flat();
    messages = messages.concat(imBorrows);
    imBorrows = imBorrows
      .map((v) =>
        excludeRanges(
          v.range,
          outLives.map((w) => w.range).concat(mBorrows.map((w) => w.range)),
        ).map((w) => ({ range: w })),
      )
      .flat();
    messages = messages.concat(mBorrows);
    mBorrows = mBorrows
      .map((v) =>
        excludeRanges(
          v.range,
          outLives.map((w) => w.range),
        ).map((w) => ({ range: w })),
      )
      .flat();

    const decoInfoMap = (info: DecoInfo[]): vscode.DecorationOptions[] => {
      return info.map((v) => ({
        range: rangeToRange(editor.document, v.range),
        hoverMessage: v.hoverMessage,
      }));
    };
    editor.setDecorations(lifetimeDecorationType, decoInfoMap(lifetime));
    editor.setDecorations(moveDecorationType, decoInfoMap(moves));
    editor.setDecorations(imBorrowDecorationType, decoInfoMap(imBorrows));
    editor.setDecorations(mBorrowDecorationType, decoInfoMap(mBorrows));
    editor.setDecorations(outLiveDecorationType, decoInfoMap(outLives));
    editor.setDecorations(emptyDecorationType, decoInfoMap(messages));
  };
  const resetDecoration = (editor: vscode.TextEditor) => {
    editor.setDecorations(lifetimeDecorationType, []);
    editor.setDecorations(moveDecorationType, []);
    editor.setDecorations(imBorrowDecorationType, []);
    editor.setDecorations(mBorrowDecorationType, []);
    editor.setDecorations(outLiveDecorationType, []);
  };

  const startAnalyze = async () => {
    console.log("start analyzing...");
    try {
      const wsPath = vscode.workspace.workspaceFolders![0].uri.path;
      const owlProcess = spawn("cargo", ["owl"], {
        stdio: ["ignore", "pipe", "pipe"],
        cwd: wsPath,
      });
      owlProcess.stderr.on("data", (c: Buffer) => {
        for (const o of c.toString().trimEnd().split("\n")) {
          console.log("rustowl stderr: ", o);
        }
      });
      let stdout = "";
      owlProcess.stdout.on("data", (c: Buffer) => {
        stdout += c.toString();
      });
      owlProcess.on("close", (code) => {
        console.log(`cargo owl exited with status code ${code}`);
        if (code === 0) {
          const data = zWorkspace.safeParse(JSON.parse(stdout));
          if (data.success) {
            analyzed = data.data;
            console.log(analyzed);
            return;
          } else {
            console.log(data.error);
          }
        }
        vscode.window.showErrorMessage("analyzer error");
      });
    } catch (err) {
      vscode.window.showErrorMessage(`Analyzer returns internal error: ${err}`);
      return;
    }
  };

  // events
  let activeEditor: vscode.TextEditor | undefined =
    vscode.window.activeTextEditor;
  vscode.window.onDidChangeActiveTextEditor(
    (editor) => {
      activeEditor = editor;
    },
    null,
    context.subscriptions,
  );
  let timeout: NodeJS.Timeout | undefined = undefined;
  vscode.workspace.onDidSaveTextDocument(
    (ev) => {
      analyzed = undefined;
      if (timeout) {
        clearTimeout(timeout);
      }
      if (
        //ev.document === activeEditor?.document &&
        activeEditor?.document.languageId === "rust"
      ) {
        if (activeEditor) {
          resetDecoration(activeEditor);
        }
        timeout = setTimeout(async () => {
          if (activeEditor) {
            //if (serverProcess !== "installing") {
            await startAnalyze();
            updateDecoration(activeEditor);
            //}
          }
        }, 1000);
      }
    },
    null,
    context.subscriptions,
  );
  vscode.window.onDidChangeTextEditorSelection(
    (ev) => {
      if (ev.textEditor === activeEditor) {
        updateDecoration(activeEditor);
      }
    },
    null,
    context.subscriptions,
  );

  /*
  if (dockerStatus === "not started") {
    startServer();
  }
  */
  /*
  if (serverProcess === undefined) {
    runServer();
  }
  */
}

export function deactivate() {
  //dockerStop();
  /*
  if (serverProcess instanceof ChildProcess) {
    serverProcess.kill();
  }
    */
}
