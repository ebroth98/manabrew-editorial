//! Headless reproductions of Java harness/UI controller choice behavior.
//!
//! Source references:
//! - forge/forge-harness/src/main/java/forge/harness/DeterministicController.java
//! - forge/forge-harness/src/main/java/forge/harness/GuiRepro.java

use crate::choice_space;
use crate::java_random::JavaRandom;

/// Mirrors Java ChoiceSpace.pickIntInRange(min, max, rng): inclusive range.
pub fn pick_int_in_range(min: i32, max: i32, rng: &mut JavaRandom) -> i32 {
    if max <= min {
        min
    } else {
        min + rng.next_int(max - min + 1)
    }
}

/// Mirrors DeterministicController.chooseColor / chooseColorAllowColorless.
pub fn choose_color(valid_colors: &[String], rng: &mut JavaRandom) -> Option<String> {
    if valid_colors.is_empty() {
        None
    } else {
        let idx = choice_space::pick_index(valid_colors.len(), rng);
        valid_colors.get(idx).cloned()
    }
}

/// Mirrors DeterministicController.chooseSomeType.
pub fn choose_type(valid_types: &[String], rng: &mut JavaRandom) -> Option<String> {
    if valid_types.is_empty() {
        None
    } else {
        let idx = choice_space::pick_index(valid_types.len(), rng);
        valid_types.get(idx).cloned()
    }
}

/// Mirrors DeterministicController.chooseCardName(List<ICardFace>, ...).
pub fn choose_card_name(valid_names: &[String], rng: &mut JavaRandom) -> Option<String> {
    if valid_names.is_empty() {
        None
    } else {
        let idx = choice_space::pick_index(valid_names.len(), rng);
        valid_names.get(idx).cloned()
    }
}

/// Mirrors DeterministicController.chooseNumber(min, max).
pub fn choose_number(min: i32, max: i32, rng: &mut JavaRandom) -> i32 {
    pick_int_in_range(min, max, rng)
}

/// Mirrors Java ChoiceSpace.pickBool.
pub fn pick_bool(rng: &mut JavaRandom) -> bool {
    rng.next_int(2) == 1
}

/// Mirrors Java ChoiceSpace.pickCount(min, max, available, rng).
pub fn pick_count(min: usize, max: usize, available: usize, rng: &mut JavaRandom) -> usize {
    let hi = max.min(available);
    let lo = min.min(hi);
    lo + if hi > lo {
        rng.next_int((hi - lo + 1) as i32) as usize
    } else {
        0
    }
}

/// Mirrors Java ChoiceSpace.pickManyCards for generic slices.
pub fn pick_many_unique<T: Copy>(
    options: &[T],
    min: usize,
    max: usize,
    rng: &mut JavaRandom,
) -> Vec<T> {
    let mut pool = options.to_vec();
    let count = pick_count(min, max, pool.len(), rng);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if pool.is_empty() {
            break;
        }
        let idx = choice_space::pick_index(pool.len(), rng);
        out.push(pool.remove(idx));
    }
    out
}

/// Mirrors Java ChoiceSpace.shuffleCopy.
pub fn shuffle_copy<T: Copy>(options: &[T], rng: &mut JavaRandom) -> Vec<T> {
    let mut out = options.to_vec();
    let len = out.len();
    if len <= 1 {
        return out;
    }
    // Fisher-Yates with JavaRandom.
    for i in (1..len).rev() {
        let j = rng.next_int((i + 1) as i32) as usize;
        out.swap(i, j);
    }
    out
}
