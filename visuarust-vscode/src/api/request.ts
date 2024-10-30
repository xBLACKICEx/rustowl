import axios from "axios";
import { zAnalyzeResponse } from "./schemas";

export const analyze = async (code: string) => {
  const resp = await axios.post("http://localhost:8000/analyze", {
    name: "main.rs",
    code,
  });
  const parsed = zAnalyzeResponse.safeParse(resp.data);
  if (!parsed.success) {
    throw Error("invalid response");
  }
  return parsed.data;
};
