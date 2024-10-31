import * as vscode from "vscode";
import { zIndex, type zInfer, zMir, zRange } from "./api/schemas";

type Mir = zInfer<typeof zMir>;
type Local = zInfer<typeof zIndex>;
type Range = zInfer<typeof zRange>;

export type Analyzed = {};

export const analyzeMir = (mir: Mir) => {};

export const rangeToRange = (doc: vscode.TextDocument, range: Range) =>
  new vscode.Range(doc.positionAt(range.from), doc.positionAt(range.until));

export const selectLocal = (pos: number, mir: Mir): Local[] => {
  const selected: Local[] = [];
  const select = (local: Local, range: Range) => {
    console.log("select? ", pos, " @ ", range);
    if (pos < range.from || range.until < pos) {
      return undefined;
    }
    selected.push(local);
  };
  console.log("select from position", pos);

  for (const decl of mir.decls) {
    if (decl.type === "user") {
      select(decl.local_index, decl.span);
    }
  }
  for (const bb of mir.basic_blocks) {
    for (const stmt of bb.statements) {
      /*
      if (stmt.type === "storage_live") {
        select(stmt.target_local_index, stmt.range);
      } else */
      if (stmt.type === "assign") {
        select(stmt.target_local_index, stmt.range);
        if (stmt.rval?.type === "move" || stmt.rval?.type === "borrow") {
          select(stmt.rval.target_local_index, stmt.rval.range);
        }
      }
    }
  }
  return selected;
};
