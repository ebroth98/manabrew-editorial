#!/usr/bin/env node
//
// Build the forge-wasm crate for use in the web frontend.
//
// This script:
// 1. Builds forge-wasm using wasm-pack
// 2. Copies the output to src/wasm for Vite to consume
//
// Prerequisites:
// - Rust toolchain with wasm32-unknown-unknown target
// - wasm-pack: cargo install wasm-pack

import { spawnSync } from "child_process";
import { existsSync, rmSync } from "fs";
import { fileURLToPath } from "url";
import { join, dirname } from "path";
import { homedir, platform } from "os";

const scriptsDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(scriptsDir, "..");
const outputDir = join(projectRoot, "src", "wasm");
const isWindows = platform() === "win32";
const exe = isWindows ? ".exe" : "";

function commandExists(cmd) {
  const probe = isWindows ? "where" : "which";
  const result = spawnSync(probe, [cmd], { stdio: "ignore" });
  return result.status === 0;
}

function resolveWasmPack() {
  if (commandExists("wasm-pack")) {
    return "wasm-pack";
  }
  const cargoBin = join(homedir(), ".cargo", "bin", `wasm-pack${exe}`);
  if (existsSync(cargoBin)) {
    return cargoBin;
  }
  return null;
}

function run(cmd, args, opts = {}) {
  const result = spawnSync(cmd, args, {
    stdio: "inherit",
    cwd: projectRoot,
    shell: isWindows,
    ...opts,
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

console.log("Building forge-wasm...");

let wasmPack = resolveWasmPack();
if (!wasmPack) {
  console.log("wasm-pack not found. Installing...");
  run("cargo", ["install", "wasm-pack"]);
  wasmPack = resolveWasmPack();
  if (!wasmPack) {
    console.error("wasm-pack install succeeded but binary still not found.");
    process.exit(1);
  }
}

run(wasmPack, [
  "build",
  "--target",
  "web",
  "--out-dir",
  outputDir,
  "--out-name",
  "forge_wasm",
  "forge-engine/crates/forge-wasm",
]);

for (const file of [".gitignore", "package.json", "README.md"]) {
  const path = join(outputDir, file);
  if (existsSync(path)) {
    rmSync(path, { force: true });
  }
}

console.log("\nBundling card data...");
// shell: false because process.execPath may contain spaces (e.g.
// "C:\Program Files\nodejs\node.exe") which cmd.exe would mis-parse when
// shell is true. Without shell, spawnSync passes the path as a single
// argument to CreateProcess.
run(process.execPath, [join(scriptsDir, "bundle-cards.mjs")], { shell: false });

console.log("\nBuild complete!");
console.log(`WASM output: ${outputDir}`);
console.log(`Card data:   ${join(projectRoot, "public", "wasm")}`);
