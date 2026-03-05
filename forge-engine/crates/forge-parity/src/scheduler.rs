//! Game job generation for continuous parity testing.
//!
//! The scheduler cycles through:
//! 1. Round-robin preset deck pairs with incrementing seeds
//! 2. Fuzz-generated random decks from the card pool

use crate::card_pool::CardPool;
use crate::deck_generator;
use crate::java_random::JavaRandom;
use forge_carddb::CardDatabase;

/// A single game job to execute.
#[derive(Debug, Clone)]
pub struct Job {
    pub deck1: String,
    pub deck2: String,
    pub seed: u64,
    pub batch_id: i64,
}

/// Generates game jobs in a round-robin pattern across preset deck pairs,
/// optionally interleaving fuzz-generated random decks.
pub struct Scheduler {
    /// All deck pair combinations (d1, d2) where d1 != d2.
    preset_pairs: Vec<(String, String)>,
    /// Current index into preset_pairs.
    pair_index: usize,
    /// Current seed counter (incremented per game).
    seed: u64,
    /// Current batch ID (incremented per full cycle).
    batch_id: i64,
    /// Number of fuzz games to generate after each preset batch.
    fuzz_per_batch: usize,
    /// How many fuzz games remain in the current fuzz phase.
    fuzz_remaining: usize,
    /// Master RNG for fuzz deck generation.
    fuzz_rng: JavaRandom,
    /// Card pool for fuzz generation (None if fuzz disabled).
    pool: Option<CardPool>,
    /// Whether we're in the fuzz phase of the current batch.
    in_fuzz_phase: bool,
}

impl Scheduler {
    /// Create a new scheduler from a list of preset deck names.
    ///
    /// - `decks`: Available preset deck names.
    /// - `start_seed`: Initial seed counter.
    /// - `fuzz_per_batch`: Number of fuzz games per batch (0 to disable fuzz).
    /// - `db`: Card database (needed for fuzz pool discovery; None if fuzz disabled).
    /// - `include_self_matchups`: If true, include d1==d2 pairs.
    /// - `games_per_matchup`: How many games to play per pair per batch (repeats with different seeds).
    pub fn new(
        decks: &[String],
        start_seed: u64,
        fuzz_per_batch: usize,
        db: Option<&CardDatabase>,
        include_self_matchups: bool,
        games_per_matchup: usize,
    ) -> Self {
        let mut preset_pairs = Vec::new();
        let games_per = games_per_matchup.max(1);
        for (i, d1) in decks.iter().enumerate() {
            for (j, d2) in decks.iter().enumerate() {
                if i != j || include_self_matchups {
                    for _ in 0..games_per {
                        preset_pairs.push((d1.clone(), d2.clone()));
                    }
                }
            }
        }

        // If no preset decks, add a placeholder so the scheduler doesn't stall
        if preset_pairs.is_empty() {
            preset_pairs.push(("red_burn".into(), "green_stompy".into()));
        }

        let pool = if fuzz_per_batch > 0 {
            db.map(|db| {
                let (pool, _stats) = CardPool::discover(db);
                pool
            })
        } else {
            None
        };

        Self {
            preset_pairs,
            pair_index: 0,
            seed: start_seed,
            batch_id: 1,
            fuzz_per_batch,
            fuzz_remaining: 0,
            fuzz_rng: JavaRandom::new(start_seed as i64),
            pool,
            in_fuzz_phase: false,
        }
    }

    /// Generate the next game job.
    pub fn next_job(&mut self) -> Job {
        if self.in_fuzz_phase && self.fuzz_remaining > 0 {
            return self.next_fuzz_job();
        }

        // Preset phase
        let (d1, d2) = self.preset_pairs[self.pair_index].clone();
        let job = Job {
            deck1: d1,
            deck2: d2,
            seed: self.seed,
            batch_id: self.batch_id,
        };

        self.seed += 1;
        self.pair_index += 1;

        // Check if we've completed a full cycle of preset pairs
        if self.pair_index >= self.preset_pairs.len() {
            self.pair_index = 0;

            if self.fuzz_per_batch > 0 && self.pool.is_some() {
                self.in_fuzz_phase = true;
                self.fuzz_remaining = self.fuzz_per_batch;
            } else {
                self.batch_id += 1;
            }
        }

        job
    }

