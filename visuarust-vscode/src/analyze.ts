import * as vscode from "vscode";
import { zIndex, type zInfer, zMir, zRange, zMirDecl } from "./api/schemas";
import { eliminatedRanges } from "./range";

type Mir = zInfer<typeof zMir>;
type Local = zInfer<typeof zIndex>;
type Range = zInfer<typeof zRange>;

export type Analyzed = {};

export const analyzeMir = (mir: Mir) => {};

export const selectLocal = (pos: number, mir: Mir): Local[] => {
  const selected: Local[] = [];
  const select = (local: Local, range: Range) => {
    if (pos < range.from || range.until < pos) {
      return undefined;
    }
    console.log("selected ", pos, " @ ", range);
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

type DeclLifetimes = Record<zInfer<typeof zIndex>, Range[]>;
export const calculateDeclsLifetimes = (
  decls: zInfer<typeof zMirDecl>[]
): DeclLifetimes => {
  const res: DeclLifetimes = {};
  for (const decl of decls) {
    res[decl.local_index] = decl.lives ? eliminatedRanges(decl.lives) : [];
  }
  return res;
};
