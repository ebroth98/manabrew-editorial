//! Verifies that `CardDatabase::load_from_archive` produces the same result
//! as `CardDatabase::load_from_directory`.
//!
//! Run with:
//!   cargo run --release --example verify_archive -p forge-carddb
//!
//! Defaults assume the workspace layout (cardsfolder + archive + editions
//! at standard paths). Override via positional args:
//!   cargo run --example verify_archive -p forge-carddb -- \
//!     <cardsfolder> <archive_path> <editions_dir>

use std::path::PathBuf;
use std::time::Instant;

use forge_carddb::CardDatabase;
use memmap2::Mmap;

fn main() {
    let mut args = std::env::args().skip(1);
    let cardsfolder = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "forge/forge-gui/res/cardsfolder".to_string()),
    );
    let archive_path = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "src-tauri/resources/cardset.rkyv".to_string()),
    );
    let editions_dir = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "forge/forge-gui/res/editions".to_string()),
    );

    for (label, path) in [
        ("cardsfolder", &cardsfolder),
        ("archive", &archive_path),
        ("editions", &editions_dir),
    ] {
        if !path.exists() {
            eprintln!("missing {label}: {}", path.display());
            std::process::exit(1);
        }
    }

    println!(
        "Loading via load_from_directory({}) …",
        cardsfolder.display()
    );
    let t = Instant::now();
    let (db_fs, fs_result) = CardDatabase::load_from_directory(&cardsfolder);
    let fs_elapsed = t.elapsed();
    println!(
        "  → {} loaded, {} failed in {:?}",
        fs_result.loaded, fs_result.failed, fs_elapsed
    );

    println!(
        "Loading via load_from_archive({}) …",
        archive_path.display()
    );
    let t = Instant::now();
    let file = std::fs::File::open(&archive_path).expect("open archive");
    let mmap = unsafe { Mmap::map(&file).expect("mmap archive") };
    let (db_archive, arch_result) =
        CardDatabase::load_from_archive(&mmap, Some(&editions_dir)).expect("load archive");
    let arch_elapsed = t.elapsed();
    println!(
        "  → {} loaded, {} failed in {:?}",
        arch_result.loaded, arch_result.failed, arch_elapsed
    );

    println!();
    println!("─── Parity ───");
    let mut failures = 0usize;

    if db_fs.len() != db_archive.len() {
        failures += 1;
        println!(
            "✗ card count differs: fs={} archive={}",
            db_fs.len(),
            db_archive.len()
        );
    } else {
        println!("✓ card count matches: {}", db_fs.len());
    }

    if fs_result.loaded != arch_result.loaded || fs_result.failed != arch_result.failed {
        failures += 1;
        println!(
            "✗ load result differs: fs(loaded={}, failed={}) vs archive(loaded={}, failed={})",
            fs_result.loaded, fs_result.failed, arch_result.loaded, arch_result.failed
        );
    } else {
        println!(
            "✓ load result matches: loaded={}, failed={}",
            fs_result.loaded, fs_result.failed
        );
    }

    let probes = [
        "Lightning Bolt",
        "Llanowar Elves",
        "Black Lotus",
        "Counterspell",
        "Birds of Paradise",
        "Lord of the Pit",
        "Wrath of God",
    ];
    for name in probes {
        let fs_hit = db_fs.get_by_card_name(name);
        let arch_hit = db_archive.get_by_card_name(name);
        match (fs_hit, arch_hit) {
            (Some(a), Some(b)) if a.name() == b.name() => {
                println!("✓ '{name}' present in both");
            }
            (Some(a), Some(b)) => {
                failures += 1;
                println!(
                    "✗ '{name}' name mismatch: fs='{}' archive='{}'",
                    a.name(),
                    b.name()
                );
            }
            (Some(_), None) => {
                failures += 1;
                println!("✗ '{name}' present in fs, missing in archive");
            }
            (None, Some(_)) => {
                failures += 1;
                println!("✗ '{name}' missing in fs, present in archive");
            }
            (None, None) => {
                println!("∅ '{name}' missing in both (not in this cardsfolder?)");
            }
        }
    }

    // Walk every card in the FS db and confirm the archive has it under the
    // same canonical name. This is the strongest parity check.
    let mut missing: Vec<String> = Vec::new();
    for (_key, fs_card) in db_fs.iter() {
        let canonical = fs_card.name();
        if db_archive.get_by_card_name(&canonical).is_none() {
            missing.push(canonical);
        }
        if missing.len() >= 20 {
            break;
        }
    }
    if missing.is_empty() {
        println!("✓ every fs card resolvable in archive (canonical-name lookup)");
    } else {
        failures += 1;
        println!(
            "✗ {} fs cards missing from archive (showing first 20):",
            missing.len()
        );
        for name in &missing {
            println!("   - {name}");
        }
    }

    println!();
    if failures == 0 {
        println!("✓ all parity checks passed");
        println!("  fs:      {:>7.0} ms", fs_elapsed.as_secs_f64() * 1000.0);
        println!(
            "  archive: {:>7.0} ms  ({:.1}× faster)",
            arch_elapsed.as_secs_f64() * 1000.0,
            fs_elapsed.as_secs_f64() / arch_elapsed.as_secs_f64()
        );
    } else {
        println!("✗ {failures} parity check(s) failed");
        std::process::exit(1);
    }
}
