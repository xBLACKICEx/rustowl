import {z} from "zod";
import {zCollectedData} from "@/utils/api";

export const analyze = (data: z.infer<typeof zCollectedData>) => {
    for (const item of data.items) {
        item.mir.decls
    }
}
