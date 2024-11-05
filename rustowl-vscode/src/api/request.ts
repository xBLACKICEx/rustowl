import axios from "axios";
import { zAnalyzeResponse, zAliveMessage } from "./schemas";

const BASE_URL = "http://localhost:7819";

export const analyze = async (code: string) => {
  const resp = await axios.post(`${BASE_URL}/analyze`, {
    name: "main.rs",
    code,
  });
  const parsed = zAnalyzeResponse.safeParse(resp.data);
  if (!parsed.success) {
    throw Error("invalid response");
  }
  return parsed.data;
};

export const isAlive = async (): Promise<boolean> => {
  try {
    const resp = await axios.get(`${BASE_URL}/`);
    const parsed = zAliveMessage.parse(resp.data);
    return parsed.status;
  } catch (_e) {
    return false;
  }
};
