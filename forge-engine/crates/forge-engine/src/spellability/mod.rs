pub mod targeting;

use serde::{Deserialize, Serialize};

use crate::ids::{CardId, PlayerId};

/// An entry on the game stack (spell or ability waiting to resolve).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackEntry {
    pub id: u32,
    /// The card associated with this stack entry (if any).
    pub source: Option<CardId>,
    /// The player who put this on the stack.
    pub controller: PlayerId,
    /// The raw ability string to execute on resolution.
    pub ability_text: String,
    /// Whether this is a creature spell (goes to battlefield on resolve).
    pub is_creature_spell: bool,
    /// Whether this is a non-creature permanent spell.
    pub is_permanent_spell: bool,
    /// Target player (if any).
    pub target_player: Option<PlayerId>,
    /// Target card (if any).
    pub target_card: Option<CardId>,
    /// Whether this is a triggered ability (not a spell).
    pub is_triggered_ability: bool,
    /// Whether this is an activated ability (not a spell).
    pub is_activated_ability: bool,
    /// Card that owns the trigger (for intervening-if recheck).
    pub trigger_source: Option<CardId>,
    /// Index into card.triggers for intervening-if recheck.
    pub trigger_index: Option<usize>,
}

/// The game stack. Spells and abilities are added to the top and resolve LIFO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicStack {
    entries: Vec<StackEntry>,
    next_id: u32,
}

impl MagicStack {
    pub fn new() -> Self {
        MagicStack {
            entries: Vec::new(),
            next_id: 0,
        }
    }

    pub fn push(&mut self, mut entry: StackEntry) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        entry.id = id;
        self.entries.push(entry);
        id
    }

    pub fn pop(&mut self) -> Option<StackEntry> {
        self.entries.pop()
    }

    pub fn peek(&self) -> Option<&StackEntry> {
        self.entries.last()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &StackEntry> {
        self.entries.iter()
    }
}

impl Default for MagicStack {
    fn default() -> Self {
        Self::new()
    }
}
