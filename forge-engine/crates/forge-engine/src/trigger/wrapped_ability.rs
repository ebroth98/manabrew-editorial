use crate::game::GameState;
use crate::spellability::SpellAbility;

/// Minimal wrapped ability shim for trigger parity.
/// Full Java parity (revalidation at resolve-time) will be implemented here.
#[derive(Debug, Clone)]
pub struct WrappedAbility {
    pub wrapped: SpellAbility,
}

impl WrappedAbility {
    pub fn new(wrapped: SpellAbility) -> Self {
        Self { wrapped }
    }

    pub fn has_param(&self, key: &str) -> bool {
        self.wrapped.params.has(key)
    }

    pub fn add_cost_to_hash_list(&mut self, cost_key: &str, value: String) {
        self.wrapped
            .paid_hash
            .entry(cost_key.to_string())
            .or_default()
            .push(value);
    }

    pub fn reset_paid_hash(&mut self) {
        self.wrapped.paid_hash.clear();
    }

    pub fn has_triggering_object(&self, key: &str) -> bool {
        self.wrapped.trigger_objects.contains_key(key)
    }

    pub fn reset_triggering_objects(&mut self) {
        self.wrapped.trigger_objects.clear();
    }

    pub fn can_play(&self) -> bool {
        true
    }

    pub fn copy(&self) -> Self {
        self.clone()
    }

    pub fn yield_key(&self) -> String {
        if !self.wrapped.stack_description.is_empty() {
            self.wrapped.stack_description.clone()
        } else if !self.wrapped.description.is_empty() {
            self.wrapped.description.clone()
        } else {
            self.wrapped.ability_text.clone()
        }
    }

    pub fn to_unsuppressed_string(&self) -> String {
        self.yield_key()
    }

    pub fn has_s_var(&self, game: &GameState, key: &str) -> bool {
        self.wrapped
            .source
            .map(|cid| game.card(cid).svars.contains_key(key))
            .unwrap_or(false)
    }

    pub fn reset_once_resolved(&mut self) {
        // Placeholder for Java parity; Rust currently tracks resolve state elsewhere.
    }

    pub fn uses_targeting(&self) -> bool {
        self.wrapped.uses_targeting()
    }

    pub fn has_additional_ability(&self, key: &str) -> bool {
        self.wrapped.params.has(key)
    }

    pub fn reset_targets(&mut self) {
        self.wrapped.clear_targets();
    }

    pub fn resolve(&self) -> bool {
        true
    }
}
