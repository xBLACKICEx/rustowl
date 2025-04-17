import fs from "node:fs/promises";
import { spawnSync } from "node:child_process";
import * as vscode from "vscode";
const version = require("../package.json").version as string;

export const hostTuple = (): string | null => {
  const platform = process.platform;
  const arch = process.arch;
  if (platform === "linux") {
    if (arch === "arm64") {
      return "aarch64-unknown-linux-gnu";
    } else if (arch === "x64") {
      return "x86_64-unknown-linux-gnu";
    }
  } else if (platform === "darwin") {
    if (arch === "arm64") {
      return "aarch64-apple-darwin";
    }
  } else if (platform === "win32") {
    if (arch === "x64") {
      return "x86_64-pc-windows-msvc";
    }
  }
  return null;
};

const exeExt = hostTuple()?.includes("windows") ? ".exe" : "";

export const downloadRustowl = async (basePath: string, version: string) => {
  const baseUrl = `https://github.com/cordx56/rustowl/releases/download/v${version}`;
  const host = hostTuple();
  if (host) {
    const owl = await fetch(`${baseUrl}/rustowl-${host}`);
    if (owl.status !== 200) {
      throw Error("RustOwl download error");
    }
    await fs.writeFile(
      `${basePath}/rustowl${exeExt}`,
      Buffer.from(await owl.arrayBuffer()),
      { flag: "w" },
    );
    const owlc = await fetch(`${baseUrl}/rustowlc-${host}`);
    if (owlc.status !== 200) {
      throw Error("RustOwl download error");
    }
    await fs.writeFile(
      `${basePath}/rustowlc${exeExt}`,
      Buffer.from(await owlc.arrayBuffer()),
      { flag: "w" },
    );
  } else {
    throw Error("unsupported host");
  }
};

const exists = (path: string) => {
  return fs
    .access(path)
    .then(() => true)
    .catch(() => false);
};
export const bootstrapRustowl = async (basePath: string) => {
  if (spawnSync("rustowl", ["--version"]).status !== null) {
    return;
  }
  if (
    (await exists(`${basePath}/rustowl${exeExt}`)) &&
    (await exists(`${basePath}/rustowlc${exeExt}`))
  ) {
    return;
  }
  await fs.mkdir(basePath, { recursive: true });

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "RustOwl installation",
      cancellable: false,
    },
    async () => {
      try {
        await downloadRustowl(basePath, version);
      } catch (e) {
        vscode.window.showErrorMessage(`${e}`);
      }
    },
  );
};
