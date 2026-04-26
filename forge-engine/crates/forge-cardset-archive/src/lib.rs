//! Single-blob rkyv archive of the Forge cardset.
//!
//! At ~32K small `.txt` files in `cardsfolder/`, the cold-start cost of
//! reading every file individually dominates the cost of parsing them.
//! This crate packs the raw text of every card into one rkyv blob that
//! the runtime mmaps zero-copy. See `build_archive_from_dir` (the writer)
//! and `load_checked` / `load_unchecked` (the readers).

use std::path::Path;

use rkyv::{Archive, Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct Card {
    /// Lowercased canonical name — used as the binary-search key.
    pub name_lower: String,
    /// Raw card-script source, exactly as on disk.
    pub raw: String,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
#[archive(check_bytes)]
pub struct CardArchive {
    /// Cards sorted by `name_lower` (ascending), deduplicated.
    pub cards: Vec<Card>,
}

impl ArchivedCardArchive {
    pub fn lookup(&self, name: &str) -> Option<&ArchivedCard> {
        let key = name.to_ascii_lowercase();
        let idx = self
            .cards
            .binary_search_by(|c| c.name_lower.as_str().cmp(key.as_str()))
            .ok()?;
        Some(&self.cards[idx])
    }
}

impl ArchivedCard {
    /// Canonical display name extracted from the raw `Name:` field, falling
    /// back to the lowercased index key if the field is missing.
    pub fn display_name(&self) -> &str {
        self.raw
            .as_str()
            .lines()
            .find_map(|line| line.strip_prefix("Name:"))
            .unwrap_or(self.name_lower.as_str())
    }
}

/// Serialize a `CardArchive` to bytes. Returns rkyv's `AlignedVec` so the
/// caller can write it straight to disk without losing alignment.
pub fn to_bytes(archive: &CardArchive) -> Result<rkyv::AlignedVec, String> {
    rkyv::to_bytes::<_, 4_194_304>(archive).map_err(|e| format!("rkyv serialize: {e}"))
}

/// Validate and access an archive in-place, zero-copy.
///
/// `bytes` must be aligned to at least the archive's required alignment.
/// In practice an `mmap`'d region is page-aligned, which trivially satisfies
/// rkyv's requirement.
pub fn load_checked(bytes: &[u8]) -> Result<&ArchivedCardArchive, String> {
    rkyv::check_archived_root::<CardArchive>(bytes).map_err(|e| format!("invalid archive: {e}"))
}

/// Same as `load_checked` but skips bytecheck validation.
///
/// # Safety
///
/// `bytes` must be a valid rkyv archive of `CardArchive`, properly aligned.
pub unsafe fn load_unchecked(bytes: &[u8]) -> &ArchivedCardArchive {
    rkyv::archived_root::<CardArchive>(bytes)
}

#[derive(Debug, Default)]
pub struct BuildStats {
    pub cards: usize,
    pub skipped: usize,
    pub duplicates: usize,
    pub bytes_written: usize,
}

/// Walk `cardsfolder` recursively, extract each card's `Name:` field, and
/// write a sorted, deduplicated rkyv archive to `out_path`.
///
/// Card files are line-oriented `Key:Value`. We extract `Name:` directly
/// rather than going through the full parser because we only need it as
/// the lookup index.
pub fn build_archive_from_dir(
    cardsfolder: &Path,
    out_path: &Path,
) -> Result<BuildStats, String> {
    if !cardsfolder.exists() {
        return Err(format!("cardsfolder does not exist: {}", cardsfolder.display()));
    }

    let mut cards: Vec<Card> = Vec::with_capacity(35_000);
    let mut stats = BuildStats::default();

    for entry in WalkDir::new(cardsfolder).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let raw = match std::fs::read_to_string(entry.path()) {
            Ok(s) => s,
            Err(_) => {
                stats.skipped += 1;
                continue;
            }
        };
        let Some(name) = extract_name(&raw) else {
            stats.skipped += 1;
            continue;
        };
        cards.push(Card {
            name_lower: name.to_ascii_lowercase(),
            raw,
        });
    }

    cards.sort_by(|a, b| a.name_lower.cmp(&b.name_lower));
    let before_dedup = cards.len();
    cards.dedup_by(|a, b| a.name_lower == b.name_lower);
    stats.duplicates = before_dedup - cards.len();
    stats.cards = cards.len();

    let archive = CardArchive { cards };
    let bytes = to_bytes(&archive)?;
    stats.bytes_written = bytes.len();

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create out dir: {e}"))?;
    }
    std::fs::write(out_path, &bytes).map_err(|e| format!("write archive: {e}"))?;

    Ok(stats)
}

fn extract_name(raw: &str) -> Option<&str> {
    raw.lines().find_map(|line| line.strip_prefix("Name:"))
}
