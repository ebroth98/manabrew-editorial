use crate::java_random::JavaRandom;
use std::cmp::Ordering;

pub fn pick_bool(rng: &mut JavaRandom) -> bool {
    pick_index(2, rng) == 1
}

pub fn pick_one<T: Copy>(options: &[T], rng: &mut JavaRandom) -> Option<T> {
    if options.is_empty() {
        return None;
    }
    let idx = pick_index(options.len(), rng);
    Some(options[idx])
}

/// Pick index in [0, size). Does not consume RNG when size <= 1.
pub fn pick_index(size: usize, rng: &mut JavaRandom) -> usize {
    if size <= 1 {
        return 0;
    }
    rng.next_int(size as i32) as usize
}

/// Pick index in [0, size] where size is PASS.
pub fn pick_index_with_pass(size: usize, rng: &mut JavaRandom) -> usize {
    rng.next_int((size + 1) as i32) as usize
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
    if roll >= safe_size.saturating_mul(safe_weight) {
        safe_size
    } else {
        roll / safe_weight
    }
}

pub fn sort_native<T: Clone>(native: &[T], mut cmp: impl FnMut(&T, &T) -> Ordering) -> Vec<T> {
    let mut out = native.to_vec();
    out.sort_by(|a, b| cmp(a, b));
    out
}

pub fn pick_count(min: usize, max: usize, available: usize, rng: &mut JavaRandom) -> usize {
    let hi = max.min(available);
    let lo = min.min(hi);
    lo + if hi > lo {
        rng.next_int((hi - lo + 1) as i32) as usize
    } else {
        0
    }
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
    out
}
