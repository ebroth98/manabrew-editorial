use std::path::{Path, PathBuf};
use std::time::SystemTime;

fn main() {
    // Order matters: tauri_build validates that every resource listed in
    // tauri.conf.json exists. The cardset archive is one of those resources,
    // so it must be present *before* tauri_build runs.
    build_cardset_archive_if_stale();
    tauri_build::build();
}

/// Regenerate `resources/cardset.rkyv` whenever the cardsfolder is newer than
/// the existing archive (or the archive is missing). The archive is bundled as
/// a Tauri resource and mmap'd at runtime by `card_db.rs`.
fn build_cardset_archive_if_stale() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cardsfolder = manifest_dir.join("../forge/forge-gui/res/cardsfolder");
    let archive_path = manifest_dir.join("resources/cardset.rkyv");

    println!("cargo:rerun-if-changed={}", cardsfolder.display());
    println!("cargo:rerun-if-env-changed=FORCE_CARDSET_REBUILD");

    if !cardsfolder.exists() {
        println!(
            "cargo:warning=cardsfolder not found at {}, skipping cardset archive build",
            cardsfolder.display()
        );
        return;
    }

    if !needs_rebuild(&cardsfolder, &archive_path) {
        return;
    }

    match forge_cardset_archive::build_archive_from_dir(&cardsfolder, &archive_path) {
        Ok(stats) => {
            println!(
                "cargo:warning=cardset archive built: {} cards, {:.2} MiB",
                stats.cards,
                stats.bytes_written as f64 / 1024.0 / 1024.0
            );
        }
        Err(err) => {
            panic!("failed to build cardset archive: {err}");
        }
    }
}

fn needs_rebuild(cardsfolder: &Path, archive: &Path) -> bool {
    let archive_mtime = match archive.metadata().and_then(|m| m.modified()) {
        Ok(m) => m,
        Err(_) => return true,
    };
    match latest_mtime(cardsfolder) {
        Some(card_mtime) => card_mtime > archive_mtime,
        None => true,
    }
}

fn latest_mtime(dir: &Path) -> Option<SystemTime> {
    let mut latest: Option<SystemTime> = None;
    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        // walkdir's metadata returns walkdir::Error, but Metadata::modified
        // returns io::Error — they don't compose via and_then.
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                latest = Some(latest.map_or(modified, |cur| cur.max(modified)));
            }
        }
    }
    latest
}
