import { z, ZodSchema } from "zod";

export const zAliveMessage = z.object({ status: z.literal(true) });

export const zIndex = z.number().int();
export const zLoc = z.number().int();
export const zRange = z.object({ from: zLoc, until: zLoc });
export const zMirDecl = z.union([
  z.object({
    type: z.literal("user"),
    local_index: zIndex,
    name: z.string(),
    span: zRange,
    ty: z.string(),
    lives: zRange.array().nullish(),
    must_live_at: zRange.array(),
  }),
  z.object({
    type: z.literal("other"),
    local_index: zIndex,
    ty: z.string(),
    lives: zRange.array().nullish(),
    must_live_at: zRange.array(),
  }),
]);
export const zMirRval = z.union([
  z.object({
    type: z.literal("move"),
    target_local_index: zIndex,
    range: zRange,
  }),
  z.object({
    type: z.literal("borrow"),
    target_local_index: zIndex,
    range: zRange,
    mutable: z.boolean(),
    outlive: zRange.nullish(),
  }),
]);
export const zMirStatement = z.union([
  z.object({
    type: z.literal("storage_live"),
    target_local_index: zIndex,
    range: zRange,
  }),
  z.object({
    type: z.literal("storage_dead"),
    target_local_index: zIndex,
    range: zRange,
  }),
  z.object({
    type: z.literal("assign"),
    target_local_index: zIndex,
    range: zRange,
    rval: zMirRval.nullish(),
  }),
]);
export const zMirTerminator = z.union([
  z.object({ type: z.literal("drop"), local_index: zIndex, range: zRange }),
  z.object({
    type: z.literal("call"),
    destination_local_index: zIndex,
    fn_span: zRange,
  }),
  z.object({ type: z.literal("other") }),
]);
export const zMirBasicBlock = z.object({
  statements: z.array(zMirStatement),
  terminator: zMirTerminator.nullish(),
});
export const zMir = z.object({
  basic_blocks: z.array(zMirBasicBlock),
  decls: z.array(zMirDecl),
});
export const zItem = z.object({
  type: z.literal("function"),
  span: zRange,
  mir: zMir,
});
export const zCollectedData = z.object({ items: z.array(zItem) });
export const zAnalyzeSuccess = z.object({
  success: z.literal(true),
  compile_error: z.boolean(),
  collected: zCollectedData,
});
export const zAnalyzeResponse = z.union([
  zAnalyzeSuccess,
  z.object({ success: z.literal(false), cause: z.string() }),
]);

export type zInfer<T extends ZodSchema> = z.infer<T>;
