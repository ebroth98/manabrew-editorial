use std::path::PathBuf;
use std::time::Instant;

use forge_cardset_archive::build_archive_from_dir;

fn main() {
    let mut args = std::env::args().skip(1);
    let cards_dir = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "forge/forge-gui/res/cardsfolder".to_string()),
    );
    let out_path = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "src-tauri/resources/cardset.rkyv".to_string()),
    );

    let started = Instant::now();
    match build_archive_from_dir(&cards_dir, &out_path) {
        Ok(stats) => {
            println!(
                "wrote {} ({} cards, {:.2} MiB, {} skipped, {} duplicates) in {:?}",
                out_path.display(),
                stats.cards,
                stats.bytes_written as f64 / 1024.0 / 1024.0,
                stats.skipped,
                stats.duplicates,
                started.elapsed()
            );
        }
        Err(err) => {
            eprintln!("build failed: {err}");
            std::process::exit(1);
        }
    }
}