    fn next_fuzz_job(&mut self) -> Job {
        let pool = self.pool.as_ref().expect("fuzz pool must exist");
        let deck1 = deck_generator::generate_deck(&mut self.fuzz_rng, pool);
        let deck2 = deck_generator::generate_deck(&mut self.fuzz_rng, pool);

        let deck1_spec = format_inline_deck(&deck1);
        let deck2_spec = format_inline_deck(&deck2);

        let job = Job {
            deck1: format!("inline:{deck1_spec}"),
            deck2: format!("inline:{deck2_spec}"),
            seed: self.seed,
            batch_id: self.batch_id,
        };

        self.seed += 1;
        self.fuzz_remaining -= 1;

        if self.fuzz_remaining == 0 {
            self.in_fuzz_phase = false;
            self.batch_id += 1;
        }

        job
    }

    /// Number of preset deck pairs per batch.
    pub fn preset_pairs_count(&self) -> usize {
        self.preset_pairs.len()
    }

    /// Current batch ID.
    pub fn current_batch(&self) -> i64 {
        self.batch_id
    }
}

/// Format a deck as inline spec: "Name*Count|Name*Count|..."
fn format_inline_deck(deck: &[(String, usize)]) -> String {
    deck.iter()
        .map(|(name, count)| format!("{}*{}", name, count))
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin_cycling() {
        let decks = vec!["a".into(), "b".into(), "c".into()];
        let mut sched = Scheduler::new(&decks, 100, 0, None, false, 1);

        // 3 decks → 6 pairs (a-b, a-c, b-a, b-c, c-a, c-b)
        assert_eq!(sched.preset_pairs_count(), 6);

        let mut jobs = Vec::new();
        for _ in 0..12 {
            jobs.push(sched.next_job());
        }

        // First 6 should be batch 1, next 6 should be batch 2
        assert!(jobs[..6].iter().all(|j| j.batch_id == 1));
        assert!(jobs[6..12].iter().all(|j| j.batch_id == 2));

        // Seeds should be incrementing
        for (i, job) in jobs.iter().enumerate() {
            assert_eq!(job.seed, 100 + i as u64);
        }
    }

    #[test]
    fn self_matchups_included() {
        let decks = vec!["a".into(), "b".into()];
        let sched = Scheduler::new(&decks, 100, 0, None, true, 1);
        // 2 decks with self-matchups → 4 pairs (a-a, a-b, b-a, b-b)
        assert_eq!(sched.preset_pairs_count(), 4);
    }

    #[test]
    fn games_per_matchup_repeats() {
        let decks = vec!["a".into(), "b".into()];
        let mut sched = Scheduler::new(&decks, 100, 0, None, false, 3);
        // 2 decks, no self → 2 pairs, 3 games each → 6 jobs per batch
        assert_eq!(sched.preset_pairs_count(), 6);

        let mut jobs = Vec::new();
        for _ in 0..6 {
            jobs.push(sched.next_job());
        }
        // All should be batch 1
        assert!(jobs.iter().all(|j| j.batch_id == 1));
        // Seeds should increment
        for (i, job) in jobs.iter().enumerate() {
            assert_eq!(job.seed, 100 + i as u64);
        }
    }

    #[test]
    fn format_inline_deck_spec() {
        let deck = vec![
            ("Mountain".into(), 20),
            ("Lightning Bolt".into(), 4),
        ];
        let spec = format_inline_deck(&deck);
        assert_eq!(spec, "Mountain*20|Lightning Bolt*4");
    }
}
