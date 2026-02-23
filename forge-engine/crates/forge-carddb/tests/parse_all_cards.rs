use forge_carddb::CardDatabase;
use std::path::Path;

#[test]
#[ignore] // Run with: cargo test --test parse_all_cards -- --ignored
fn parse_all_card_scripts() {
    let cards_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../forge/forge-gui/res/cardsfolder");

    if !cards_dir.exists() {
        eprintln!(
            "Skipping test: card scripts directory not found at {:?}",
            cards_dir
        );
        return;
    }

    let (_db, result) = CardDatabase::load_from_directory(&cards_dir);

    println!("Loaded: {} cards", result.loaded);
    println!("Failed: {} cards", result.failed);

    if !result.errors.is_empty() {
        println!("\nFirst 20 errors:");
        for (file, err) in result.errors.iter().take(20) {
            println!("  {}: {}", file, err);
        }
    }

    let failure_rate = if result.loaded + result.failed > 0 {
        result.failed as f64 / (result.loaded + result.failed) as f64 * 100.0
    } else {
        0.0
    };

    println!(
        "\nSuccess rate: {:.2}% ({}/{})",
        100.0 - failure_rate,
        result.loaded,
        result.loaded + result.failed
    );

    assert!(
        result.loaded > 30000,
        "Expected at least 30000 cards, got {}",
        result.loaded
    );
    assert_eq!(
        result.failed, 0,
        "Expected zero parse failures, got {}",
        result.failed
    );
}
