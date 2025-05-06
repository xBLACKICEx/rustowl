import fs from "node:fs/promises";
import { spawnSync } from "node:child_process";
import * as vscode from "vscode";
const version = require("../package.json").version as string;

export const hostTuple = (): string | null => {
  let arch = null;
  if (process.arch === "arm64") {
    arch = "aarch64";
  } else if (process.arch === "x64") {
    arch = "x86_64";
  }
  let platform = null;
  if (process.platform === "linux") {
    platform = "unknown-linux-gnu";
  } else if (process.platform === "darwin") {
    platform = "apple-darwin";
  } else if (process.platform === "win32") {
    platform = "pc-windows-msvc";
  }
  if (arch && platform) {
    return `${arch}-${platform}`;
  } else {
    return null;
  }
};

const exeExt = hostTuple()?.includes("windows") ? ".exe" : "";

export const downloadRustowl = async (basePath: string) => {
  const baseUrl = `https://github.com/cordx56/rustowl/releases/download/v${version}`;
  const host = hostTuple();
  if (host) {
    const owl = await fetch(`${baseUrl}/rustowl-${host}${exeExt}`);
    if (owl.status !== 200) {
      throw Error("RustOwl download error");
    }
    await fs.writeFile(
      `${basePath}/rustowl${exeExt}`,
      Buffer.from(await owl.arrayBuffer()),
      { flag: "w" },
    );
    fs.chmod(`${basePath}/rustowl${exeExt}`, "755");
  } else {
    throw Error("unsupported architecture or platform");
  }
};

const exists = async (path: string) => {
  return fs
    .access(path)
    .then(() => true)
    .catch(() => false);
};
const needUpdated = async (currentVersion: string) => {
  console.log(`current RustOwl version: ${currentVersion.trim()}`);
  console.log(`extension version: v${version}`);
  try {
    const semverParser = await import("semver-parser");
    const current = semverParser.parseSemVer(currentVersion.trim(), false);
    const self = semverParser.parseSemVer(version, false);
    if (
      current.major === self.major &&
      current.minor === self.minor &&
      JSON.stringify(current.pre) === JSON.stringify(self.pre)
    ) {
      return false;
    } else {
      console.log("B");
      return true;
    }
  } catch (_e) {
    return true;
  }
};
export const bootstrapRustowl = async (dirPath: string): Promise<string> => {
  if (
    !(await needUpdated(
      spawnSync("rustowl", ["--version", "--quiet"]).stdout.toString(),
    ))
  ) {
    return "rustowl";
  }
  const rustowlPath = `${dirPath}/rustowl${exeExt}`;
  if (
    (await exists(rustowlPath)) &&
    !(await needUpdated(
      spawnSync(rustowlPath, ["--version", "--quiet"]).stdout.toString(),
    ))
  ) {
    return rustowlPath;
  }
  await fs.mkdir(dirPath, { recursive: true });

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "RustOwl installing...",
      cancellable: false,
    },
    async () => {
      try {
        await downloadRustowl(dirPath);
        if (spawnSync(rustowlPath, ["toolchain", "install"]).status !== 0) {
          throw Error("toolchain setup failed");
        }
      } catch (e) {
        vscode.window.showErrorMessage(`${e}`);
      }
    },
  );
  return `${dirPath}/rustowl${exeExt}`;
};
