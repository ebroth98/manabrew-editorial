#!/usr/bin/env node
import { createHash } from "crypto";
import { spawnSync } from "child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "fs";
import { fileURLToPath } from "url";
import { join, relative } from "path";

const scriptsDir = fileURLToPath(new URL(".", import.meta.url));
const root = join(scriptsDir, "..");
const forgeRoot = join(root, "forge");
const jarPath = join(forgeRoot, "forge-harness", "target", "forge-harness-jar-with-dependencies.jar");
const checksumPath = join(forgeRoot, "forge-harness", "target", ".harness-sources-checksum");

const sourceDirs = [
  join(forgeRoot, "forge-core", "src"),
  join(forgeRoot, "forge-game", "src"),
  join(forgeRoot, "forge-ai", "src"),
  join(forgeRoot, "forge-gui", "src"),
  join(forgeRoot, "forge-harness", "src"),
];

const pomFiles = [
  join(forgeRoot, "pom.xml"),
  join(forgeRoot, "forge-core", "pom.xml"),
  join(forgeRoot, "forge-game", "pom.xml"),
  join(forgeRoot, "forge-ai", "pom.xml"),
  join(forgeRoot, "forge-gui", "pom.xml"),
  join(forgeRoot, "forge-harness", "pom.xml"),
];

function walkFiles(dir, predicate, acc = []) {
  if (!existsSync(dir)) {
    return acc;
  }

  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      walkFiles(fullPath, predicate, acc);
    } else if (predicate(fullPath)) {
      acc.push(fullPath);
    }
  }

  return acc;
}

function sha256Buffer(buffer) {
  return createHash("sha256").update(buffer).digest("hex");
}

function computeChecksum() {
  const javaFiles = sourceDirs
    .flatMap((dir) => walkFiles(dir, (filePath) => filePath.endsWith(".java")))
    .sort();

  const hashedEntries = [
    ...javaFiles.map((filePath) => `${relative(root, filePath)}:${sha256Buffer(readFileSync(filePath))}`),
    ...pomFiles
      .filter((filePath) => existsSync(filePath))
      .sort()
      .map((filePath) => `${relative(root, filePath)}:${sha256Buffer(readFileSync(filePath))}`),
  ];

  return sha256Buffer(Buffer.from(hashedEntries.join("\n"), "utf8"));
}

function updateChecksum() {
  mkdirSync(join(forgeRoot, "forge-harness", "target"), { recursive: true });
  writeFileSync(checksumPath, `${computeChecksum()}\n`);
  console.log("harness: checksum updated");
}

function isStale() {
  if (!existsSync(jarPath)) {
    console.log(`harness: JAR not found at ${jarPath}`);
    return true;
  }

  if (!existsSync(checksumPath)) {
    console.log("harness: no stored checksum, assuming stale");
    return true;
  }

  const storedChecksum = readFileSync(checksumPath, "utf8").trim();
  const currentChecksum = computeChecksum();
  if (storedChecksum !== currentChecksum) {
    console.log("harness: sources changed (checksum mismatch)");
    return true;
  }

  return false;
}

function canRun(command, args) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "ignore",
    shell: process.platform === "win32" && command.toLowerCase().endsWith(".cmd"),
  });

  return !result.error && result.status === 0;
}

function resolveMaven() {
  const mvnwCmd = join(forgeRoot, "mvnw.cmd");
  const mvnw = join(forgeRoot, "mvnw");

  if (process.platform === "win32" && existsSync(mvnwCmd)) {
    return mvnwCmd;
  }

  if (process.platform !== "win32" && existsSync(mvnw)) {
    return mvnw;
  }

  if (process.platform === "win32" && canRun("mvn.cmd", ["-version"])) {
    return "mvn.cmd";
  }

  if (canRun("mvn", ["-version"])) {
    return "mvn";
  }

  return null;
}

function assertPrereqs() {
  const missing = [];

  if (!canRun("java", ["-version"])) {
    missing.push("Java 18+");
  }

  if (!resolveMaven()) {
    missing.push("Maven");
  }

  if (missing.length === 0) {
    return;
  }

  console.error(`harness: missing prerequisites: ${missing.join(", ")}`);
  if (process.platform === "win32") {
    console.error("Windows setup:");
    console.error("  1. Install a JDK and verify with: java -version");
    console.error("  2. Install Maven and verify with: mvn -version");
    console.error("  3. Restart PowerShell so PATH changes are picked up");
  } else {
    console.error("Install Java 18+ and Maven, then verify with:");
    console.error("  java -version");
    console.error("  mvn -version");
  }
  process.exit(1);
}

function rebuild() {
  assertPrereqs();
  const maven = resolveMaven();

  console.log("harness: rebuilding JAR...");
  const result = spawnSync(
    maven,
    ["-pl", "forge-harness", "-am", "package", "-DskipTests"],
    {
      cwd: forgeRoot,
      stdio: "inherit",
      shell: process.platform === "win32" && maven.toLowerCase().endsWith(".cmd"),
    },
  );

  if (result.error) {
    console.error(`harness: failed to launch Maven: ${result.error.message}`);
    process.exit(1);
  }

  if (result.status !== 0) {
    console.error(`harness: rebuild FAILED (exit code ${result.status ?? 1})`);
    process.exit(result.status ?? 1);
  }

  updateChecksum();
  console.log("harness: rebuild complete");
}

const mode = process.argv[2] ?? "ensure";

switch (mode) {
  case "build":
    rebuild();
    break;
  case "ensure":
    if (isStale()) {
      rebuild();
    } else {
      console.log("harness: JAR is up-to-date");
    }
    break;
  case "check":
    process.exit(isStale() ? 1 : 0);
    break;
  case "update-checksum":
    updateChecksum();
    break;
  default:
    console.error("Usage: node scripts/harness.mjs <build|ensure|check|update-checksum>");
    process.exit(1);
}
