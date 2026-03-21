#!/usr/bin/env node
// scan_structure.js — Compare Java (forge/game) files against Rust (forge-engine/src) ports.
// Prints a colorful tree and per-module coverage report.

const fs = require('fs');
const path = require('path');

const JAVA_ROOT = path.join(__dirname, 'forge/forge-game/src/main/java/forge/game');
const RUST_ROOT = path.join(__dirname, 'forge-engine/crates/forge-engine/src');

// Parse flags
let filterModule = null;
const modIdx = process.argv.indexOf('--module');
if (modIdx !== -1) {
  filterModule = process.argv[modIdx + 1];
  if (!filterModule) {
    console.error('Usage: node scan_structure.cjs [--module <module_name>] [--symbols]');
    process.exit(1);
  }
}
const showSymbols = process.argv.includes('--symbols');

// ── Scan map: file & symbol overrides ──
const SCAN_MAP_PATH = path.join(__dirname, 'scan_map.pmap');
const fileRemaps = {};    // "cost/Cost.java" -> "cost/mod.rs"
const symbolRemaps = {};  // "cost/CostPayment.java" -> { "getMana": "mana" }

if (fs.existsSync(SCAN_MAP_PATH)) {
  const lines = fs.readFileSync(SCAN_MAP_PATH, 'utf-8').split('\n');
  for (const raw of lines) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const parts = line.split('->').map(s => s.trim());
    if (parts.length !== 2) continue;
    const [left, right] = parts;

    // Count slashes to distinguish file vs symbol remaps
    const leftParts = left.split('/');
    const lastLeft = leftParts[leftParts.length - 1];

    if (!lastLeft.includes('.')) {
      // Symbol remap: cost/CostPayment.java/getMana -> cost/cost_payment.rs/mana
      const javaFile = leftParts.slice(0, -1).join('/');
      const javaSymbol = lastLeft;
      const rightParts = right.split('/');
      const rustSymbol = rightParts[rightParts.length - 1];
      const rustFile = rightParts.slice(0, -1).join('/');
      if (!symbolRemaps[javaFile]) symbolRemaps[javaFile] = {};
      symbolRemaps[javaFile][javaSymbol] = { rustFile, rustSymbol };
    } else {
      // File remap: cost/Cost.java -> cost/mod.rs
      fileRemaps[left] = right;
    }
  }
}

const BLUE = '\x1b[1;34m';

// Colors
const GREEN = '\x1b[0;32m';
const RED = '\x1b[0;31m';
const YELLOW = '\x1b[0;33m';
const CYAN = '\x1b[1;36m';
const WHITE = '\x1b[0;37m';
const BOLD = '\x1b[1m';
const DIM = '\x1b[2m';
const RESET = '\x1b[0m';

// Java interface pattern: starts with I followed by an uppercase letter (e.g. ICombat, IIdentifiable)
function isJavaInterface(filename) {
  const name = filename.replace(/\.java$/, '');
  return /^I[A-Z]/.test(name);
}

function camelToSnake(name) {
  return name
    .replace(/\.java$/, '')
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1_$2')
    .toLowerCase() + '.rs';
}

// Convert a camelCase method name to snake_case
function methodToSnake(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1_$2')
    .toLowerCase();
}

// Java methods that don't translate 1:1 to Rust free functions
const SKIP_METHODS = new Set([
  // toString/hashCode/equals/clone/compareTo -> traits/derives in Rust
  'toString', 'hashCode', 'equals', 'clone', 'compareTo',
  // visitor pattern -> enums + match in Rust
  'accept',
  // serialization
  'readObject', 'writeObject', 'readResolve',
]);

// Getter/setter pattern: getFoo, setFoo, isFoo
function isGetterSetter(name) {
  return /^(get|set|is)[A-Z]/.test(name);
}

