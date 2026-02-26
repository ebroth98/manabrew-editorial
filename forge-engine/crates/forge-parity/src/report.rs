//! Report generation: JSON and human-readable text summaries of parity results.

use crate::protocol::{Divergence, GameTrace, MatchupStatus, MatrixReport, ParityReport};

/// Build a parity report from a Rust trace and a set of divergences.
pub fn build_report(trace: &GameTrace, divergences: Vec<Divergence>) -> ParityReport {
    let passed = divergences.is_empty();
    ParityReport {
        seed: trace.seed,
        deck1: trace.deck1.clone(),
        deck2: trace.deck2.clone(),
        total_snapshots: trace.snapshots.len(),
        divergences,
        passed,
    }
}

/// Format a parity report as a JSON string.
pub fn format_json(report: &ParityReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Format a parity report as human-readable text.
pub fn format_text(report: &ParityReport) -> String {
    let mut out = String::new();

    out.push_str("=== Forge Parity Report ===\n\n");
    out.push_str(&format!("Seed:       {}\n", report.seed));
    out.push_str(&format!("Deck 1:     {}\n", report.deck1));
    out.push_str(&format!("Deck 2:     {}\n", report.deck2));
    out.push_str(&format!("Snapshots:  {}\n", report.total_snapshots));
    out.push_str(&format!(
        "Result:     {}\n\n",
        if report.passed { "PASS" } else { "FAIL" }
    ));

    if report.divergences.is_empty() {
        out.push_str("No divergences found. Engines agree on all state.\n");
    } else {
        out.push_str(&format!(
            "Found {} divergence(s):\n\n",
            report.divergences.len()
        ));
        for (i, div) in report.divergences.iter().enumerate() {
            out.push_str(&format!(
                "  {}. [T{} {}] {}\n     Rust: {}\n     Java: {}\n\n",
                i + 1,
                div.turn,
                div.phase,
                div.field,
                div.rust_value,
                div.java_value,
            ));
        }
    }

    out
}

/// Format a matrix report as a JSON string.
pub fn format_matrix_json(report: &MatrixReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Format a matrix report as human-readable text.
pub fn format_matrix_text(report: &MatrixReport) -> String {
    let mut out = String::new();

    out.push_str("=== Forge Parity Matrix Report ===\n\n");
    out.push_str(&format!(
        "Seeds:      {}\n",
        report
            .seeds
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str(&format!("Decks:      {}\n", report.decks.join(", ")));
    out.push_str(&format!("Max turns:  {}\n\n", report.max_turns));

    out.push_str(&format!(
        "Results: {} matchups | {} PASS | {} FAIL | {} ERROR\n\n",
        report.total_matchups, report.passed, report.failed, report.errors
    ));

    // Column header
    out.push_str(&format!(
        "  {:<20} {:<20} {:<6} {:<7} {}\n",
        "Deck1", "Deck2", "Seed", "Status", "Divergences"
    ));
    out.push_str(&format!("  {}\n", "-".repeat(75)));

    for r in &report.results {
        let status_str = match r.status {
            MatchupStatus::Pass => "PASS",
            MatchupStatus::Fail => "FAIL",
            MatchupStatus::Error => "ERROR",
        };
        out.push_str(&format!(
            "  {:<20} {:<20} {:<6} {:<7} {}\n",
            r.deck1, r.deck2, r.seed, status_str, r.divergence_count
        ));
    }

    // Print failure details
    let failures: Vec<_> = report
        .results
        .iter()
        .filter(|r| r.status != MatchupStatus::Pass)
        .collect();
    if !failures.is_empty() {
        out.push_str(&format!("\nFailures:\n"));
        for (i, r) in failures.iter().enumerate() {
            match &r.error_message {
                Some(msg) => {
                    out.push_str(&format!(
                        "  {}. {} vs {} | seed={}\n     Error: {}\n\n",
                        i + 1,
                        r.deck1,
                        r.deck2,
                        r.seed,
                        msg
                    ));
                }
                None => {
                    if let Some(ref div) = r.first_divergence {
                        out.push_str(&format!(
                            "  {}. {} vs {} | seed={}\n     [T{} {}] {}: Rust={} Java={}\n\n",
                            i + 1,
                            r.deck1,
                            r.deck2,
                            r.seed,
                            div.turn,
                            div.phase,
                            div.field,
                            div.rust_value,
                            div.java_value,
                        ));
                    } else {
                        out.push_str(&format!(
                            "  {}. {} vs {} | seed={}\n     {} divergence(s)\n\n",
                            i + 1,
                            r.deck1,
                            r.deck2,
                            r.seed,
                            r.divergence_count,
                        ));
                    }
                }
            }
        }
    }

    out
}

/// Format a game trace as human-readable text (Rust-only mode).
pub fn format_trace_text(trace: &GameTrace) -> String {
    let mut out = String::new();

    out.push_str("=== Forge Parity Trace (Rust-only) ===\n\n");
    out.push_str(&format!("Seed:       {}\n", trace.seed));
    out.push_str(&format!("Deck 1:     {}\n", trace.deck1));
    out.push_str(&format!("Deck 2:     {}\n", trace.deck2));
    out.push_str(&format!("Max turns:  {}\n", trace.max_turns));
    out.push_str(&format!("Snapshots:  {}\n\n", trace.snapshots.len()));

    for (i, snap) in trace.snapshots.iter().enumerate() {
        out.push_str(&format!(
            "--- Snapshot {} | Turn {} | {} | Active: P{} ---\n",
            i, snap.turn, snap.phase, snap.active_player,
        ));

        if snap.game_over {
            out.push_str(&format!(
                "  GAME OVER — winner: {}\n",
                snap.winner
                    .map(|w| format!("P{}", w))
                    .unwrap_or_else(|| "draw".into()),
            ));
        }

        for player in &snap.players {
            out.push_str(&format!(
                "  P{} {} — Life:{} Poison:{} Lost:{} Won:{}\n",
                player.index,
                player.name,
                player.life,
                player.poison,
                player.has_lost,
                player.has_won,
            ));
            if !player.battlefield.is_empty() {
                out.push_str("    Battlefield: ");
                let cards: Vec<String> = player
                    .battlefield
                    .iter()
                    .map(|c| {
                        let mut s = c.name.clone();
                        if c.tapped {
                            s.push_str(" (T)");
                        }
                        if let (Some(p), Some(t)) = (c.power, c.toughness) {
                            s.push_str(&format!(" {}/{}", p, t));
                        }
                        s
                    })
                    .collect();
                out.push_str(&cards.join(", "));
                out.push('\n');
            }
            if !player.hand.is_empty() {
                out.push_str(&format!(
                    "    Hand ({}): {}\n",
                    player.hand.len(),
                    player.hand.join(", ")
                ));
            }
            if !player.graveyard.is_empty() {
                out.push_str(&format!("    Graveyard: {}\n", player.graveyard.join(", ")));
            }
            out.push_str(&format!("    Library: {} cards\n", player.library_size));
        }

        if !snap.stack.is_empty() {
            out.push_str(&format!("  Stack: {}\n", snap.stack.join(", ")));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::StateSnapshot;

    #[test]
    fn pass_report_text() {
        let report = ParityReport {
            seed: 42,
            deck1: "red_burn".into(),
            deck2: "green_stompy".into(),
            total_snapshots: 10,
            divergences: vec![],
            passed: true,
        };
        let text = format_text(&report);
        assert!(text.contains("PASS"));
        assert!(text.contains("No divergences"));
    }

    #[test]
    fn fail_report_text() {
        let report = ParityReport {
            seed: 42,
            deck1: "red_burn".into(),
            deck2: "green_stompy".into(),
            total_snapshots: 10,
            divergences: vec![Divergence {
                snapshot_index: 3,
                turn: 2,
                phase: "Main1".into(),
                field: "players[0].life".into(),
                rust_value: "18".into(),
                java_value: "20".into(),
            }],
            passed: false,
        };
        let text = format_text(&report);
        assert!(text.contains("FAIL"));
        assert!(text.contains("players[0].life"));
    }

    #[test]
    fn trace_text_format() {
        let trace = GameTrace {
            seed: 42,
            deck1: "red_burn".into(),
            deck2: "green_stompy".into(),
            max_turns: 5,
            snapshots: vec![StateSnapshot {
                turn: 1,
                phase: "Untap".into(),
                active_player: 0,
                game_over: false,
                winner: None,
                players: vec![],
                stack: vec![],
            }],
        };
        let text = format_trace_text(&trace);
        assert!(text.contains("Rust-only"));
        assert!(text.contains("Snapshot 0"));
    }
}
