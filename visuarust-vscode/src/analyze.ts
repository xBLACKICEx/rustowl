import * as vscode from "vscode";
import {
  zIndex,
  type zInfer,
  zCollectedData,
  zMir,
  zRange,
  zMirDecl,
} from "./api/schemas";
import { rangeToRange, eliminatedRanges } from "./range";

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

// obtain the list of message and range
export const messagesAndRanges = (
  doc: vscode.TextDocument,
  { items }: zInfer<typeof zCollectedData>
) => {
  const res: vscode.DecorationOptions[] = [];
  const push = (range: Range, hoverMessage: string) => {
    res.push({ range: rangeToRange(doc, range), hoverMessage });
  };
  for (const item of items) {
    const mir = item.mir;
    for (const decl of mir.decls) {
      if (decl.type === "user") {
        push(decl.span, `declaration of \`${decl.name}\``);
        for (const live of decl.lives || []) {
          push(live, `lifetime of \`${decl.name}\``);
        }
      }
    }
    const getDeclFromLocal = (local: number) =>
      mir.decls.filter((v) => v.local_index === local).at(0);

    for (const bb of mir.basic_blocks) {
      for (const stmt of bb.statements) {
        if (stmt.type === "assign") {
          if (stmt.rval) {
            if (stmt.rval.type === "borrow") {
              const borrowFrom = getDeclFromLocal(stmt.rval.target_local_index);
              if (borrowFrom?.type === "user") {
                if (stmt.rval.mutable) {
                  push(
                    stmt.rval.range,
                    "mutable borrow" +
                      (borrowFrom?.type === "user"
                        ? ` of \`${borrowFrom.name}\``
                        : "")
                  );
                } else {
                  push(
                    stmt.rval.range,
                    "immutable borrow" +
                      (borrowFrom?.type === "user"
                        ? ` of \`${borrowFrom.name}\``
                        : "")
                  );
                }
              }
            } else if (stmt.rval.type === "move") {
              const movedFrom = getDeclFromLocal(stmt.rval.target_local_index);
              const movedTo = getDeclFromLocal(stmt.target_local_index);
              push(
                stmt.rval.range,
                "ownership moved" +
                  (movedFrom?.type === "user"
                    ? ` from \`${movedFrom.name}\``
                    : "") +
                  (movedTo?.type === "user" ? ` to \`${movedTo.name}\`` : "")
              );
            }
          }
        }
      }
    }
  }
  return res;
};
