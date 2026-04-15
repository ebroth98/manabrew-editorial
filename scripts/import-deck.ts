#!/usr/bin/env node
// Deck importer CLI.
// Usage:
//   yarn import-deck "<query>" [--format=<format>]
//   yarn import-deck --url=<archidekt-url> [--format=<format>]
// Searches Archidekt (or jumps directly to a deck when --url is passed),
// prompts for a selection, and writes a preset deck JSON file to preset_decks/.

import { readdir, readFile, writeFile } from "node:fs/promises";
import { createInterface } from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";

import {
  ARCHIDEKT_PAGE_SIZE,
  fetchArchidektDeck,
  searchArchidekt,
  type ArchidektDeck,
  type ArchidektSearchResult,
} from "../src/lib/archidekt.ts";
import {
  fetchDeckBySource,
  fetchResultBySource,
  parseDeckUrl,
} from "../src/lib/deckImport.ts";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PRESET_DIR = resolve(__dirname, "..", "preset_decks");

const USE_COLOR = process.stdout.isTTY && process.env.NO_COLOR == null;
const ANSI = {
  reset: "\x1b[0m",
  bold: "\x1b[1m",
  dim: "\x1b[2m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  cyan: "\x1b[36m",
  gray: "\x1b[90m",
} as const;
const paint = (code: string, s: string) => (USE_COLOR ? `${code}${s}${ANSI.reset}` : s);
const bold = (s: string) => paint(ANSI.bold, s);
const dim = (s: string) => paint(ANSI.dim, s);
const cyan = (s: string) => paint(ANSI.cyan, s);
const green = (s: string) => paint(ANSI.green, s);
const yellow = (s: string) => paint(ANSI.yellow, s);
const red = (s: string) => paint(ANSI.red, s);
const gray = (s: string) => paint(ANSI.gray, s);

const rule = (char = "─", width = 60) => gray(char.repeat(width));
const header = (title: string) => `\n${bold(cyan("▸ " + title))}\n${rule()}`;

const COLOR_MAP: Record<string, string> = {
  W: "text-yellow-200",
  U: "text-blue-400",
  B: "text-zinc-400",
  R: "text-red-400",
  G: "text-green-400",
};

interface CliArgs {
  query: string;
  format: string;
  url: string;
}

function parseArgs(argv: string[]): CliArgs {
  const args: CliArgs = { query: "", format: "standard", url: "" };
  const rest: string[] = [];
  for (const a of argv) {
    if (a.startsWith("--format=")) args.format = a.slice("--format=".length);
    else if (a.startsWith("--url=")) args.url = a.slice("--url=".length);
    else rest.push(a);
  }
  args.query = rest.join(" ").trim();
  return args;
}

function pickColor(colors: string[]): string {
  if (!colors || colors.length === 0) return "text-gray-400";
  if (colors.length === 1) return COLOR_MAP[colors[0]] ?? "text-gray-400";
  return "text-amber-300";
}

function slugify(str: string): string {
  return (
    str
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "")
      .slice(0, 60) || "imported_deck"
  );
}

async function nextOrder(): Promise<number> {
  const files = await readdir(PRESET_DIR);
  let max = 0;
  for (const f of files) {
    if (!f.endsWith(".json")) continue;
    try {
      const data = JSON.parse(await readFile(join(PRESET_DIR, f), "utf8")) as { order?: number };
      if (typeof data.order === "number" && data.order > max) max = data.order;
    } catch {
      // ignore malformed preset
    }
  }
  return max + 1;
}

function truncate(str: string | undefined, len: number): string {
  if (!str) return "";
  return str.length > len ? str.slice(0, len - 1) + "…" : str;
}

type Rl = ReturnType<typeof createInterface>;

async function promptIndex(rl: Rl, max: number): Promise<number> {
  while (true) {
    const answer = (
      await rl.question(`\n${cyan("?")} Select deck ${gray(`[1-${max}, q to quit]`)} ${cyan("›")} `)
    ).trim();
    if (answer.toLowerCase() === "q") return -1;
    const n = Number(answer);
    if (Number.isInteger(n) && n >= 1 && n <= max) return n - 1;
    console.log(red("  ✗ invalid selection"));
  }
}

