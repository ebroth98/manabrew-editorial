//! File-system cache for Java harness output.
//!
//! Avoids spawning the Java harness for matchups whose output hasn't changed.
//! Cache is keyed on:
//! - A **source hash** covering all Java source files + deck definitions
//! - Per-matchup parameters (deck1, deck2, seed, max_turns, prefer_actions)
//!
//! When the source hash changes the entire cache is wiped (cheap — just delete
//! the directory).  Individual entries are stored as compressed JSON files so
//! they are portable between local dev, CI artefacts and Docker volumes.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::java_bridge::JavaMatchupData;
use crate::protocol::{DecisionRecord, StateSnapshot};

/// Lightweight wrapper that manages a directory of cached Java matchup outputs.
pub struct JavaCache {
    cache_dir: PathBuf,
    source_hash: String,
}

/// Parameters that uniquely identify a matchup (together with the source hash).
#[derive(Hash)]
struct MatchupKey<'a> {
    deck1: &'a str,
    deck2: &'a str,
    seed: u64,
    max_turns: u32,
    prefer_actions: bool,
}

// Minimal serde wrappers so we can store JavaMatchupData as JSON without
// requiring Serialize/Deserialize on the original struct.
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedMatchup {
    snapshots: Vec<StateSnapshot>,
    decisions: Vec<DecisionRecord>,
}

impl From<&JavaMatchupData> for CachedMatchup {
    fn from(d: &JavaMatchupData) -> Self {
        Self {
            snapshots: d.snapshots.clone(),
            decisions: d.decisions.clone(),
        }
    }
}

