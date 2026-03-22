//! Card utility helpers (Java parity subset: `CardUtil`).

use std::collections::HashSet;

use crate::spellability::SpellAbility;

/// Java parity: accumulate colors/types a mana ability can produce.
pub fn can_produce(
    max_choices: usize,
    sa: Option<&SpellAbility>,
    mut colors: HashSet<String>,
) -> HashSet<String> {
    let Some(sa) = sa else {
        return colors;
    };
    let Some(produced) = sa.produced() else {
        return colors;
    };

    let produced_upper = produced.to_ascii_uppercase();
    if produced_upper.contains("ANY") {
        colors.insert("White".to_string());
        colors.insert("Blue".to_string());
        colors.insert("Black".to_string());
        colors.insert("Red".to_string());
        colors.insert("Green".to_string());
        if max_choices == 6 {
            colors.insert("Colorless".to_string());
        }
        return colors;
    }

    if produced_upper.contains('W') {
        colors.insert("White".to_string());
    }
    if produced_upper.contains('U') {
        colors.insert("Blue".to_string());
    }
    if produced_upper.contains('B') {
        colors.insert("Black".to_string());
    }
    if produced_upper.contains('R') {
        colors.insert("Red".to_string());
    }
    if produced_upper.contains('G') {
        colors.insert("Green".to_string());
    }
    if max_choices == 6 && produced_upper.contains('C') {
        colors.insert("Colorless".to_string());
    }
    colors
}