async function promptAction(
  rl: Rl,
  { allowBack = true }: { allowBack?: boolean } = {},
): Promise<"import" | "back" | "quit"> {
  const hint = allowBack ? "[i]mport · [b]ack · [q]uit" : "[i]mport · [q]uit";
  while (true) {
    const answer = (await rl.question(`\n${cyan("?")} ${gray(hint)} ${cyan("›")} `))
      .trim()
      .toLowerCase();
    if (answer === "i" || answer === "import" || answer === "") return "import";
    if (allowBack && (answer === "b" || answer === "back")) return "back";
    if (answer === "q" || answer === "quit") return "quit";
    console.log(red("  ✗ invalid choice"));
  }
}

function renderDeckDetails(result: ArchidektSearchResult, deck: ArchidektDeck) {
  console.log(header("Deck details"));
  const allCards = [...deck.commanders, ...deck.cards];
  const totalCount = allCards.reduce((s, c) => s + c.count, 0);
  const colors = (deck.colors ?? []).join("") || "—";
  const descFirst = (deck.description ?? "").split("\n").find((l) => l.trim()) ?? "";
  console.log(`  ${dim("name   :")} ${bold(deck.name ?? result.name)}`);
  console.log(`  ${dim("author :")} ${result.author ?? "—"}`);
  if (result.format) console.log(`  ${dim("format :")} ${result.format}`);
  console.log(`  ${dim("colors :")} ${colors}`);
  console.log(`  ${dim("unique :")} ${allCards.length}  ${dim("total:")} ${totalCount}`);
  if (deck.commanders.length) {
    console.log(
      `  ${dim("cmdr   :")} ${deck.commanders.map((c) => c.name).join(", ")}`,
    );
  }
  if (descFirst) console.log(`  ${dim("desc   :")} ${truncate(descFirst, 120)}`);

  const sorted = [...allCards].sort(
    (a, b) => b.count - a.count || a.name.localeCompare(b.name),
  );
  if (sorted.length) {
    console.log(`\n  ${dim(`cards (${sorted.length}):`)}`);
    const termWidth = process.stdout.columns || 100;
    const colMax = 46;
    const gap = 4;
    const cols = termWidth >= colMax * 2 + gap + 4 ? 2 : 1;
    const colWidth = Math.min(colMax, Math.floor((termWidth - 4 - gap * (cols - 1)) / cols));
    const formatEntry = (card: { name: string; count: number }) => {
      const count = yellow(String(card.count).padStart(2));
      const nameMax = colWidth - 5;
      const name = truncate(card.name, nameMax).padEnd(nameMax);
      return `${count} × ${name}`;
    };
    const rows = Math.ceil(sorted.length / cols);
    for (let r = 0; r < rows; r++) {
      const parts: string[] = [];
      for (let col = 0; col < cols; col++) {
        const entry = sorted[col * rows + r];
        if (entry) parts.push(formatEntry(entry));
      }
      console.log("    " + parts.join(" ".repeat(gap)));
    }
  }
  console.log(rule());
}

function renderResults(results: ArchidektSearchResult[]) {
  console.log(header(`Results (${results.length})`));
  const indexWidth = String(results.length).length;
  const nameWidth = Math.min(
    50,
    results.reduce((m, r) => Math.max(m, r.name.length), 0),
  );
  const indent = " ".repeat(indexWidth + 4);
  results.forEach((r, i) => {
    const idx = yellow(String(i + 1).padStart(indexWidth));
    const name = truncate(r.name, 50).padEnd(nameWidth);
    const author = r.author ? gray(`by ${r.author}`) : "";
    const format = r.format ? gray(`· ${r.format}`) : "";
    console.log(`  ${idx}  ${name}  ${author} ${format}`);
    const blurb = r.description || (r.tags.length ? `tags: ${r.tags.join(", ")}` : "");
    if (blurb) console.log(`${indent}${dim(truncate(blurb, 90))}`);
  });
  console.log(rule(" ", 60));
}

