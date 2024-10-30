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

  const lifetimeDecoration = vscode.window.createTextEditorDecorationType({
    light: {
      textDecoration: "underline 4px darkblue",
    },
    dark: {
      textDecoration: "underline 4px lightblue",
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
    const decorations: vscode.DecorationOptions[] = [];
    for (const item of analyzed.items) {
      if (item.type === "function") {
        const mir = item.mir;
        const local = selectLocal(cursor, mir);
        console.log(
          "selected local is",
          local,
          "@",
          mir.decls.map((v) => v.local_index)
        );
        const decl = mir.decls.filter((v) => v.local_index === local).at(0);
        console.log(decl);
        if (decl?.lives) {
          for (const live of decl.lives) {
            decorations.push({
              range: rangeToRange(editor.document, live),
              hoverMessage: "lifetime",
            });
          }
          editor.setDecorations(lifetimeDecoration, decorations);
        }
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
