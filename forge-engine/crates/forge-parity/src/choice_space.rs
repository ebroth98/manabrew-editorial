use crate::java_random::JavaRandom;
use crate::parity_log;
use std::cmp::Ordering;

/// Prefix used on all choice-space log entries so the display layer can
/// distinguish them from regular callback args.
const P: &str = "> ";

// ── Choice functions ──────────────────────────────────────────────

pub fn pick_bool(rng: &mut JavaRandom) -> bool {
    let idx = pick_index(2, rng);
    let result = idx == 1;
    parity_log::log(&format!("{P}pick_bool -> {result} {{{}}}", rng.call_count));
    result
}

pub fn pick_one<T: Copy>(options: &[T], rng: &mut JavaRandom) -> Option<T> {
    if options.is_empty() {
        return None;
    }
    let idx = pick_index(options.len(), rng);
    parity_log::log(&format!("{P}pick_one [{len}] -> idx={idx} {{{cc}}}", len = options.len(), cc = rng.call_count));
    Some(options[idx])
}

/// Pick index in [0, size). Does not consume RNG when size <= 1.
pub fn pick_index(size: usize, rng: &mut JavaRandom) -> usize {
    if size == 0 {
        parity_log::log(&format!("{P}pick_index [0] -> idx=0 {{{}}}", rng.call_count));
        return 0;
    }
    if size == 1 {
        parity_log::log(&format!("{P}pick_index [1] -> idx=0 {{{}}}", rng.call_count));
        return 0;
    }
    let idx = rng.next_int(size as i32) as usize;
    parity_log::log(&format!(
        "{P}pick_index [{size}] -> idx={idx} {{{cc}}}",
        cc = rng.call_count
    ));
    idx
}

/// Pick index in [0, size] where size is PASS.
pub fn pick_index_with_pass(size: usize, rng: &mut JavaRandom) -> usize {
    let idx = rng.next_int((size + 1) as i32) as usize;
    let cc = rng.call_count;
    if idx >= size {
        parity_log::log(&format!("{P}pick_index_with_pass [{size}] -> PASS {{{cc}}}"));
    } else {
        parity_log::log(&format!("{P}pick_index_with_pass [{size}] -> idx={idx} {{{cc}}}"));
    }
    idx
}

pub fn pick_weighted_index_with_pass(
    size: usize,
    action_weight: usize,
    rng: &mut JavaRandom,
) -> usize {
    let safe_size = size;
    let safe_weight = action_weight.max(1);
    let total = safe_size.saturating_mul(safe_weight).saturating_add(1);
    let roll = pick_index(total, rng);
    let idx = if roll >= safe_size.saturating_mul(safe_weight) {
        safe_size
    } else {
        roll / safe_weight
    };
    let cc = rng.call_count;
    if idx >= safe_size {
        parity_log::log(&format!("{P}pick_weighted [{safe_size}] w={safe_weight} -> PASS {{{cc}}}"));
    } else {
        parity_log::log(&format!("{P}pick_weighted [{safe_size}] w={safe_weight} -> idx={idx} {{{cc}}}"));
    }
    idx
}

pub fn sort_native<T: Clone>(native: &[T], mut cmp: impl FnMut(&T, &T) -> Ordering) -> Vec<T> {
    let mut out = native.to_vec();
    out.sort_by(|a, b| cmp(a, b));
    out
}

pub fn pick_count(min: usize, max: usize, available: usize, rng: &mut JavaRandom) -> usize {
    let hi = max.min(available);
    let lo = min.min(hi);
    let count = lo + if hi > lo {
        rng.next_int((hi - lo + 1) as i32) as usize
    } else {
        0
    };
    parity_log::log(&format!("{P}pick_count [{min}-{max}] of {available} -> {count} {{{cc}}}", cc = rng.call_count));
    count
}

pub fn pick_many_unique<T: Copy>(
    options: &[T],
    min: usize,
    max: usize,
    rng: &mut JavaRandom,
) -> Vec<T> {
    let mut pool = options.to_vec();
    let count = pick_count(min, max, pool.len(), rng);
    let mut out = Vec::new();
    for _ in 0..count {
        if pool.is_empty() {
            break;
        }
        let idx = pick_index(pool.len(), rng);
        out.push(pool.remove(idx));
    }
    let len = options.len();
    let picked = out.len();
    parity_log::log(&format!("{P}pick_many_unique [{min}-{max}] of {len} -> picked {picked} {{{cc}}}", cc = rng.call_count));
    out
}
