#!/usr/bin/env node
import { readFileSync } from "fs";
import { execSync } from "child_process";
import { fileURLToPath } from "url";
import { dirname, join } from "path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const registryPath = join(root, "forge-engine/crates/forge-parity/regression.json");
const javaJar = join(root, "forge/forge-harness/target/forge-harness-jar-with-dependencies.jar");

// Ensure the Java harness JAR is up-to-date before running any parity test
execSync("bash scripts/check-harness.sh", { stdio: "inherit", cwd: root });

const name = process.argv[2];
if (!name) {
  const registry = JSON.parse(readFileSync(registryPath, "utf-8"));
  console.error("Usage: npm run parity <test-name>\n");
  console.error("Available tests:");
  for (const entry of registry) {
    console.error(`  - ${entry.name}`);
  }
  process.exit(1);
}

const registry = JSON.parse(readFileSync(registryPath, "utf-8"));
const entry = registry.find((e) => e.name === name);
if (!entry) {
  console.error(`Unknown test: "${name}". Available: ${registry.map((e) => e.name).join(", ")}`);
  process.exit(1);
}

const cmd = `cargo run -p forge-parity --bin forge-parity -- --java-jar "${javaJar}" ${entry.args}`;
console.log(`> ${cmd}\n`);
execSync(cmd, { stdio: "inherit", cwd: root });
