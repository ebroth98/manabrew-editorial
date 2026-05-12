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

// Post-process with wasm-opt if Binaryen is on PATH. `wasm-pack`'s bundled
// `wasm-opt` is disabled in `forge-wasm/Cargo.toml` (old Binaryen crashed
// on our output); a modern Binaryen installed via brew / a release binary
// handles it fine and produces a smaller, Firefox-compilable wasm. If
// `wasm-opt` isn't installed we ship the unoptimized wasm — Chrome compiles
// it, Firefox doesn't (see the comments on `forge-wasm` in Cargo.toml).
const wasmFile = join(outputDir, "forge_wasm_bg.wasm");
if (commandExists("wasm-opt")) {
  console.log("\nOptimizing with wasm-opt...");
  const tmpOut = `${wasmFile}.opt`;
  run("wasm-opt", ["-O3", "--enable-bulk-memory", wasmFile, "-o", tmpOut]);
  rmSync(wasmFile);
  run("mv", [tmpOut, wasmFile]);
} else {
  console.log(
    "\n[build-wasm] `wasm-opt` not on PATH; shipping unoptimized wasm.",
  );
  console.log(
    "[build-wasm] Install Binaryen (`brew install binaryen`) to fix Firefox compat",
  );
  console.log(
    "[build-wasm] and shrink the .wasm by ~30–50%.",
  );
}

console.log("\nBuilding card archive...");
run(
  "cargo",
  [
    "run",
    "--release",
    "-p",
    "forge-cardset-archive",
    "--bin",
    "build-cardset-archive",
    "--features",
    "build",
    "--",
    "forge/forge-gui/res/cardsfolder",
    "forge/forge-gui/res/tokenscripts",
    "forge/forge-gui/res/editions",
    "forge/forge-gui/res/blockdata",
    "public/wasm/cardset.v4.rkyv",
  ],
  { shell: false },
);

// Preset deck JSONs live at `public/preset_decks/*.json` and ship as-is —
// no bundling step. Both the web worker (HTTP fetch) and the Tauri shell
// (bundled resource) read the same per-deck files directly.

console.log("\nBuild complete!");
console.log(`WASM output: ${outputDir}`);
console.log(`Card data:   ${join(projectRoot, "public", "wasm")}`);