async function main() {
  const { query, format, url } = parseArgs(process.argv.slice(2));
  if (!query && !url) {
    console.error(red("✗ Missing query or --url."));
    console.error(dim('  Usage: yarn import-deck "<query>" [--format=<format>]'));
    console.error(dim("         yarn import-deck --url=<archidekt-url> [--format=<format>]"));
    process.exit(1);
  }

  const parsedUrl = url ? parseDeckUrl(url) : null;
  if (url && !parsedUrl) {
    console.error(red(`✗ Not a valid Archidekt or Moxfield URL: ${url}`));
    process.exit(1);
  }

  console.log(header("Deck Importer"));
  console.log(`  ${dim(parsedUrl ? "url   :" : "query :")} ${bold(parsedUrl ? url : query)}`);
  if (parsedUrl) console.log(`  ${dim("source:")} ${bold(parsedUrl.source)}`);
  console.log(`  ${dim("format:")} ${bold(format)}`);
  console.log(rule());

  const rl = createInterface({ input, output });
  let chosen: ArchidektSearchResult;
  let deck: ArchidektDeck;

  if (parsedUrl) {
    console.log(dim("  fetching deck…"));
    try {
      chosen = await fetchResultBySource(parsedUrl.source, parsedUrl.id);
      deck = await fetchDeckBySource(parsedUrl.source, parsedUrl.id);
    } catch (e) {
      rl.close();
      const msg = e instanceof Error ? e.message : String(e);
      console.error(red(`\n✗ ${msg}`));
      process.exit(1);
    }
    renderDeckDetails(chosen, deck);
    const action = await promptAction(rl, { allowBack: false });
    if (action !== "import") {
      rl.close();
      console.log(yellow("\n  cancelled."));
      return;
    }
  } else {
    console.log(dim("  searching…"));
    let results: ArchidektSearchResult[];
    try {
      results = await searchArchidekt(query, { pageSize: ARCHIDEKT_PAGE_SIZE });
    } catch (e) {
      rl.close();
      const msg = e instanceof Error ? e.message : String(e);
      console.error(red(`\n✗ ${msg}`));
      process.exit(1);
    }

    if (results.length === 0) {
      rl.close();
      console.error(red("\n✗ No decks found."));
      process.exit(1);
    }

    let picked: ArchidektSearchResult | null = null;
    let pickedDeck: ArchidektDeck | null = null;
    while (true) {
      renderResults(results);
      const idx = await promptIndex(rl, results.length);
      if (idx < 0) {
        rl.close();
        console.log(yellow("\n  cancelled."));
        return;
      }
      const candidate = results[idx];
      console.log(`\n${dim("  fetching deck…")}`);
      let fetched: ArchidektDeck;
      try {
        fetched = await fetchArchidektDeck(candidate.id);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        console.error(red(`  ✗ fetch failed: ${msg}`));
        continue;
      }
      renderDeckDetails(candidate, fetched);
      const action = await promptAction(rl);
      if (action === "import") {
        picked = candidate;
        pickedDeck = fetched;
        break;
      }
      if (action === "quit") {
        rl.close();
        console.log(yellow("\n  cancelled."));
        return;
      }
    }
    chosen = picked!;
    deck = pickedDeck!;
  }

  console.log(header("Preset metadata"));
  const labelAnswer = (
    await rl.question(`  ${dim("label    ")} ${gray(`[${chosen.name}]`)} ${cyan("›")} `)
  ).trim();
  const descAnswer = (
    await rl.question(`  ${dim("desc     ")} ${gray("(optional)")}          ${cyan("›")} `)
  ).trim();
  const fileAnswer = (
    await rl.question(`  ${dim("filename ")} ${gray(`[${slugify(chosen.name)}]`)} ${cyan("›")} `)
  ).trim();
  rl.close();

  const label = labelAnswer || chosen.name;
  const desc =
    descAnswer ||
    deck.description.split("\n")[0].slice(0, 120) ||
    "Imported from archidekt";
  const slug = fileAnswer || slugify(chosen.name);

  // Commanders live in a separate Archidekt category; the preset deck JSON has
  // no commander field today, so fold them back into `cards` for the CLI output.
  const allCards = [...deck.commanders, ...deck.cards].filter((c) => c.name && c.count > 0);
  const preset = {
    label,
    desc,
    color: pickColor(deck.colors),
    format,
    opponent: "",
    ai_eligible: false,
    order: await nextOrder(),
    cards: allCards,
  };

  const outPath = join(PRESET_DIR, `${slug}.json`);
  await writeFile(outPath, JSON.stringify(preset, null, 2) + "\n");

  const totalCount = preset.cards.reduce((s, c) => s + c.count, 0);
  console.log(header("Done"));
  console.log(`  ${green("✓")} ${bold(preset.label)}`);
  console.log(`  ${dim("file   :")} ${outPath}`);
  console.log(`  ${dim("entries:")} ${preset.cards.length}  ${dim("total:")} ${totalCount}`);
  console.log(`  ${dim("color  :")} ${preset.color}`);
  console.log(`  ${dim("format :")} ${preset.format}`);
  console.log(`  ${dim("order  :")} ${preset.order}`);
  console.log(rule() + "\n");
}

main().catch((err) => {
  const msg = err instanceof Error ? err.message : String(err);
  console.error(red(`\n✗ ${msg}`));
  process.exit(1);
});
