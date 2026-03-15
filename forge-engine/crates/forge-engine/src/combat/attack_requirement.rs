use crate::card::CardInstance;
use crate::ids::{CardId, PlayerId};
use crate::staticability::static_ability_must_attack;

/// Represents a requirement for a creature to attack.
/// Mirrors Java's `AttackRequirement.java`.
#[derive(Debug, Clone)]
pub struct AttackRequirement {
    /// The creature that must attack.
    pub attacker: CardId,
    /// True if the creature must attack any legal defender.
    pub must_attack_any: bool,
    /// If set, the creature must attack this specific defender (if able).
    pub must_attack_defender: Option<PlayerId>,
    /// The player that goaded this creature (it can't attack that player).
    pub goaded_by: Option<PlayerId>,
}

/// Compute attack requirements for all available creatures.
/// Returns a list of requirements — creatures that must attack if able.
///
/// Sources of must-attack:
/// 1. Static abilities with `MustAttack` mode (existing `must_attack()` check)
/// 2. Goad: creature is goaded and must attack a player other than the goader
pub fn compute_attack_requirements(
    cards: &[CardInstance],
    available: &[CardId],
    defending: PlayerId,
) -> Vec<AttackRequirement> {
    let mut requirements = Vec::new();

    for &attacker_id in available {
        let card = &cards[attacker_id.index()];

        let must_from_static = static_ability_must_attack::must_attack(cards, card);
        let goaded = card.goaded_by;

        if must_from_static || goaded.is_some() {
            let goaded_by_player = goaded;
            // Goad: creature must attack if able. The "can't attack goader"
            // part only matters in multiplayer when there are multiple
            // defenders to choose from. In 2-player, the goaded creature
            // must attack the only opponent regardless of who goaded it.
            let must_attack_any = must_from_static || goaded.is_some();
            let must_attack_defender = if goaded.is_some() && goaded != Some(defending) {
                // Goaded by someone other than the defender — must attack
                // the defender (in 2-player this is always the case since
                // only the opponent can goad).
                Some(defending)
            } else {
                None
            };

            requirements.push(AttackRequirement {
                attacker: attacker_id,
                must_attack_any,
                must_attack_defender,
                goaded_by: goaded_by_player,
            });
        }
    }

    requirements
}

/// Get all creature IDs that must attack (from requirements).
pub fn must_attack_ids(requirements: &[AttackRequirement]) -> Vec<CardId> {
    requirements
        .iter()
        .filter(|r| r.must_attack_any)
        .map(|r| r.attacker)
        .collect()
}
