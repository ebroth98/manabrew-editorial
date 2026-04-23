//! Typed accessor helpers for SpellAbility parameters.
//!
//! These methods replace raw `sa.params.get("ParamName")` calls with
//! typed accessors that are discoverable via autocomplete and catch
//! typos at compile time.

use super::SpellAbility;
use crate::parsing::{keys, CompiledSelector};
use forge_foundation::ZoneType;

/// Typed accessors for common SpellAbility parameters.
/// These mirror the param keys used in Java Forge's ability text format.
impl SpellAbility {
    // ── Card/Target filters ────────────────────────────────────────────────

    /// Get the compiled `ValidCard$` filter.
    pub fn valid_card(&self) -> Option<&CompiledSelector> {
        self.params.selector(keys::VALID_CARD)
    }

    /// Get the compiled `ValidPlayer$` filter.
    pub fn valid_player(&self) -> Option<&CompiledSelector> {
        self.params.selector(keys::VALID_PLAYER)
    }

    /// Get the compiled `ValidTarget$` filter.
    pub fn valid_target(&self) -> Option<&CompiledSelector> {
        self.params.selector(keys::VALID_TARGET)
    }

    /// Get the `ChangeType$` filter string (used by zone-change effects).
    pub fn change_type(&self) -> Option<&str> {
        self.params.selector_value(keys::CHANGE_TYPE)
    }

    // ── Zone/movement params ───────────────────────────────────────────────

    /// Get the `Defined$` card reference (e.g. "Self", "Remembered", "Targeted").
    pub fn defined(&self) -> Option<&str> {
        self.params.reference_value(keys::DEFINED)
    }

    /// Get the `DefinedPlayer$` player reference (e.g. "Player", "You", "Opponent").
    pub fn defined_player(&self) -> Option<&str> {
        self.params.reference_value(keys::DEFINED_PLAYER)
    }

    /// Get the `Origin$` zone type string.
    pub fn origin(&self) -> Option<&str> {
        self.params.get(keys::ORIGIN)
    }

    /// Get the `Origin$` zone type.
    pub fn origin_zone(&self) -> Option<ZoneType> {
        self.params.zone_type(keys::ORIGIN)
    }

    /// Get the `Origin$` zone list.
    pub fn origin_zones(&self) -> Vec<ZoneType> {
        self.params.zone_types(keys::ORIGIN)
    }

    /// Get the `Destination$` zone type string.
    pub fn destination(&self) -> Option<&str> {
        self.params.get(keys::DESTINATION)
    }

    /// Get the `Destination$` zone type.
    pub fn destination_zone(&self) -> Option<ZoneType> {
        self.params.zone_type(keys::DESTINATION)
    }

    /// Get the `LibraryPosition$` string (e.g. "0", "-1", "Bottom").
    pub fn library_position(&self) -> Option<&str> {
        self.params.get(keys::LIBRARY_POSITION)
    }

    // ── Mana/cost params ───────────────────────────────────────────────────

    /// Get the `Produced$` mana type string.
    pub fn produced(&self) -> Option<&str> {
        self.params.get(keys::PRODUCED)
    }

    // ── Boolean params ─────────────────────────────────────────────────────

    /// Check if a boolean param is set to "True" (case-insensitive).
    /// This is a more discoverable alias for the existing `param_is_true`.
    pub fn is_param_true(&self, key: &str) -> bool {
        self.params.is_true(key)
    }

    /// Get `Hidden$` as boolean.
    pub fn is_hidden(&self) -> bool {
        self.params.is_true(keys::HIDDEN)
    }

    /// Get `Mandatory$` as boolean.
    pub fn is_mandatory(&self) -> bool {
        self.params.is_true(keys::MANDATORY)
    }

    /// Get `Tapped$` as boolean.
    pub fn is_tapped(&self) -> bool {
        self.params.is_true(keys::TAPPED)
    }

    /// Get `Shuffle$` as boolean.
    pub fn is_shuffle(&self) -> bool {
        self.params.is_true(keys::SHUFFLE)
    }

