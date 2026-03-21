//! Typed accessor helpers for SpellAbility parameters.
//!
//! These methods replace raw `sa.params.get("ParamName")` calls with
//! typed accessors that are discoverable via autocomplete and catch
//! typos at compile time.

use super::SpellAbility;

/// Typed accessors for common SpellAbility parameters.
/// These mirror the param keys used in Java Forge's ability text format.
impl SpellAbility {
    // ── Card/Target filters ────────────────────────────────────────────────

    /// Get the `ValidCard$` filter string (e.g. "Creature.YouCtrl").
    pub fn valid_card(&self) -> Option<&str> {
        self.params.get("ValidCard").map(|s| s.as_str())
    }

    /// Get the `ValidPlayer$` filter string.
    pub fn valid_player(&self) -> Option<&str> {
        self.params.get("ValidPlayer").map(|s| s.as_str())
    }

    /// Get the `ValidTarget$` filter string.
    pub fn valid_target(&self) -> Option<&str> {
        self.params.get("ValidTarget").map(|s| s.as_str())
    }

    /// Get the `ChangeType$` filter string (used by zone-change effects).
    pub fn change_type(&self) -> Option<&str> {
        self.params.get("ChangeType").map(|s| s.as_str())
    }

    // ── Zone/movement params ───────────────────────────────────────────────

    /// Get the `Defined$` card reference (e.g. "Self", "Remembered", "Targeted").
    pub fn defined(&self) -> Option<&str> {
        self.params.get("Defined").map(|s| s.as_str())
    }

    /// Get the `DefinedPlayer$` player reference (e.g. "Player", "You", "Opponent").
    pub fn defined_player(&self) -> Option<&str> {
        self.params.get("DefinedPlayer").map(|s| s.as_str())
    }

    /// Get the `Origin$` zone type string.
    pub fn origin(&self) -> Option<&str> {
        self.params.get("Origin").map(|s| s.as_str())
    }

    /// Get the `Destination$` zone type string.
    pub fn destination(&self) -> Option<&str> {
        self.params.get("Destination").map(|s| s.as_str())
    }

    /// Get the `LibraryPosition$` string (e.g. "0", "-1", "Bottom").
    pub fn library_position(&self) -> Option<&str> {
        self.params.get("LibraryPosition").map(|s| s.as_str())
    }

    // ── Mana/cost params ───────────────────────────────────────────────────

    /// Get the `Produced$` mana type string.
    pub fn produced(&self) -> Option<&str> {
        self.params.get("Produced").map(|s| s.as_str())
    }

    // ── Boolean params ─────────────────────────────────────────────────────

    /// Check if a boolean param is set to "True" (case-insensitive).
    /// This is a more discoverable alias for the existing `param_is_true`.
    pub fn is_param_true(&self, key: &str) -> bool {
        self.params
            .get(key)
            .map_or(false, |v| v.eq_ignore_ascii_case("True"))
    }

    /// Get `Hidden$` as boolean.
    pub fn is_hidden(&self) -> bool {
        self.is_param_true("Hidden")
    }

    /// Get `Mandatory$` as boolean.
    pub fn is_mandatory(&self) -> bool {
        self.is_param_true("Mandatory")
    }

    /// Get `Tapped$` as boolean.
    pub fn is_tapped(&self) -> bool {
        self.is_param_true("Tapped")
    }

    /// Get `Shuffle$` as boolean.
    pub fn is_shuffle(&self) -> bool {
        self.is_param_true("Shuffle")
    }

    /// Get `RememberChanged$` as boolean.
    pub fn is_remember_changed(&self) -> bool {
        self.is_param_true("RememberChanged")
    }

    /// Get `Optional$` as boolean.
    pub fn is_optional(&self) -> bool {
        self.is_param_true("Optional")
    }

    /// Get `GainControl$` as boolean.
    pub fn is_gain_control(&self) -> bool {
        self.is_param_true("GainControl")
    }

    /// Get `ForgetChanged$` as boolean.
    pub fn is_forget_changed(&self) -> bool {
        self.is_param_true("ForgetChanged")
    }

    /// Get `Imprint$` as boolean.
    pub fn is_imprint(&self) -> bool {
        self.is_param_true("Imprint")
    }

    /// Get `FaceDown$` as boolean.
    pub fn is_face_down(&self) -> bool {
        self.is_param_true("FaceDown")
    }

    /// Get `ExileFaceDown$` as boolean.
    pub fn is_exile_face_down(&self) -> bool {
        self.is_param_true("ExileFaceDown")
    }

    /// Get `Transformed$` as boolean.
    pub fn is_transformed(&self) -> bool {
        self.is_param_true("Transformed")
    }

    /// Get `AtRandom$` as boolean.
    pub fn is_at_random(&self) -> bool {
        self.is_param_true("AtRandom")
    }

    /// Get `Reveal$` as boolean (default true for searches — NoReveal overrides).
    pub fn is_reveal(&self) -> bool {
        !self.is_param_true("NoReveal")
    }

    /// Get `ChangeNum$` as usize, defaulting to 1.
    pub fn change_num(&self) -> usize {
        self.params
            .get("ChangeNum")
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(1)
    }

    /// Get `Chooser$` player reference.
    pub fn chooser(&self) -> Option<&str> {
        self.params.get("Chooser").map(|s| s.as_str())
    }

    /// Get `AttachedTo$` target definition.
    pub fn attached_to(&self) -> Option<&str> {
        self.params.get("AttachedTo").map(|s| s.as_str())
    }

    /// Get `DestinationAlternative$` zone.
    pub fn destination_alternative(&self) -> Option<&str> {
        self.params.get("DestinationAlternative").map(|s| s.as_str())
    }

    /// Get `SelectPrompt$` custom prompt text.
    pub fn select_prompt(&self) -> Option<&str> {
        self.params.get("SelectPrompt").map(|s| s.as_str())
    }

    // ── Numeric params ─────────────────────────────────────────────────────

    /// Get a numeric param by key, returning None if absent or non-numeric.
    pub fn param_as_i32(&self, key: &str) -> Option<i32> {
        self.params.get(key).and_then(|v| v.trim().parse().ok())
    }

    // ── SubAbility chain ───────────────────────────────────────────────────

    /// Get the `SubAbility$` SVar name.
    pub fn sub_ability_name(&self) -> Option<&str> {
        self.params.get("SubAbility").map(|s| s.as_str())
    }

    // ── Token params ───────────────────────────────────────────────────────

    /// Get the `TokenScript$` name.
    pub fn token_script(&self) -> Option<&str> {
        self.params.get("TokenScript").map(|s| s.as_str())
    }

    /// Get the `TokenOwner$` reference.
    pub fn token_owner(&self) -> Option<&str> {
        self.params.get("TokenOwner").map(|s| s.as_str())
    }

    // ── Counter params ─────────────────────────────────────────────────────

    /// Get the `WithCountersType$` string.
    pub fn with_counters_type(&self) -> Option<&str> {
        self.params.get("WithCountersType").map(|s| s.as_str())
    }

    /// Get the `WithCountersAmount$` as i32.
    pub fn with_counters_amount(&self) -> Option<i32> {
        self.param_as_i32("WithCountersAmount")
    }
}
