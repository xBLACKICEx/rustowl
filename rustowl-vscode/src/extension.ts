import * as vscode from "vscode";

import { spawn, ChildProcessWithoutNullStreams } from "node:child_process";

import { analyze, isAlive } from "./api/request";
import { zCollectedData, zInfer, zRange } from "./api/schemas";
import {
  selectLocal,
  messagesAndRanges,
  localAssigner,
  traceAssignersRanges,
} from "./analyze";
import {
  decideHue,
  resetColor,
  registerDecorationType,
  applyDecoration,
  resetDecorationType,
  clearDecoration,
} from "./decos";
import { eliminatedRanges, rangeToRange, excludeRange } from "./range";

type Range = zInfer<typeof zRange>;

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

export async function activate(context: vscode.ExtensionContext) {
  console.log("rustowl activated");
  let tryDocker = false;
  const startServer = async () => {
    if (!(await isAlive()) && tryDocker === false) {
      vscode.window.showInformationMessage(
        "starting rustowl-server with Docker..."
      );
      tryDocker = true;
      const run = dockerRun();
      run.on("exit", (code) => {
        if (code !== 0) {
          vscode.window.showErrorMessage(
            `rustowl-server on Docker exited with status code ${code}`
          );
        } else {
          vscode.window.showInformationMessage("rustowl-server started");
        }
      });
    }
  };

  const userVarDeclDecorationType =
    vscode.window.createTextEditorDecorationType({
      light: {
        backgroundColor: "yellow",
      },
      dark: {
        backgroundColor: "yellow",
      },
    });
  const dropDecorationType = vscode.window.createTextEditorDecorationType({
    light: {
      backgroundColor: "red",
    },
    dark: {
      backgroundColor: "crimson",
    },
  });
  const mutBorrowDecorationType = vscode.window.createTextEditorDecorationType({
    light: { backgroundColor: "lightblue" },
    dark: { backgroundColor: "blue" },
  });
  const imBorrowDecorationType = vscode.window.createTextEditorDecorationType({
    light: { backgroundColor: "lightgreen" },
    dark: { backgroundColor: "green" },
  });
  const moveDecorationType = vscode.window.createTextEditorDecorationType({
    light: {
      backgroundColor: "orange",
    },
    dark: {
      backgroundColor: "darkorange",
    },
  });
  const assignerDecorationType = vscode.window.createTextEditorDecorationType({
    light: {
      textDecoration: "underline solid 4px darkgreen",
    },
    dark: {
      textDecoration: "underline solid 4px lightgreen",
    },
  });
  const emptyDecorationType = vscode.window.createTextEditorDecorationType({});

  let analyzed: zInfer<typeof zCollectedData> | undefined = undefined;

  let hues: Record<number, number> = {};
  let itemToLocalToDeco: Record<number, Record<number, number>> = {};
  const updateLifetimeDecoration = (editor: vscode.TextEditor) => {
    if (!analyzed) {
      return;
    }
    //clearDecoration(editor);
    const cursor = editor.document.offsetAt(editor.selection.active);
    for (const itemId in analyzed.items) {
      const item = analyzed.items[itemId];
      if (item.type === "function") {
        const mir = item.mir;
        const locals = selectLocal(cursor, mir);
        console.log(
          "selected locals are",
          locals,
          "in",
          mir.decls.map((v) => v.local_index)
        );
        const userDecls = mir.decls.filter((v) => v.type === "user");
        // get declaration from MIR Local
        const notSelected = userDecls
          .filter((v) => !locals.includes(v.local_index))
          .map((v) => ({ ...v, lives: v.lives || [] }));
        const selected = userDecls
          .filter((v) => locals.includes(v.local_index))
          .map((v) => ({ ...v, lives: v.lives || [] }));
        const selectedLifetime = eliminatedRanges(
          selected.map((v) => v.lives).flat()
        );
        console.log("not selected vars:", notSelected);
        console.log("selected vars:", selected);
        console.log(selectedLifetime);
        for (const i in notSelected) {
          let newLives: Range[] = [];
          for (let j = 0; j < notSelected[i].lives.length; j++) {
            let newRanges = [notSelected[i].lives[j]];
            for (const selectedRange of selectedLifetime) {
              for (let k = 0; k < newRanges.length; k++) {
                newRanges = excludeRange(newRanges[k], selectedRange);
              }
            }
            newLives = [...newLives, ...newRanges];
          }
          notSelected[i].lives = newLives;
        }
        const decls = [...notSelected, ...selected];
        console.log("target decls:", decls);
        // check all declaration
        for (const decl of decls) {
          const tyId = itemToLocalToDeco[itemId][decl.local_index];
          const decoList = [];
          for (const live of decl.lives || []) {
            decoList.push({
              range: rangeToRange(editor.document, live),
            });
          }
          console.log("decolist", decoList);
          applyDecoration(editor, tyId, decoList);
        }
      }
    }
  };

  const updateUserVarDeclDecoration = (editor: vscode.TextEditor) => {
    if (!analyzed) {
      return;
    }
    const cursor = editor.document.offsetAt(editor.selection.active);
    const userVarDeclDecorations: vscode.DecorationOptions[] = [];
    for (const itemId in analyzed.items) {
      const item = analyzed.items[itemId];
      if (item.type === "function") {
        const mir = item.mir;
        const locals = selectLocal(cursor, mir);
        for (const decl of mir.decls.filter((v) =>
          locals.includes(v.local_index)
        )) {
          if (decl.type === "user") {
            userVarDeclDecorations.push({
              range: rangeToRange(editor.document, decl.span),
            });
          }
        }
      }
    }
    editor.setDecorations(userVarDeclDecorationType, userVarDeclDecorations);
  };

  let assignerDecoTypes: Record<number, vscode.TextEditorDecorationType> = {};

  const startAnalyze = async (editor: vscode.TextEditor) => {
    // reset decoration
    resetColor();
    resetDecorationType(editor);
    assignerDecoTypes = {};
    console.log("start analyzing...");
    try {
      const collected = await analyze(editor.document.getText());
      console.log(`analyzed: ${collected.success}`);
      if (collected.success) {
        analyzed = collected.collected;
        // initialize and generate decorations
        for (let itemId in analyzed.items) {
          const item = analyzed.items[itemId];
          if (item.type === "function") {
            // eliminate redundant lives
            for (const declId in item.mir.decls) {
              item.mir.decls[declId].lives = item.mir.decls[declId].lives
                ? eliminatedRanges(item.mir.decls[declId].lives)
                : null;
            }
            itemToLocalToDeco[itemId] = {};
            const locals = item.mir.decls
              //.filter((v) => v.type === "user")
              .map((v) => v.local_index);
            hues = decideHue(locals);
            for (const local of locals) {
              const hue = hues[local];
              itemToLocalToDeco[itemId][local] = registerDecorationType(
                vscode.window.createTextEditorDecorationType({
                  textDecoration: `underline dotted 4px hsla(${hue}, 80%, 60%, 0.7)`,
                })
              );
            }
          }
        }
        editor.setDecorations(
          emptyDecorationType,
          messagesAndRanges(editor.document, analyzed)
        );
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

  // update decorations, on any change (cursor, text...)
  const updateDecorations = (editor: vscode.TextEditor) => {
    if (!analyzed) {
      return;
    }
    console.log("update deco");

    for (const deco of Object.values(assignerDecoTypes)) {
      editor.setDecorations(deco, []);
    }

    updateLifetimeDecoration(editor);
    updateUserVarDeclDecoration(editor);

    const cursor = editor.document.offsetAt(editor.selection.active);
    const mutBorrowDecorations: vscode.DecorationOptions[] = [];
    const imBorrowDecorations: vscode.DecorationOptions[] = [];
    const moveDecorations: vscode.DecorationOptions[] = [];
    const dropDecorations: vscode.DecorationOptions[] = [];
    let assigners: vscode.DecorationOptions[] = [];
    for (const itemId in analyzed.items) {
      const item = analyzed.items[itemId];
      if (item.type === "function") {
        const mir = item.mir;
        const locals = selectLocal(cursor, mir);
        console.log(
          "selected locals are",
          locals,
          "in",
          mir.decls.map((v) => v.local_index)
        );
        // get declaration from MIR Local
        const declFromLocal = (local: number) =>
          mir.decls.filter((v) => v.local_index === local).at(0);

        // check all basic blocks
        for (const bb of mir.basic_blocks) {
          for (const stmt of bb.statements) {
            if (stmt.type === "assign") {
              if (stmt.rval && locals.includes(stmt.rval.target_local_index)) {
                if (stmt.rval.type === "borrow") {
                  const borrowFrom = declFromLocal(
                    stmt.rval.target_local_index
                  );
                  if (borrowFrom?.type === "user") {
                    if (stmt.rval.mutable) {
                      mutBorrowDecorations.push({
                        range: rangeToRange(editor.document, stmt.rval.range),
                      });
                    } else {
                      imBorrowDecorations.push({
                        range: rangeToRange(editor.document, stmt.rval.range),
                      });
                    }
                  }
                } else if (stmt.rval.type === "move") {
                  const movedFrom = declFromLocal(stmt.rval.target_local_index);
                  const movedTo = declFromLocal(stmt.target_local_index);
                  moveDecorations.push({
                    range: rangeToRange(editor.document, stmt.rval.range),
                  });
                }
              }
            }
          }
          if (
            bb.terminator?.type === "drop" &&
            locals.includes(bb.terminator.local_index)
          ) {
            dropDecorations.push({
              range: rangeToRange(editor.document, bb.terminator.range),
            });
          }
        }

        // assigner
        /*
        assigners = assigners.concat(
          locals
            .map((local) => localAssigner(local, mir))
            .map((v) => [
              ...v.terminators
                .filter((w) => w?.type === "call")
                .map((w) => w.fn_span),
              ...v.statements
                .filter((w) => w.type === "assign" && w.rval)
                .map((w) => w.range),
            ])
            .flat()
            .map((v) => ({ range: rangeToRange(editor.document, v) }))
        );
      }
      */

        //editor.setDecorations(lifetimeDecorationType, lifetimeDecorations);
        editor.setDecorations(mutBorrowDecorationType, mutBorrowDecorations);
        editor.setDecorations(imBorrowDecorationType, imBorrowDecorations);
        editor.setDecorations(moveDecorationType, moveDecorations);
        editor.setDecorations(dropDecorationType, dropDecorations);

        // assigner trace
        for (const select of locals) {
          if (!(select in assignerDecoTypes)) {
            assignerDecoTypes[select] =
              vscode.window.createTextEditorDecorationType({
                textDecoration: `underline solid 4px hsla(${hues[select]}, 80%, 60%, 0.7)`,
              });
          }
          const traced = traceAssignersRanges(select, item.mir);
          console.log("traced", traced);
          editor.setDecorations(
            assignerDecoTypes[select],
            traced.map((v) => ({
              range: rangeToRange(editor.document, v),
              hoverMessage: "value traced",
            }))
          );
        }
      }
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
        ev.document === activeEditor?.document &&
        activeEditor.document.fileName.endsWith(".rs")
      ) {
        startServer();
        timeout = setTimeout(async () => {
          if (activeEditor) {
            await startAnalyze(activeEditor);
            updateDecorations(activeEditor);
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
        updateDecorations(activeEditor);
      }
    },
    null,
    context.subscriptions
  );
}

export function deactivate() {
  dockerStop();
}
