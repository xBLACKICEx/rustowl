// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from "vscode";

import { analyze } from "./api/request";
import { zCollectedData, zInfer } from "./api/schemas";
import { rangeToRange, selectLocal } from "./analyze";

// This method is called when your extension is activated
// Your extension is activated the very first time the command is executed
export function activate(context: vscode.ExtensionContext) {
  // Use the console to output diagnostic information (console.log) and errors (console.error)
  // This line of code will only be executed once when your extension is activated
  console.log(
    'Congratulations, your extension "visuarust-vscode" is now active!'
  );

  // The command has been defined in the package.json file
  // Now provide the implementation of the command with registerCommand
  // The commandId parameter must match the command field in package.json
  const disposable = vscode.commands.registerCommand(
    "visuarust-vscode.helloWorld",
    () => {
      // The code you place here will be executed every time your command is executed
      // Display a message box to the user
      vscode.window.showInformationMessage(
        "Hello World from visuarust-vscode!"
      );
    }
  );

  context.subscriptions.push(disposable);

  const lifetimeDecorationType = vscode.window.createTextEditorDecorationType({
    light: {
      textDecoration: "underline dotted 4px darkgreen",
    },
    dark: {
      textDecoration: "underline dotted 4px lightgreen",
    },
  });
  const userVarDeclDecorationType =
    vscode.window.createTextEditorDecorationType({
      light: {
        backgroundColor: "lightred",
      },
      dark: {
        backgroundColor: "darkred",
      },
    });
  const mutBorrowDecorationType = vscode.window.createTextEditorDecorationType({
    light: { backgroundColor: "lightblue" },
    dark: { backgroundColor: "darkblue" },
  });
  const imBorrowDecorationType = vscode.window.createTextEditorDecorationType({
    light: { backgroundColor: "lightgreen" },
    dark: { backgroundColor: "darkgreen" },
  });
  const moveDecorationType = vscode.window.createTextEditorDecorationType({
    light: {
      backgroundColor: "orange",
    },
    dark: {
      backgroundColor: "darkorange",
    },
  });

  let analyzed: zInfer<typeof zCollectedData> | undefined = undefined;
  const startAnalyze = async (editor: vscode.TextEditor) => {
    console.log("start analyzing...");
    try {
      const collected = await analyze(editor.document.getText());
      console.log(`analyzed: ${collected.success}`);
      if (collected.success) {
        analyzed = collected.collected;
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
  const updateDecorations = (editor: vscode.TextEditor) => {
    if (!analyzed) {
      return;
    }
    console.log("update deco");
    const cursor = editor.document.offsetAt(editor.selection.active);
    const userVarDeclDecorations: vscode.DecorationOptions[] = [];
    const lifetimeDecorations: vscode.DecorationOptions[] = [];
    const mutBorrowDecorations: vscode.DecorationOptions[] = [];
    const imBorrowDecorations: vscode.DecorationOptions[] = [];
    const moveDecorations: vscode.DecorationOptions[] = [];
    for (const item of analyzed.items) {
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
        const decls = mir.decls.filter((v) => locals.includes(v.local_index));
        console.log("target decls:", decls);
        // check all declaration
        for (const decl of decls) {
          if (decl.type === "user") {
            userVarDeclDecorations.push({
              range: rangeToRange(editor.document, decl.span),
              hoverMessage: `declaration of \`${decl.name}\``,
            });
          }
          for (const live of decl.lives || []) {
            lifetimeDecorations.push({
              range: rangeToRange(editor.document, live),
              hoverMessage:
                decl.type === "user"
                  ? `lifetime of \`${decl.name}\``
                  : "lifetime",
            });
          }
        }
        // check all basic blocks
        for (const bb of mir.basic_blocks) {
          for (const stmt of bb.statements) {
            if (stmt.type === "assign") {
              if (stmt.rval && locals.includes(stmt.rval.target_local_index)) {
                if (stmt.rval.type === "borrow") {
                  const borrowFrom = declFromLocal(
                    stmt.rval.target_local_index
                  );
                  if (stmt.rval.mutable) {
                    mutBorrowDecorations.push({
                      range: rangeToRange(editor.document, stmt.rval.range),
                      hoverMessage:
                        "mutable borrow" +
                        (borrowFrom?.type === "user"
                          ? ` of \`${borrowFrom.name}\``
                          : ""),
                    });
                  } else {
                    imBorrowDecorations.push({
                      range: rangeToRange(editor.document, stmt.rval.range),
                      hoverMessage:
                        "immutable borrow" +
                        (borrowFrom?.type === "user"
                          ? ` of \`${borrowFrom.name}\``
                          : ""),
                    });
                  }
                } else if (stmt.rval.type === "move") {
                  const movedFrom = declFromLocal(stmt.rval.target_local_index);
                  const movedTo = declFromLocal(stmt.target_local_index);
                  moveDecorations.push({
                    range: rangeToRange(editor.document, stmt.rval.range),
                    hoverMessage:
                      "ownership moved" +
                      (movedFrom?.type === "user"
                        ? ` from \`${movedFrom.name}\``
                        : "") +
                      (movedTo?.type === "user"
                        ? ` to \`${movedTo.name}\``
                        : ""),
                  });
                }
              }
            }
          }
        }
        editor.setDecorations(lifetimeDecorationType, lifetimeDecorations);
        editor.setDecorations(
          userVarDeclDecorationType,
          userVarDeclDecorations
        );
        editor.setDecorations(mutBorrowDecorationType, mutBorrowDecorations);
        editor.setDecorations(imBorrowDecorationType, imBorrowDecorations);
        editor.setDecorations(moveDecorationType, moveDecorations);
      }
    }
  };

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
      if (ev.document === activeEditor?.document) {
        analyzed = undefined;
        if (timeout) {
          clearTimeout(timeout);
        }
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

// This method is called when your extension is deactivated
export function deactivate() {}
