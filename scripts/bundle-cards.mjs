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
const OUTPUT_FILE = path.join(PROJECT_ROOT, 'public/wasm/cards-bundle.json');

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

  if (!fs.existsSync(letterDir)) {
    return null;
  }

  // Try exact match first
  const exactPath = path.join(letterDir, `${normalized}.txt`);
  if (fs.existsSync(exactPath)) {
    return exactPath;
  }

  // Try files in the directory
  const files = fs.readdirSync(letterDir);
  for (const file of files) {
    if (file.endsWith('.txt')) {
      const content = fs.readFileSync(path.join(letterDir, file), 'utf-8');
      // Check if the Name: field matches
      const nameMatch = content.match(/^Name:(.+)$/m);
      if (nameMatch && nameMatch[1].trim().toLowerCase() === cardName.toLowerCase()) {
        return path.join(letterDir, file);
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
      cardNames.add(card.name);
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
      cards: deck.cards
    });
  }

  const presetsFile = path.join(path.dirname(OUTPUT_FILE), 'preset-decks.json');
  fs.writeFileSync(presetsFile, JSON.stringify(presets, null, 2));
  console.log(`Preset decks written to: ${presetsFile}`);
}

bundleCards();
bundlePresetDecks();
console.log('Done!');