impl From<CachedMatchup> for JavaMatchupData {
    fn from(c: CachedMatchup) -> Self {
        Self {
            snapshots: c.snapshots,
            decisions: c.decisions,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Manifest {
    source_hash: String,
    version: u32,
}

const MANIFEST_FILE: &str = "manifest.json";
const CACHE_VERSION: u32 = 1;

impl JavaCache {
    /// Open (or create) a cache directory.
    ///
    /// `source_hash` is an opaque string that identifies the current Java
    /// source + deck definitions.  When it changes the entire cache is wiped.
    pub fn open(cache_dir: &Path, source_hash: String) -> std::io::Result<Self> {
        fs::create_dir_all(cache_dir)?;

        // Check manifest — if source hash differs, wipe everything.
        let manifest_path = cache_dir.join(MANIFEST_FILE);
        let needs_wipe = if manifest_path.exists() {
            match fs::read_to_string(&manifest_path) {
                Ok(s) => match serde_json::from_str::<Manifest>(&s) {
                    Ok(m) => m.source_hash != source_hash || m.version != CACHE_VERSION,
                    Err(_) => true,
                },
                Err(_) => true,
            }
        } else {
            // No manifest — fresh cache, no wipe needed.
            false
        };

        if needs_wipe {
            eprintln!(
                "[java-cache] Source hash changed — wiping cache at {}",
                cache_dir.display()
            );
            // Remove all files except the directory itself
            for entry in fs::read_dir(cache_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let _ = fs::remove_dir_all(&path);
                } else {
                    let _ = fs::remove_file(&path);
                }
            }
        }

        // Write fresh manifest
        let manifest = Manifest {
            source_hash: source_hash.clone(),
            version: CACHE_VERSION,
        };
        fs::write(&manifest_path, serde_json::to_string(&manifest)?)?;

        Ok(Self {
            cache_dir: cache_dir.to_path_buf(),
            source_hash,
        })
    }

    /// Look up a cached matchup.  Returns `None` on miss or corruption.
    pub fn get(
        &self,
        deck1: &str,
        deck2: &str,
        seed: u64,
        max_turns: u32,
        prefer_actions: bool,
    ) -> Option<JavaMatchupData> {
        let path = self.entry_path(deck1, deck2, seed, max_turns, prefer_actions);
        let bytes = fs::read(&path).ok()?;
        let cached: CachedMatchup = match serde_json::from_slice(&bytes) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "[java-cache] Corrupt entry {}, removing: {}",
                    path.display(),
                    e
                );
                let _ = fs::remove_file(&path);
                return None;
            }
        };
        Some(cached.into())
    }

    /// Store a matchup result.  Uses atomic write (temp + rename) to be safe
    /// under concurrent access from rayon threads.
    pub fn put(
        &self,
        deck1: &str,
        deck2: &str,
        seed: u64,
        max_turns: u32,
        prefer_actions: bool,
        data: &JavaMatchupData,
    ) -> std::io::Result<()> {
        let path = self.entry_path(deck1, deck2, seed, max_turns, prefer_actions);

        // Ensure shard directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let cached = CachedMatchup::from(data);
        let json = serde_json::to_vec(&cached)?;

        // Atomic write: write to temp file, then rename
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, &json)?;
        fs::rename(&tmp, &path)?;

        Ok(())
    }

    /// Number of cached entries (for stats logging).
    pub fn len(&self) -> usize {
        let mut count = 0;
        for entry in fs::read_dir(&self.cache_dir).into_iter().flatten() {
            if let Ok(entry) = entry {
                if entry.path().is_dir() && entry.file_name() != "." {
                    for sub in fs::read_dir(entry.path()).into_iter().flatten() {
                        if let Ok(sub) = sub {
                            if sub.path().extension().map(|e| e == "json").unwrap_or(false)
                                && sub.file_name() != MANIFEST_FILE
                            {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        count
    }

    pub fn source_hash(&self) -> &str {
        &self.source_hash
    }

    // ── internal ──────────────────────────────────────────────────────

    fn entry_path(
        &self,
        deck1: &str,
        deck2: &str,
        seed: u64,
        max_turns: u32,
        prefer_actions: bool,
    ) -> PathBuf {
        let key = MatchupKey {
            deck1,
            deck2,
            seed,
            max_turns,
            prefer_actions,
        };
        let hash = {
            let mut h = DefaultHasher::new();
            self.source_hash.hash(&mut h);
            key.hash(&mut h);
            format!("{:016x}", h.finish())
        };
        // Shard by first 2 hex chars to avoid giant flat directories
        let shard = &hash[..2];
        self.cache_dir.join(shard).join(format!("{}.json", hash))
    }
}

/// Compute a source hash covering all Java source files that affect harness
/// output.  This includes:
/// - `forge/forge-harness/src/` (the team's harness code)
/// - `forge/forge-game/src/`   (the reference Java engine)
/// - `forge/forge-core/src/`   (core types)
/// - `forge/forge-ai/src/`     (AI module, used by game)
/// - `preset_decks/`           (deck definitions)
///
/// We hash file paths + contents to detect additions, deletions, and edits.
/// Uses a stable sort so the hash is deterministic across platforms.
pub fn compute_source_hash(project_root: &Path) -> String {
    let dirs_to_hash = [
        "forge/forge-harness/src",
        "forge/forge-game/src",
        "forge/forge-core/src",
        "forge/forge-ai/src",
        "preset_decks",
    ];

    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

    for dir in &dirs_to_hash {
        let full = project_root.join(dir);
        if !full.exists() {
            continue;
        }
        collect_files(&full, &full, &mut entries);
    }

    // Stable sort by relative path for determinism
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = DefaultHasher::new();
    for (rel_path, content) in &entries {
        rel_path.hash(&mut hasher);
        content.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
}

/// Compute a source hash from the JAR file itself (alternative when source
/// dirs aren't available, e.g. in Docker where only the JAR is present).
pub fn compute_jar_hash(jar_path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(jar_path)?;
    let mut hasher = DefaultHasher::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        buf[..n].hash(&mut hasher);
    }
    Ok(format!("{:016x}", hasher.finish()))
}

fn collect_files(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, out);
        } else if path.is_file() {
            // Only hash source-like files
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "java" | "json" | "xml" | "properties") {
                if let Ok(content) = fs::read(&path) {
                    let rel = path
                        .strip_prefix(base)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();
                    out.push((rel, content));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{DecisionRecord, PlayerSnapshot, StateSnapshot};

    fn dummy_data() -> JavaMatchupData {
        JavaMatchupData {
            snapshots: vec![StateSnapshot {
                turn: 1,
                phase: "Untap".into(),
                active_player: 0,
                game_over: false,
                winner: None,
                players: vec![PlayerSnapshot {
                    name: "P1".into(),
                    index: 0,
                    life: 20,
                    poison: 0,
                    lands_played: 0,
                    has_lost: false,
                    has_won: false,
                    hand: vec!["Mountain".into()],
                    battlefield: vec![],
                    graveyard: vec![],
                    exile: vec![],
                    library_size: 50,
                }],
                stack: vec![],
            }],
            decisions: vec![DecisionRecord {
                turn: 1,
                phase: "Main1".into(),
                deciding_player: 0,
                kind: "main_action".into(),
                options: vec!["PASS".into()],
                choice: "PASS".into(),
            }],
        }
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let cache = JavaCache::open(dir.path(), "test_hash".into()).unwrap();

        // Miss
        assert!(cache.get("a", "b", 42, 10, false).is_none());

        // Put + hit
        let data = dummy_data();
        cache.put("a", "b", 42, 10, false, &data).unwrap();
        let got = cache.get("a", "b", 42, 10, false).unwrap();
        assert_eq!(got.snapshots.len(), 1);
        assert_eq!(got.decisions.len(), 1);
        assert_eq!(got.snapshots[0].turn, 1);

        // Different params → miss
        assert!(cache.get("a", "b", 99, 10, false).is_none());
    }

    #[test]
    fn wipes_on_hash_change() {
        let dir = tempfile::tempdir().unwrap();

        let cache1 = JavaCache::open(dir.path(), "hash_v1".into()).unwrap();
        cache1.put("a", "b", 42, 10, false, &dummy_data()).unwrap();
        assert!(cache1.get("a", "b", 42, 10, false).is_some());

        // Reopen with different hash → cache wiped
        let cache2 = JavaCache::open(dir.path(), "hash_v2".into()).unwrap();
        assert!(cache2.get("a", "b", 42, 10, false).is_none());
    }
}