// Extract public method names from a Java file (skip constructors, boilerplate)
function extractJavaMethods(filePath) {
  const content = fs.readFileSync(filePath, 'utf-8');
  const className = path.basename(filePath, '.java');
  const methods = [];
  const re = /^\s*public\s+(?:static\s+)?(?:final\s+)?(?:synchronized\s+)?(?:<[^>]+>\s+)?(\S+)\s+([a-zA-Z_]\w*)\s*\(/gm;
  let m;
  while ((m = re.exec(content)) !== null) {
    const methodName = m[2];
    if (methodName === className) continue;
    if (SKIP_METHODS.has(methodName)) continue;
    if (isGetterSetter(methodName)) continue;
    methods.push(methodName);
  }
  // Deduplicate (overloaded methods)
  return [...new Set(methods)];
}

// Extract pub fn names from a Rust file
function extractRustFns(filePath) {
  const content = fs.readFileSync(filePath, 'utf-8');
  const fns = [];
  const re = /pub\s+(?:async\s+)?fn\s+([a-z_]\w*)\s*[(<]/gm;
  let m;
  while ((m = re.exec(content)) !== null) {
    fns.push(m[1]);
  }
  return new Set(fns);
}

function walkJava(dir, rel = '') {
  const results = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    const relPath = rel ? `${rel}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      results.push(...walkJava(fullPath, relPath));
    } else if (entry.name.endsWith('.java')) {
      results.push(relPath);
    }
  }
  return results;
}

const javaFiles = walkJava(JAVA_ROOT).sort();

let totalFiles = 0;
let totalPorted = 0;
let totalSymbols = 0;
let totalSymbolsPorted = 0;
const moduleStats = {}; // module -> { ported, total, symbols, symbolsPorted }

let currentDir = null;

for (const jfile of javaFiles) {
  const dir = path.dirname(jfile);
  const base = path.basename(jfile);
  const expectedRs = camelToSnake(base);

  let rustPath, module;
  let fileRemapped = false;
  const baseName = base.replace(/\.java$/, '').toLowerCase();

  // Check file remap first
  if (fileRemaps[jfile]) {
    rustPath = path.join(RUST_ROOT, fileRemaps[jfile]);
    module = dir === '.' ? '(root)' : dir.split('/')[0];
    fileRemapped = true;
  } else if (dir === '.') {
    rustPath = path.join(RUST_ROOT, expectedRs);
    module = '(root)';
  } else {
    rustPath = path.join(RUST_ROOT, dir, expectedRs);
    module = dir.split('/')[0];
    // If class name matches folder name (case insensitive), also check mod.rs
    const folderName = path.basename(dir).toLowerCase();
    if (baseName === folderName && !fs.existsSync(rustPath)) {
      const modRsPath = path.join(RUST_ROOT, dir, 'mod.rs');
      if (fs.existsSync(modRsPath)) {
        rustPath = modRsPath;
      }
    }
  }

  // Skip files not in the target module when filtering
  if (filterModule && module !== filterModule) continue;

  if (dir !== currentDir) {
    currentDir = dir;
    console.log('');
    process.stdout.write(`${CYAN}${BOLD}  📂 ${dir}/${RESET}\n`);
  }

  const isInterface = isJavaInterface(base);
  const exists = fs.existsSync(rustPath);
  const displayRs = fileRemapped ? fileRemaps[jfile] : (rustPath.endsWith('mod.rs') ? 'mod.rs' : expectedRs);
  const overrideTag = fileRemapped ? ` ${BLUE}⬡ mapped${RESET}` : '';

  if (!moduleStats[module]) moduleStats[module] = { ported: 0, total: 0, symbols: 0, symbolsPorted: 0 };

  if (!isInterface) {
    totalFiles++;
    moduleStats[module].total++;
  }

  const padded = base.padEnd(45);
  if (isInterface && !exists) {
    process.stdout.write(`${WHITE}     ${padded} -> interface (skipped)${RESET}\n`);
  } else if (exists) {
    if (!isInterface) {
      totalPorted++;
      moduleStats[module].ported++;
    }

    // Symbol matching (only for ported, non-interface files)
    if (showSymbols && !isInterface) {
      const javaPath = dir === '.' ? path.join(JAVA_ROOT, base) : path.join(JAVA_ROOT, dir, base);
      const javaMethods = extractJavaMethods(javaPath);
      const rustFns = extractRustFns(rustPath);

      // Also load symbols from remap target files
      const fileSymRemaps = symbolRemaps[jfile] || {};
      const extraRustFnSets = {};  // rustFile -> Set of fns (lazy loaded)

      const matched = [];
      const missing = [];
      for (const m of javaMethods) {
        const snake = methodToSnake(m);

        if (fileSymRemaps[m]) {
          // Symbol has a remap override
          const remap = fileSymRemaps[m];
          const remapRustPath = path.join(RUST_ROOT, remap.rustFile);
          if (!extraRustFnSets[remap.rustFile]) {
            extraRustFnSets[remap.rustFile] = fs.existsSync(remapRustPath)
              ? extractRustFns(remapRustPath) : new Set();
          }
          const targetFns = extraRustFnSets[remap.rustFile];
          if (targetFns.has(remap.rustSymbol)) {
            matched.push({ java: m, rust: remap.rustSymbol, remapped: true, target: remap.rustFile });
          } else {
            missing.push({ java: m, rust: remap.rustSymbol, remapped: true, target: remap.rustFile });
          }
        } else if (rustFns.has(snake)) {
          matched.push({ java: m, rust: snake });
        } else {
          missing.push({ java: m, rust: snake });
        }
      }

      const symTotal = javaMethods.length;
      const symPorted = matched.length;
      totalSymbols += symTotal;
      totalSymbolsPorted += symPorted;
      moduleStats[module].symbols += symTotal;
      moduleStats[module].symbolsPorted += symPorted;

      if (symTotal > 0) {
        const symPct = Math.floor((symPorted / symTotal) * 100);
        let symColor;
        if (symPct >= 80) symColor = GREEN;
        else if (symPct >= 40) symColor = YELLOW;
        else symColor = RED;
        process.stdout.write(`${GREEN}     ${padded} -> ${displayRs}${overrideTag}  ${symColor}${symPorted}/${symTotal} symbols (${symPct}%)${RESET}\n`);
      } else {
        process.stdout.write(`${GREEN}     ${padded} -> ${displayRs}${overrideTag}${RESET}\n`);
      }

      for (const s of matched) {
        if (s.remapped) {
          process.stdout.write(`${DIM}        ${BLUE}⬡ ${s.java} -> ${s.rust} (${s.target})${RESET}\n`);
        } else {
          process.stdout.write(`${DIM}        ${GREEN}✓ ${s.java} -> ${s.rust}${RESET}\n`);
        }
      }
      for (const s of missing) {
        if (s.remapped) {
          process.stdout.write(`${DIM}        ${BLUE}✗ ${s.java} -> ${s.rust} (${s.target})${RESET}\n`);
        } else {
          process.stdout.write(`${DIM}        ${RED}✗ ${s.java} -> ${s.rust}${RESET}\n`);
        }
      }
    } else {
      process.stdout.write(`${GREEN}     ${padded} -> ${displayRs}${overrideTag}${RESET}\n`);
    }
  } else {
    if (fileRemapped) {
      process.stdout.write(`${RED}     ${padded} -> missing${RESET} ${BLUE}⬡ mapped (${displayRs})${RESET}\n`);
    } else {
      process.stdout.write(`${RED}     ${padded} -> missing${RESET}\n`);
    }
  }
}

// Summary
console.log('\n');
console.log(`${BOLD}═══════════════════════════════════════════════════════════════${RESET}`);
console.log(`${BOLD}  PORT COVERAGE REPORT${RESET}`);
console.log(`${BOLD}═══════════════════════════════════════════════════════════════${RESET}`);
console.log('');

const sortedModules = Object.keys(moduleStats).sort();

for (const mod of sortedModules) {
  const { ported, total } = moduleStats[mod];
  const pct = total > 0 ? Math.floor((ported / total) * 100) : 0;

  let color;
  if (pct >= 80) color = GREEN;
  else if (pct >= 40) color = YELLOW;
  else color = RED;

  const filled = Math.floor(pct / 5);
  const empty = 20 - filled;
  const bar = '█'.repeat(filled) + '░'.repeat(empty);

  const modPad = mod.padEnd(20);
  const pctStr = String(pct).padStart(3);
  if (showSymbols && moduleStats[mod].symbols > 0) {
    const symPct = Math.floor((moduleStats[mod].symbolsPorted / moduleStats[mod].symbols) * 100);
    let symColor;
    if (symPct >= 80) symColor = GREEN;
    else if (symPct >= 40) symColor = YELLOW;
    else symColor = RED;
    process.stdout.write(`  ${BOLD}${modPad}${RESET} ${color}${bar} ${pctStr}%${RESET}  ${DIM}(${ported}/${total})${RESET}  ${symColor}symbols: ${symPct}%${RESET} ${DIM}(${moduleStats[mod].symbolsPorted}/${moduleStats[mod].symbols})${RESET}\n`);
  } else {
    process.stdout.write(`  ${BOLD}${modPad}${RESET} ${color}${bar} ${pctStr}%${RESET}  ${DIM}(${ported}/${total})${RESET}\n`);
  }
}

console.log('');

const overallPct = totalFiles > 0 ? Math.floor((totalPorted / totalFiles) * 100) : 0;
let overallColor;
if (overallPct >= 80) overallColor = GREEN;
else if (overallPct >= 40) overallColor = YELLOW;
else overallColor = RED;

if (showSymbols && totalSymbols > 0) {
  const symOverallPct = Math.floor((totalSymbolsPorted / totalSymbols) * 100);
  let symOverallColor;
  if (symOverallPct >= 80) symOverallColor = GREEN;
  else if (symOverallPct >= 40) symOverallColor = YELLOW;
  else symOverallColor = RED;
  process.stdout.write(`${BOLD}  OVERALL:${RESET}            ${overallColor}${String(overallPct).padStart(3)}%${RESET}  ${DIM}(${totalPorted}/${totalFiles} files)${RESET}  ${symOverallColor}symbols: ${symOverallPct}%${RESET} ${DIM}(${totalSymbolsPorted}/${totalSymbols})${RESET}\n`);
} else {
  process.stdout.write(`${BOLD}  OVERALL:${RESET}            ${overallColor}${String(overallPct).padStart(3)}%${RESET}  ${DIM}(${totalPorted}/${totalFiles} files ported)${RESET}\n`);
}
console.log('');
console.log(`${BOLD}═══════════════════════════════════════════════════════════════${RESET}`);
