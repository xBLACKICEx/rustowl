import * as vscode from "vscode";

import { zInfer, zLspCursorResponse, zLspRange } from "./schemas";
import {
  LanguageClient,
  ServerOptions,
  Executable,
  TransportKind,
  LanguageClientOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined = undefined;

let decoTimer: NodeJS.Timeout | null = null;

export function activate(context: vscode.ExtensionContext) {
  console.log("rustowl activated");

  const lspExec: Executable = {
    command: "cargo",
    args: ["owlsp"],
    transport: TransportKind.stdio,
  };
  const serverOptions: ServerOptions = lspExec;
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "rust" }],
  };

  client = new LanguageClient(
    "rustowlsp",
    "RustOwLSP",
    serverOptions,
    clientOptions,
  );
  client.start();

  let lifetimeDecorationType = vscode.window.createTextEditorDecorationType({});
  let moveDecorationType = vscode.window.createTextEditorDecorationType({});
  let imBorrowDecorationType = vscode.window.createTextEditorDecorationType({});
  let mBorrowDecorationType = vscode.window.createTextEditorDecorationType({});
  let outLiveDecorationType = vscode.window.createTextEditorDecorationType({});
  let emptyDecorationType = vscode.window.createTextEditorDecorationType({});

  // update decoration
  const updateDecoration = (
    editor: vscode.TextEditor,
    data: zInfer<typeof zLspCursorResponse>,
  ) => {
    const rangeToRange = (range: zInfer<typeof zLspRange>) => {
      return new vscode.Range(
        new vscode.Position(range.start.line, range.start.character),
        new vscode.Position(range.end.line, range.end.character),
      );
    };

    const { underlineThickness } = vscode.workspace.getConfiguration("rustowl");

    lifetimeDecorationType = vscode.window.createTextEditorDecorationType({
      textDecoration: `underline solid ${underlineThickness}px hsla(125, 80%, 60%, 0.8)`,
    });
    moveDecorationType = vscode.window.createTextEditorDecorationType({
      textDecoration: `underline solid ${underlineThickness}px hsla(35, 80%, 60%, 0.8)`,
    });
    imBorrowDecorationType = vscode.window.createTextEditorDecorationType({
      textDecoration: `underline solid ${underlineThickness}px hsla(230, 80%, 60%, 0.8)`,
    });
    mBorrowDecorationType = vscode.window.createTextEditorDecorationType({
      textDecoration: `underline solid ${underlineThickness}px hsla(300, 80%, 60%, 0.8)`,
    });
    outLiveDecorationType = vscode.window.createTextEditorDecorationType({
      textDecoration: `underline solid ${underlineThickness}px hsla(0, 80%, 60%, 0.8)`,
    });
    emptyDecorationType = vscode.window.createTextEditorDecorationType({});

    const lifetime: vscode.DecorationOptions[] = [];
    const immut: vscode.DecorationOptions[] = [];
    const mut: vscode.DecorationOptions[] = [];
    const moveCall: vscode.DecorationOptions[] = [];
    const outlive: vscode.DecorationOptions[] = [];
    const messages: vscode.DecorationOptions[] = [];
    for (const deco of data.decorations) {
      const range = rangeToRange(deco.range);
      if (deco.is_display) {
        if (deco.type === "lifetime") {
          lifetime.push({
            range,
          });
        } else if (deco.type === "imm_borrow") {
          immut.push({ range });
        } else if (deco.type === "mut_borrow") {
          mut.push({ range });
        } else if (deco.type === "call" || deco.type === "move") {
          moveCall.push({ range });
        } else if (deco.type === "outlive") {
          outlive.push({ range });
        }
      }
      const hoverMessage = deco.hover_text;
      if (hoverMessage) {
        messages.push({ range, hoverMessage });
      }
    }
    editor.setDecorations(lifetimeDecorationType, lifetime);
    editor.setDecorations(imBorrowDecorationType, immut);
    editor.setDecorations(mBorrowDecorationType, mut);
    editor.setDecorations(moveDecorationType, moveCall);
    editor.setDecorations(outLiveDecorationType, outlive);
    editor.setDecorations(emptyDecorationType, messages);
  };
  const resetDecoration = () => {
    lifetimeDecorationType.dispose();
    moveDecorationType.dispose();
    imBorrowDecorationType.dispose();
    mBorrowDecorationType.dispose();
    outLiveDecorationType.dispose();
    emptyDecorationType.dispose();
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
  //let timeout: NodeJS.Timeout | undefined = undefined;
  vscode.workspace.onDidSaveTextDocument(
    (_ev) => {},
    null,
    context.subscriptions,
  );
  vscode.window.onDidChangeTextEditorSelection(
    (ev) => {
      if (ev.textEditor === activeEditor) {
        resetDecoration();
        if (decoTimer) {
          clearTimeout(decoTimer);
          decoTimer = null;
        }
        decoTimer = setTimeout(async () => {
          const select = ev.textEditor.selection.active;
          const uri = ev.textEditor.document.uri.toString();
          const req = client?.sendRequest("rustowl/cursor", {
            position: {
              line: select.line,
              character: select.character,
            },
            document: { uri },
          });
          const resp = await req;
          const data = zLspCursorResponse.safeParse(resp);
          if (data.success) {
            updateDecoration(ev.textEditor, data.data);
          }
        }, 2000);
      }
    },
    null,
    context.subscriptions,
  );
}

export function deactivate() {
  if (client) {
    client.stop();
  }
}
