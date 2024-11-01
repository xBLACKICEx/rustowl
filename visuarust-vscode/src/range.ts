import * as vscode from "vscode";
import { zIndex, type zInfer, zMir, zRange } from "./api/schemas";

type Range = zInfer<typeof zRange>;

export const rangeToRange = (doc: vscode.TextDocument, range: Range) =>
  new vscode.Range(doc.positionAt(range.from), doc.positionAt(range.until));

export const commonRange = (r1: Range, r2: Range): Range | null => {
  if (r2.from < r1.from) {
    return commonRange(r2, r1);
  }
  if (r1.until < r2.from) {
    return null;
  }
  const from = r2.from;
  const until = Math.min(r1.until, r2.until);
  return { from, until };
};

export const mergeRange = (r1: Range, r2: Range): Range | null => {
  const common = commonRange(r1, r2);
  if (common) {
    const from = Math.min(r1.from, r2.from);
    const until = Math.max(r1.until, r2.until);
    return { from, until };
  } else {
    return null;
  }
};

export const eliminatedRanges = (ranges: Range[]): Range[] => {
  const res = [...ranges];
  for (let i = 0; i < res.length; i++) {
    for (let j = i + 1; j < res.length; j++) {
      const eliminated = mergeRange(res[i], res[j]);
      if (eliminated) {
        res[i] = eliminated;
        res.splice(j, 1);
        i = -1;
        break;
      }
    }
  }
  return res;
};

export const excludeRange = (from: Range, ex: Range): Range[] => {
  const common = commonRange(from, ex);
  if (common) {
    const r1 = { from: from.from, until: common.from - 1 };
    const r2 = { from: common.until + 1, until: from.until };
    return eliminatedRanges(
      (r1.from <= r1.until ? [r1] : []).concat(r2.from <= r2.until ? [r2] : [])
    );
  } else {
    return [from];
  }
};