    /// Get `RememberChanged$` as boolean.
    pub fn is_remember_changed(&self) -> bool {
        self.params.is_true(keys::REMEMBER_CHANGED)
    }

    /// Get `Optional$` as boolean.
    pub fn is_optional(&self) -> bool {
        self.params.is_true(keys::OPTIONAL)
    }

    /// Get `GainControl$` as boolean.
    pub fn is_gain_control(&self) -> bool {
        self.params.is_true(keys::GAIN_CONTROL)
    }

    /// Get `ForgetChanged$` as boolean.
    pub fn is_forget_changed(&self) -> bool {
        self.params.is_true(keys::FORGET_CHANGED)
    }

    /// Get `Imprint$` as boolean.
    pub fn is_imprint(&self) -> bool {
        self.params.is_true(keys::IMPRINT)
    }

    /// Get `FaceDown$` as boolean.
    pub fn is_face_down(&self) -> bool {
        self.params.is_true(keys::FACE_DOWN)
    }

    /// Get `ExileFaceDown$` as boolean.
    pub fn is_exile_face_down(&self) -> bool {
        self.params.is_true(keys::EXILE_FACE_DOWN)
    }

    /// Get `Transformed$` as boolean.
    pub fn is_transformed(&self) -> bool {
        self.params.is_true(keys::TRANSFORMED)
    }

    /// Get `AtRandom$` as boolean.
    pub fn is_at_random(&self) -> bool {
        self.params.is_true(keys::AT_RANDOM)
    }

    /// Get `Reveal$` as boolean (default true for searches — NoReveal overrides).
    pub fn is_reveal(&self) -> bool {
        !self.params.is_true(keys::NO_REVEAL)
    }

    /// Get `ChangeNum$` as usize, defaulting to 1.
    pub fn change_num(&self) -> usize {
        self.params.as_usize(keys::CHANGE_NUM).unwrap_or(1)
    }

    /// Get `Chooser$` player reference.
    pub fn chooser(&self) -> Option<&str> {
        self.params.get(keys::CHOOSER)
    }

    /// Get `AttachedTo$` target definition.
    pub fn attached_to(&self) -> Option<&str> {
        self.params.get(keys::ATTACHED_TO)
    }

    /// Get `DestinationAlternative$` zone.
    pub fn destination_alternative(&self) -> Option<&str> {
        self.params.get(keys::DESTINATION_ALTERNATIVE)
    }

    /// Get `SelectPrompt$` custom prompt text.
    pub fn select_prompt(&self) -> Option<&str> {
        self.params.get(keys::SELECT_PROMPT)
    }

    // ── Numeric params ─────────────────────────────────────────────────────

    /// Get a numeric param by key, returning None if absent or non-numeric.
    pub fn param_as_i32(&self, key: &str) -> Option<i32> {
        self.params.as_i32(key)
    }

    // ── SubAbility chain ───────────────────────────────────────────────────

    /// Get the `SubAbility$` SVar name.
    pub fn sub_ability_name(&self) -> Option<&str> {
        self.params.get(keys::SUB_ABILITY)
    }

    // ── Token params ───────────────────────────────────────────────────────

    /// Get the `TokenScript$` name.
    pub fn token_script(&self) -> Option<&str> {
        self.params.get(keys::TOKEN_SCRIPT)
    }

    /// Get the `TokenOwner$` reference.
    pub fn token_owner(&self) -> Option<&str> {
        self.params.get(keys::TOKEN_OWNER)
    }

    // ── Counter params ─────────────────────────────────────────────────────

    /// Get the `WithCountersType$` string.
    pub fn with_counters_type(&self) -> Option<&str> {
        self.params.get(keys::WITH_COUNTERS_TYPE)
    }

    /// Get the `WithCountersAmount$` as i32.
    pub fn with_counters_amount(&self) -> Option<i32> {
        self.params.as_i32(keys::WITH_COUNTERS_AMOUNT)
    }
}
