"use client";

import { useState, useEffect } from "react";
import { z } from "zod";
import { SelectionRange } from "@uiw/react-codemirror";
import { analyze, zCollectedData } from "@/utils/api";
import CodeMirror, { Effect } from "@/components/editor/CodeMirror";

export default () => {
  const [code, setCode] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [analyzed, setAnalyzed] = useState<
    z.infer<typeof zCollectedData> | undefined
  >(undefined);
  const [compileError, setCompileError] = useState(false);

  const [timer, setTimer] = useState<NodeJS.Timeout | null>(null);
  const analyzeTimer = (code: string, delay: number) => {
    return setTimeout(async () => {
      const res = await analyze(code);
      if (!res.success) {
        setMessage(res.cause);
        setAnalyzed(undefined);
      } else {
        setMessage(null);
        setCompileError(res.compile_error);
        setAnalyzed(res.collected);
      }
    }, delay);
  };

  const [cursorPos, setCursorPos] = useState<SelectionRange | undefined>(
    undefined,
  );
  const [effects, setEffects] = useState<Effect[]>([]);
  useEffect(() => {
    setEffects([]);
    if (analyzed === undefined || cursorPos === undefined) {
      return;
    }
    let tmpEffects: Effect[] = [];
    for (const item of analyzed.items) {
      for (const bb of item.mir.basic_blocks) {
        let nearest:
          | { index: number; from: number; until: number }
          | undefined = undefined;
        const updateNearest = (range: {
          index: number;
          from: number;
          until: number;
        }) => {
          if (cursorPos.head < range.from || range.until < cursorPos.head) {
            return;
          }
          if (nearest === undefined || nearest.from <= cursorPos.head) {
            nearest = { ...range };
          }
        };
        for (const stmt of bb.statements) {
          if (stmt.type === "assign" && stmt.rval) {
            updateNearest({
              index: stmt.rval.target_local_index,
              ...stmt.rval.range,
            });
          } else {
            updateNearest({ index: stmt.target_local_index, ...stmt.range });
          }
        }
        const actions = bb.statements.filter(
          (v) => v.target_local_index === nearest?.index,
        );
        let liveFrom = undefined;
        let moved = false;
        const decls = item.mir.decls.filter(
          (v) => v.local_index === nearest?.index,
        );
        if (decls.length !== 1) {
          continue;
        }
        const decl = decls[0];
        console.log("selected decl:", decl);
        if (decl.type === "user") {
          console.log("highlight decl:", decl.span);
          tmpEffects.push({
            style: "border-bottom: 4px solid rgba(200, 100, 200, 0.5)",
            from: decl.span.from,
            until: decl.span.until,
          });
          liveFrom = decl.span.until;
          if (decl.lives) {
            for (const live of decl.lives) {
              tmpEffects.push({
                style: "border-bottom: 4px dashed rgba(100, 200, 200, 0.5)",
                from: live.from,
                until: live.until,
              });
            }
          }
        }
        for (const stmt of actions) {
          if (stmt.type === "assign") {
            if (stmt.rval?.type === "move") {
              console.log("highlight moved:", stmt.rval.range);
              if (liveFrom) {
                tmpEffects.push({
                  style: "border-bottom: 4px dotted rgba(0, 200, 0, 0.5)",
                  from: liveFrom,
                  until: stmt.rval.range.from,
                });
              }
              tmpEffects.push({
                style: "border-right: 4px solid rgba(200, 200, 0, 0.5)",
                from: stmt.rval.range.from,
                until: stmt.rval.range.until,
              });
            } else {
              if (stmt.rval) {
                console.log("highlight rval:", stmt.rval.range);
                tmpEffects.push({
                  style: stmt.rval.mutable
                    ? "border-bottom: 5px solid rgba(200, 100, 100, 0.5)"
                    : "border-bottom: 5px solid rgba(100, 200, 100, 0.5)",
                  from: stmt.range.from,
                  until: stmt.range.until,
                });
                if (stmt.rval.outlive) {
                  console.log("highlight outlive:", stmt.rval.range);
                  tmpEffects.push({
                    style: "background-color: rgba(100, 200, 100, 0.5)",
                    from: stmt.rval.outlive.from,
                    until: stmt.rval.outlive.until,
                  });
                }
              }
            }
          }
        }
      }
    }
    setEffects(tmpEffects);
  }, [analyzed, cursorPos, code]);

  const onUpdateCode = (code: string) => {
    setCode(code);
    if (timer) {
      clearTimeout(timer);
    }
    setTimer(analyzeTimer(code, 1000));
  };

  return (
    <div className="flex flex-col container mx-auto h-screen">
      {message ? <p>{message}</p> : null}
      <p>compile error: {String(compileError)}</p>
      <p>
        cursorPos: {cursorPos?.head}-{cursorPos?.anchor}
      </p>
      <div className="flex h-full">
        <div className="h-full flex-grow">
          <CodeMirror
            code={code}
            setCode={onUpdateCode}
            analyzed={analyzed}
            effects={effects}
            onCursorPos={(p) => setCursorPos(p)}
          />
        </div>
        <div className="w-1/3">
          <pre className="whitespace-pre-wrap">
            {JSON.stringify(analyzed, null, 4)}
          </pre>
        </div>
      </div>
    </div>
  );
};
