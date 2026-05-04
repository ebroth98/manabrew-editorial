#!/usr/bin/env node
/**
 * Bundle card scripts needed for preset decks into a JSON file.
 * This creates src/wasm/cards-bundle.json with all card scripts
 * needed to play the preset decks in the browser.
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.resolve(__dirname, '..');

const PRESET_DECKS_DIR = path.join(PROJECT_ROOT, 'preset_decks');
const CARDSFOLDER_DIR = path.join(PROJECT_ROOT, 'forge/forge-gui/res/cardsfolder');
const TOKEN_SCRIPTS_DIR = path.join(PROJECT_ROOT, 'forge/forge-gui/res/tokenscripts');
const UPCOMING_DIR = path.join(CARDSFOLDER_DIR, 'upcoming');
const EDITIONS_DIR = path.join(PROJECT_ROOT, 'forge/forge-gui/res/editions');
const OUTPUT_FILE = path.join(PROJECT_ROOT, 'public/wasm/cards-bundle.json');
const TOKEN_OUTPUT_FILE = path.join(PROJECT_ROOT, 'public/wasm/tokens-bundle.json');

function extractFlavorName(line) {
  const match = line.match(/"flavorName"\s*:\s*"([^"]+)"/);
  return match ? match[1].trim() : null;
}

function parseEditionFlavorAliasLine(line) {
  const flavorName = extractFlavorName(line);
  if (!flavorName) return null;

  const parts = line.trim().split(/\s+/);
  if (parts.length < 3) return null;

  const rest = line.trim().replace(/^\S+\s+\S+\s+/, '');
  const cardName = rest.split(/ @|\s\$\{/)[0]?.trim();
  if (!cardName) return null;

  return {
    flavorName,
    cardName,
  };
}

function buildFlavorAliasMap() {
  const aliases = new Map();
  if (!fs.existsSync(EDITIONS_DIR)) {
    return aliases;
  }

  const editionFiles = fs.readdirSync(EDITIONS_DIR).filter((file) => file.endsWith('.txt'));
  for (const file of editionFiles) {
    const contents = fs.readFileSync(path.join(EDITIONS_DIR, file), 'utf-8');
    let inEntries = false;

    for (const rawLine of contents.split('\n')) {
      const line = rawLine.trim();
      if (!line || line.startsWith('#')) continue;
      if (line.startsWith('[') && line.endsWith(']')) {
        const section = line.slice(1, -1);
        inEntries = section.toLowerCase() !== 'metadata';
        continue;
      }
      if (!inEntries) continue;

      const parsed = parseEditionFlavorAliasLine(line);
      if (!parsed) continue;
      aliases.set(parsed.flavorName.toLowerCase(), parsed.cardName);
    }
  }

  return aliases;
}

const FLAVOR_ALIAS_MAP = buildFlavorAliasMap();

function resolveCardName(name) {
  return FLAVOR_ALIAS_MAP.get(name.toLowerCase()) ?? name;
}

// Ensure output directory exists
const outputDir = path.dirname(OUTPUT_FILE);
if (!fs.existsSync(outputDir)) {
  fs.mkdirSync(outputDir, { recursive: true });
}

/**
 * Normalize card name to filename format (lowercase, underscores, no special chars)
 */
