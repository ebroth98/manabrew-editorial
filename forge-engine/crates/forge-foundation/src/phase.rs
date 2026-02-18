use serde::{Deserialize, Serialize};

/// Turn phases/steps. Mirrors Java `PhaseType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhaseType {
    Untap,
    Upkeep,
    Draw,
    Main1,
    CombatBegin,
    CombatDeclareAttackers,
    CombatDeclareBlockers,
    CombatFirstStrikeDamage,
    CombatDamage,
    CombatEnd,
    Main2,
    EndOfTurn,
    Cleanup,
}

impl PhaseType {
    /// The full turn sequence in order.
    pub const TURN_ORDER: [PhaseType; 13] = [
        PhaseType::Untap,
        PhaseType::Upkeep,
        PhaseType::Draw,
        PhaseType::Main1,
        PhaseType::CombatBegin,
        PhaseType::CombatDeclareAttackers,
        PhaseType::CombatDeclareBlockers,
        PhaseType::CombatFirstStrikeDamage,
        PhaseType::CombatDamage,
        PhaseType::CombatEnd,
        PhaseType::Main2,
        PhaseType::EndOfTurn,
        PhaseType::Cleanup,
    ];

    /// Phase groups for grouping related steps.
    pub const BEGINNING_PHASE: [PhaseType; 3] =
        [PhaseType::Untap, PhaseType::Upkeep, PhaseType::Draw];

    pub const COMBAT_PHASE: [PhaseType; 6] = [
        PhaseType::CombatBegin,
        PhaseType::CombatDeclareAttackers,
        PhaseType::CombatDeclareBlockers,
        PhaseType::CombatFirstStrikeDamage,
        PhaseType::CombatDamage,
        PhaseType::CombatEnd,
    ];

    pub fn is_main(self) -> bool {
        matches!(self, PhaseType::Main1 | PhaseType::Main2)
    }

    pub fn is_combat(self) -> bool {
        matches!(
            self,
            PhaseType::CombatBegin
                | PhaseType::CombatDeclareAttackers
                | PhaseType::CombatDeclareBlockers
                | PhaseType::CombatFirstStrikeDamage
                | PhaseType::CombatDamage
                | PhaseType::CombatEnd
        )
    }

    /// Index in the turn order (0-12).
    pub fn index(self) -> usize {
        Self::TURN_ORDER
            .iter()
            .position(|&p| p == self)
            .unwrap()
    }

    /// Get the next phase in the turn sequence. Wraps from Cleanup -> Untap.
    pub fn next(self) -> PhaseType {
        let idx = self.index();
        Self::TURN_ORDER[(idx + 1) % Self::TURN_ORDER.len()]
    }

    pub fn is_before(self, other: PhaseType) -> bool {
        self.index() < other.index()
    }

    pub fn is_after(self, other: PhaseType) -> bool {
        self.index() > other.index()
    }

    /// Script-compatible name used in card definition files.
    pub fn script_name(self) -> &'static str {
        match self {
            PhaseType::Untap => "Untap",
            PhaseType::Upkeep => "Upkeep",
            PhaseType::Draw => "Draw",
            PhaseType::Main1 => "Main1",
            PhaseType::CombatBegin => "BeginCombat",
            PhaseType::CombatDeclareAttackers => "Declare Attackers",
            PhaseType::CombatDeclareBlockers => "Declare Blockers",
            PhaseType::CombatFirstStrikeDamage => "First Strike Damage",
            PhaseType::CombatDamage => "Combat Damage",
            PhaseType::CombatEnd => "EndCombat",
            PhaseType::Main2 => "Main2",
            PhaseType::EndOfTurn => "End of Turn",
            PhaseType::Cleanup => "Cleanup",
        }
    }

    pub fn from_script_name(s: &str) -> Option<Self> {
        let s = s.trim();
        for &phase in &Self::TURN_ORDER {
            if phase.script_name().eq_ignore_ascii_case(s)
                || format!("{:?}", phase).eq_ignore_ascii_case(s)
            {
                return Some(phase);
            }
        }
        // "Main" matches both main phases — return Main1 as default
        if s.eq_ignore_ascii_case("Main") {
            return Some(PhaseType::Main1);
        }
        None
    }
}

impl std::fmt::Display for PhaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.script_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_order() {
        assert_eq!(PhaseType::Untap.next(), PhaseType::Upkeep);
        assert_eq!(PhaseType::Cleanup.next(), PhaseType::Untap);
    }

    #[test]
    fn is_before_after() {
        assert!(PhaseType::Untap.is_before(PhaseType::Draw));
        assert!(PhaseType::Main2.is_after(PhaseType::Main1));
    }

    #[test]
    fn script_names() {
        assert_eq!(
            PhaseType::from_script_name("BeginCombat"),
            Some(PhaseType::CombatBegin)
        );
        assert_eq!(
            PhaseType::from_script_name("End of Turn"),
            Some(PhaseType::EndOfTurn)
        );
    }
}
