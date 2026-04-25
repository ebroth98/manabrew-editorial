#!/usr/bin/env node
// Runs a filtered regression.json through two forge-parity binaries (branch vs main)
// in rust-only mode and diffs the emitted JSON GameTrace per (entry, seed).
//
// Usage:
//   node scripts/parity-rust-vs-rust.mjs \
//     --branch-bin ./target/release/forge-parity \
//     --main-bin   ./main-bin/forge-parity \
//     --entries    existing-entries.json \
//     --cards-dir  forge/forge-gui/res/cardsfolder \
//     --decks-dir  preset_decks \
//     --out-dir    rust-vs-rust-out
//
// `entries` is a subset of regression.json (same shape): array of {name, args}.
// Each entry's `args` is re-parsed; `--games N --seed S` is expanded into N single-seed
// runs so each invocation produces one GameTrace JSON for byte-level comparison.
//
// Exit codes: 0 = all match, 1 = at least one divergence, 2 = harness error.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";

function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (!a.startsWith("--")) continue;
    const key = a.slice(2);
    const val = argv[i + 1] && !argv[i + 1].startsWith("--") ? argv[++i] : "true";
    out[key] = val;
  }
  return out;
}

function shellSplit(s) {
  // Minimal POSIX-ish tokenizer: handles single/double quotes, no $var expansion.
  const out = [];
  let cur = "";
  let quote = null;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (quote) {
      if (c === quote) { quote = null; continue; }
      cur += c;
    } else if (c === "'" || c === '"') {
      quote = c;
    } else if (/\s/.test(c)) {
      if (cur) { out.push(cur); cur = ""; }
    } else {
      cur += c;
    }
  }
  if (cur) out.push(cur);
  return out;
}

function pullFlag(argv, flag) {
  const idx = argv.indexOf(flag);
  if (idx === -1) return null;
  const val = argv[idx + 1];
  argv.splice(idx, 2);
  return val;
}

// Fields that are wall-clock or otherwise non-deterministic debugging metadata.
// Stripped before hashing so rust-vs-rust comparisons only see game state.
const IGNORED_FIELDS = new Set(["timestamp_ms"]);

function stripIgnored(value) {
  if (Array.isArray(value)) {
    for (const v of value) stripIgnored(v);
  } else if (value && typeof value === "object") {
    for (const k of Object.keys(value)) {
      if (IGNORED_FIELDS.has(k)) delete value[k];
      else stripIgnored(value[k]);
    }
  }
}

function hashTrace(path) {
  const parsed = JSON.parse(readFileSync(path, "utf8"));
  stripIgnored(parsed);
  const h = createHash("sha256");
  h.update(JSON.stringify(parsed));
  return h.digest("hex");
}

function runBin(binPath, args, outJson) {
  const finalArgs = [...args, "--format", "json", "--output", outJson];
  const res = spawnSync(binPath, finalArgs, { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  return { status: res.status, stderr: res.stderr, stdout: res.stdout };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const branchBin = args["branch-bin"];
  const mainBin = args["main-bin"];
  const entriesPath = args["entries"];
  const cardsDir = args["cards-dir"];
  const decksDir = args["decks-dir"];
  const outDir = args["out-dir"] || "rust-vs-rust-out";

  if (!branchBin || !mainBin || !entriesPath) {
    console.error("missing --branch-bin / --main-bin / --entries");
    process.exit(2);
  }
  if (!existsSync(branchBin) || !existsSync(mainBin)) {
    console.error("binary not found");
    process.exit(2);
  }

  mkdirSync(outDir, { recursive: true });
  const entries = JSON.parse(readFileSync(entriesPath, "utf8"));

  const divergences = [];
  let runs = 0;

  for (const entry of entries) {
    const argv = shellSplit(entry.args);

    // Expand --games N into N single-seed runs.
    const seedStr = pullFlag(argv, "--seed");
    const gamesStr = pullFlag(argv, "--games");
    const baseSeed = seedStr ? Number(seedStr) : 42;
    const games = gamesStr ? Number(gamesStr) : 1;

    for (let i = 0; i < games; i++) {
      const seed = baseSeed + i;
      const perRunArgs = [...argv, "--seed", String(seed), "--games", "1"];
      if (cardsDir) perRunArgs.push("--cards-dir", cardsDir);
      if (decksDir) perRunArgs.push("--decks-dir", decksDir);

      const tag = `${entry.name}_seed${seed}`;
      const branchOut = join(outDir, `${tag}.branch.json`);
      const mainOut = join(outDir, `${tag}.main.json`);

      const b = runBin(branchBin, perRunArgs, branchOut);
      const m = runBin(mainBin, perRunArgs, mainOut);
      runs++;

      if (b.status !== 0 || m.status !== 0) {
        divergences.push({
          tag,
          kind: "run_failed",
          branch_exit: b.status,
          main_exit: m.status,
          branch_stderr: b.stderr.slice(-2000),
          main_stderr: m.stderr.slice(-2000),
        });
        continue;
      }

      const bh = hashTrace(branchOut);
      const mh = hashTrace(mainOut);
      if (bh !== mh) {
        divergences.push({ tag, kind: "trace_mismatch", branch_sha: bh, main_sha: mh });
      }
    }
  }

  const summary = { runs, divergences };
  writeFileSync(join(outDir, "summary.json"), JSON.stringify(summary, null, 2));

  if (divergences.length > 0) {
    console.error(`rust-vs-rust: ${divergences.length}/${runs} runs diverged`);
    for (const d of divergences.slice(0, 10)) {
      console.error(`  - ${d.tag}: ${d.kind}`);
    }
    process.exit(1);
  }
  console.log(`rust-vs-rust: all ${runs} runs matched`);
}

main();