function normalizeCardName(name) {
  return name
    .toLowerCase()
    .replace(/['"]/g, '')
    .replace(/[^a-z0-9]/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_|_$/g, '');
}

/**
 * Find a card script file by card name.
 * Card scripts are organized by first letter.
 */
function findCardScript(cardName) {
  const normalized = normalizeCardName(cardName);
  const firstLetter = normalized[0] || 'a';
  const letterDir = path.join(CARDSFOLDER_DIR, firstLetter);

  const searchDirs = [];
  if (fs.existsSync(letterDir)) {
    searchDirs.push(letterDir);
  }
  if (fs.existsSync(UPCOMING_DIR)) {
    searchDirs.push(UPCOMING_DIR);
  }

  for (const dir of searchDirs) {
    const exactPath = path.join(dir, `${normalized}.txt`);
    if (fs.existsSync(exactPath)) {
      return exactPath;
    }
  }

  for (const dir of searchDirs) {
    const files = fs.readdirSync(dir);
    for (const file of files) {
      if (file.endsWith('.txt')) {
        const fullPath = path.join(dir, file);
        const content = fs.readFileSync(fullPath, 'utf-8');
        // Check if the Name: field matches
        const nameMatch = content.match(/^Name:(.+)$/m);
        if (nameMatch && nameMatch[1].trim().toLowerCase() === cardName.toLowerCase()) {
          return fullPath;
        }
      }
    }
  }

  return null;
}

/**
 * Load all preset decks and extract required card names.
 */
function getRequiredCards() {
  const cardNames = new Set();
  const presetFiles = fs.readdirSync(PRESET_DECKS_DIR)
    .filter(f => f.endsWith('.json'));

  for (const file of presetFiles) {
    const content = fs.readFileSync(path.join(PRESET_DECKS_DIR, file), 'utf-8');
    const deck = JSON.parse(content);

    for (const card of deck.cards || []) {
      cardNames.add(resolveCardName(card.name));
    }
  }

  // Add basic lands
  ['Plains', 'Island', 'Swamp', 'Mountain', 'Forest'].forEach(land => cardNames.add(land));

  return Array.from(cardNames);
}

/**
 * Main bundling function.
 */
function bundleCards() {
  console.log('Bundling card scripts for web...');

  const requiredCards = getRequiredCards();
  console.log(`Found ${requiredCards.length} unique cards in preset decks`);

  const bundle = {
    version: 1,
    generatedAt: new Date().toISOString(),
    cards: {}
  };

  let found = 0;
  let missing = 0;
  const missingCards = [];

  for (const cardName of requiredCards) {
    const scriptPath = findCardScript(cardName);

    if (scriptPath) {
      const content = fs.readFileSync(scriptPath, 'utf-8');
      const filename = path.basename(scriptPath, '.txt');
      bundle.cards[filename] = content;
      found++;
    } else {
      missing++;
      missingCards.push(cardName);
    }
  }

  console.log(`Found scripts for ${found} cards`);
  if (missing > 0) {
    console.warn(`Missing scripts for ${missing} cards:`);
    missingCards.slice(0, 10).forEach(name => console.warn(`  - ${name}`));
    if (missingCards.length > 10) {
      console.warn(`  ... and ${missingCards.length - 10} more`);
    }
  }

  // Write the bundle
  fs.writeFileSync(OUTPUT_FILE, JSON.stringify(bundle, null, 2));
  console.log(`Bundle written to: ${OUTPUT_FILE}`);
  console.log(`Bundle size: ${(fs.statSync(OUTPUT_FILE).size / 1024).toFixed(1)} KB`);
}

function bundleTokens() {
  console.log('Bundling token scripts for web...');

  const bundle = {
    version: 1,
    generatedAt: new Date().toISOString(),
    cards: {}
  };

  const tokenFiles = fs.readdirSync(TOKEN_SCRIPTS_DIR)
    .filter((file) => file.endsWith('.txt'))
    .sort();

  for (const file of tokenFiles) {
    const fullPath = path.join(TOKEN_SCRIPTS_DIR, file);
    const content = fs.readFileSync(fullPath, 'utf-8');
    const filename = path.basename(file, '.txt');
    bundle.cards[filename] = content;
  }

  fs.writeFileSync(TOKEN_OUTPUT_FILE, JSON.stringify(bundle, null, 2));
  console.log(`Token bundle written to: ${TOKEN_OUTPUT_FILE}`);
  console.log(`Token bundle size: ${(fs.statSync(TOKEN_OUTPUT_FILE).size / 1024).toFixed(1)} KB`);
  console.log(`Bundled ${tokenFiles.length} token scripts`);
}

// Also load and bundle preset deck metadata
function bundlePresetDecks() {
  const presets = [];
  const presetFiles = fs.readdirSync(PRESET_DECKS_DIR)
    .filter(f => f.endsWith('.json'));

  for (const file of presetFiles) {
    const id = path.basename(file, '.json');
    const content = fs.readFileSync(path.join(PRESET_DECKS_DIR, file), 'utf-8');
    const deck = JSON.parse(content);

    presets.push({
      id,
      label: deck.label,
      desc: deck.desc,
      color: deck.color,
      format: deck.format ?? "standard",
      commander: deck.commander,
      cards: (deck.cards || []).map((card) => ({
        ...card,
        name: resolveCardName(card.name),
      }))
    });
  }

  const presetsFile = path.join(path.dirname(OUTPUT_FILE), 'preset-decks.json');
  fs.writeFileSync(presetsFile, JSON.stringify(presets, null, 2));
  console.log(`Preset decks written to: ${presetsFile}`);
}

bundleCards();
bundleTokens();
bundlePresetDecks();
console.log('Done!');
