import * as vscode from "vscode";

import {
  spawn,
  ChildProcessWithoutNullStreams,
  ChildProcess,
} from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";

import { analyze, isAlive } from "./api/request";
import { zCollectedData, zInfer, zRange } from "./api/schemas";
import { selectLocal } from "./analyze";
import { eliminatedRanges, rangeToRange, excludeRanges } from "./range";

const DOCKER_CONTAINER_NAME = "rustowl-server";

const dockerRun = (): ChildProcessWithoutNullStreams => {
  return spawn("docker", [
    "run",
    "-d",
    "--rm",
    "--name",
    DOCKER_CONTAINER_NAME,
    "-p",
    "127.0.0.1:7819:7819",
    "ghcr.io/cordx56/rustowl:latest",
  ]);
};
const dockerStop = (): ChildProcessWithoutNullStreams => {
  return spawn("docker", ["stop", DOCKER_CONTAINER_NAME]);
};

let serverProcess:
  | ChildProcessWithoutNullStreams
  | "running"
  | "installing"
  | undefined = undefined;

export function activate(context: vscode.ExtensionContext) {
  console.log("rustowl activated");

  const runServer = async () => {
    if (await isAlive()) {
      serverProcess = "running";
      return;
    }
    const storage = context.globalStorageUri;
    const storagePath = storage.fsPath;
    if (!storagePath) {
      return;
    }
    serverProcess = "installing";
    const installScriptPath = path.join(storagePath, "install.sh");
    if (!fs.statSync(installScriptPath).isFile()) {
      try {
        const script = await fetch(
          "https://github.com/cordx56/rustowl/releases/latest/download/install.sh"
        );
        fs.writeFileSync(
          installScriptPath,
          Buffer.from(await script.arrayBuffer())
        );
      } catch (_e) {
        vscode.window.showInformationMessage(
          "installing rustowl-server failed"
        );
      }
    }
    vscode.window.showInformationMessage("Installing rustowl-server");
    let installProcess = spawn("./install.sh", {
      cwd: storagePath,
    });
    installProcess.on("exit", (code) => {
      if (code === 0) {
        serverProcess = spawn("./install.sh", ["run"], {
          cwd: storagePath,
        });
        vscode.window.showInformationMessage("rustowl-server started");
      } else {
        vscode.window.showInformationMessage(
          "installing rustowl-server failed"
        );
      }
    });
  };

  // for quick start, automatically starts Docker container
  // 1: not started, 2: starting, 3: started, 4: error
  let dockerStatus: "not started" | "starting" | "started" | "error" =
    "not started";
  const startServer = async () => {
    if (dockerStatus === "not started") {
      if (await isAlive()) {
        dockerStatus = "started";
        return dockerStatus;
      }
      vscode.window.showInformationMessage(
        "starting rustowl-server with Docker..."
      );
      dockerStatus = "starting";
      const run = dockerRun();
      run.on("exit", (code) => {
        if (code !== 0) {
          dockerStatus = "error";
          vscode.window.showErrorMessage(
            `rustowl-server on Docker exited with status code ${code}`
          );
        } else {
          dockerStatus = "started";
          vscode.window.showInformationMessage("rustowl-server started");
        }
      });
    }
    return dockerStatus;
  };

  const lifetimeDecorationType = vscode.window.createTextEditorDecorationType({
    textDecoration: "underline solid 4px hsla(125, 80%, 60%, 0.8)",
  });
  const moveDecorationType = vscode.window.createTextEditorDecorationType({
    textDecoration: "underline solid 4px hsla(35, 80%, 60%, 0.8)",
  });
  const imBorrowDecorationType = vscode.window.createTextEditorDecorationType({
    textDecoration: "underline solid 4px hsla(230, 80%, 60%, 0.8)",
  });
  const mBorrowDecorationType = vscode.window.createTextEditorDecorationType({
    textDecoration: "underline solid 4px hsla(300, 80%, 60%, 0.8)",
  });
  //const emptyDecorationType = vscode.window.createTextEditorDecorationType({});

  let analyzed: zInfer<typeof zCollectedData> | undefined = undefined;

  // update decoration
  const updateDecoration = (editor: vscode.TextEditor) => {
    if (!analyzed) {
      return;
    }
    type DecoInfo = {
      range: zInfer<typeof zRange>;
      hoverMessage?: string;
    };
    let lifetime: DecoInfo[] = [];
    let moves: DecoInfo[] = [];
    let imBorrows: DecoInfo[] = [];
    let mBorrows: DecoInfo[] = [];
    //clearDecoration(editor);
    const cursor = editor.document.offsetAt(editor.selection.active);
    for (const itemId in analyzed.items) {
      const item = analyzed.items[itemId];
      if (item.type === "function") {
        const mir = item.mir;

        const getDeclFromLocal = (local: number) =>
          mir.decls.filter((v) => v.local_index === local).at(0);

        const locals = selectLocal(cursor, mir);
        const userDecls = mir.decls.filter((v) => v.type === "user");
        const selectedDecls = userDecls
          .filter((v) => locals.includes(v.local_index))
          .map((v) => ({
            ...v,
            lives: v.lives ? eliminatedRanges(v.lives) : null,
          }));

        const selectedLives = selectedDecls
          .map(
            (v) =>
              v.lives?.map((w) => ({
                range: w,
                hoverMessage: `lifetime of variable ${v.name}`,
              })) || []
          )
          .flat();
        lifetime = lifetime.concat(selectedLives);

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
                    stmt.rval.target_local_index
                  );
                  const movedTo = getDeclFromLocal(stmt.target_local_index);
                  moves.push({
                    range: stmt.rval.range,
                    hoverMessage:
                      "ownership moved" +
                      (movedFrom?.type === "user"
                        ? ` from \`${movedFrom.name}\``
                        : "") +
                      (movedTo?.type === "user"
                        ? ` to \`${movedTo.name}\``
                        : ""),
                  });
                } else if (stmt.rval.type === "borrow") {
                  const borrowFrom = getDeclFromLocal(
                    stmt.rval.target_local_index
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
                  bb.terminator.destination_local_index
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
    }
    lifetime = lifetime
      .map((v) =>
        excludeRanges(
          v.range,
          mBorrows
            .map((w) => w.range)
            .concat(
              imBorrows.map((w) => w.range),
              moves.map((w) => w.range)
            )
        ).map((w) => ({ ...v, range: w }))
      )
      .flat();
    moves = moves
      .map((v) =>
        excludeRanges(
          v.range,
          mBorrows.map((w) => w.range).concat(imBorrows.map((w) => w.range))
        ).map((w) => ({ ...v, range: w }))
      )
      .flat();
    imBorrows = imBorrows
      .map((v) =>
        excludeRanges(
          v.range,
          mBorrows.map((w) => w.range)
        ).map((w) => ({ ...v, range: w }))
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
  };
  const resetDecoration = (editor: vscode.TextEditor) => {
    editor.setDecorations(lifetimeDecorationType, []);
    editor.setDecorations(moveDecorationType, []);
    editor.setDecorations(imBorrowDecorationType, []);
    editor.setDecorations(mBorrowDecorationType, []);
  };

  const startAnalyze = async (editor: vscode.TextEditor) => {
    console.log("start analyzing...");
    try {
      const collected = await analyze(editor.document.getText());
      console.log(`analyzed: ${collected.success}`);
      if (collected.success) {
        analyzed = collected.collected;
        // initialize and generate decorations
        /*
        editor.setDecorations(
          emptyDecorationType,
          messagesAndRanges(editor.document, analyzed)
        );
        */
        // decoration initialize end
      } else {
        vscode.window.showErrorMessage(
          `Analyzer works but return compile error`
        );
      }
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
    context.subscriptions
  );
  let timeout: NodeJS.Timeout | undefined = undefined;
  vscode.workspace.onDidChangeTextDocument(
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
            if (serverProcess === undefined) {
              await runServer();
            } else if (serverProcess !== "installing") {
              await startAnalyze(activeEditor);
              updateDecoration(activeEditor);
            }
          }
        }, 1000);
      }
    },
    null,
    context.subscriptions
  );
  vscode.window.onDidChangeTextEditorSelection(
    (ev) => {
      if (ev.textEditor === activeEditor) {
        updateDecoration(activeEditor);
      }
    },
    null,
    context.subscriptions
  );
}

export function deactivate() {
  if (serverProcess instanceof ChildProcess) {
    serverProcess.kill();
  }
}
