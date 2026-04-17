use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

use crate::card::Card;
use crate::ids::CardId;
use crate::spellability::SpellAbility;
use crate::staticability::parse_static_ability;

use super::{resolve_defined_player, resolve_defined_players, EffectContext};
use crate::parsing::keys;

/// Mirrors Java's `EffectEffect` for static-ability effect cards in command.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let Some(source_id) = sa.source else {
        return;
    };

    let (host_name, host_svars, host_remembered_cards, host_remembered_players) = {
        let host = ctx.game.card(source_id);
        (
            host.card_name.clone(),
            host.svars.clone(),
            host.remembered_cards.clone(),
            host.remembered_players.clone(),
        )
    };
    let static_refs: Vec<String> = sa
        .params
        .get(keys::STATIC_ABILITIES)
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    if static_refs.is_empty() {
        return;
    }

    let duration = sa.params.get(keys::DURATION);
    let effect_name = sa
        .params
        .get_cloned(keys::NAME)
        .unwrap_or_else(|| format!("{} Effect", host_name));
    let effect_owner_defined = sa.params.get(keys::EFFECT_OWNER).unwrap_or("You");

    let parsed_static_abilities = static_refs
        .iter()
        .filter_map(|svar_name| host_svars.get(svar_name))
        .filter_map(|raw| parse_static_ability(&format!("S$ {}", raw)))
        .collect::<Vec<_>>();

    if parsed_static_abilities.is_empty() {
        return;
    }

    let owners = resolve_defined_players(effect_owner_defined, sa.activating_player, ctx.game);
    for owner in owners {
        let mut effect = Card::new(
            CardId(0),
            effect_name.clone(),
            owner,
            CardTypeLine::parse("Effect"),
            ManaCost::parse("0"),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        effect.set_controller(owner);
        effect.set_effect_source(Some(source_id));
        effect.set_static_abilities(parsed_static_abilities.clone());
        apply_duration_flags(&mut effect, duration, source_id);
        apply_forget_on_moved_flags(&mut effect, sa);
        apply_remembered(
            &mut effect,
            sa,
            ctx,
            &host_remembered_cards,
            &host_remembered_players,
        );

        let effect_id = ctx.game.create_card(effect);
        ctx.move_card(effect_id, ZoneType::Command, owner);
    }
}

fn apply_duration_flags(effect: &mut Card, duration: Option<&str>, source_id: CardId) {
    match duration {
        Some("Permanent") => {}
        Some("UntilHostLeavesPlay") => {
            effect.set_temp_effect_host(Some(source_id));
        }
        Some("UntilHostLeavesPlayOrEOT") => {
            effect.set_temp_effect_host(Some(source_id));
            effect.set_temp_effect_until_eot(true);
        }
        _ => {
            // Java default for EffectEffect: expire at end of turn.
            effect.set_temp_effect_until_eot(true);
        }
    }
}

fn apply_forget_on_moved_flags(effect: &mut Card, sa: &SpellAbility) {
    if let Some(zone) = sa
        .params
        .get(keys::FORGET_ON_MOVED)
        .and_then(|z| parse_zone_name(z))
    {
        effect.set_forget_on_moved_origin(Some(zone));
        // Java forget flow exiles effect when no remembered objects remain.
        effect.set_exile_when_no_remembered(true);
    }
}

fn parse_zone_name(s: &str) -> Option<ZoneType> {
    match s.trim() {
        "Battlefield" => Some(ZoneType::Battlefield),
        "Graveyard" => Some(ZoneType::Graveyard),
        "Hand" => Some(ZoneType::Hand),
        "Library" => Some(ZoneType::Library),
        "Exile" => Some(ZoneType::Exile),
        "Stack" => Some(ZoneType::Stack),
        "Command" => Some(ZoneType::Command),
        _ => None,
    }
}

fn apply_remembered(
    effect: &mut Card,
    sa: &SpellAbility,
    ctx: &EffectContext,
    host_remembered_cards: &[CardId],
    host_remembered_players: &[crate::ids::PlayerId],
) {
    let Some(remember) = sa.params.get(keys::REMEMBER_OBJECTS) else {
        return;
    };
    for token in remember.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match token {
            // Copy the host card's remembered state.
            "Remembered" => {
                effect.add_remembered_cards(host_remembered_cards.iter().copied());
                effect.add_remembered_players(host_remembered_players.iter().copied());
            }
            // Remember targeted card (or parent targeted card for sub-abilities).
            "Targeted" => {
                if let Some(cid) = sa.target_chosen.target_card.or(ctx.parent_target_card) {
                    effect.add_remembered_card(cid);
                }
            }
            // Remember targeted player.
            "TargetedPlayer" => {
                if let Some(pid) = sa.target_chosen.target_player {
                    effect.add_remembered_player(pid);
                }
            }
            // Basic Defined$ fallback for common player references.
            other => {
                if let Some(pid) = resolve_defined_player(other, sa.activating_player, ctx.game) {
                    effect.add_remembered_player(pid);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::effects::EffectContext;
    use crate::agent::{PassAgent, PlayerAgent};
    use crate::card::Card;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;

    #[test]
    fn effect_adds_command_static_for_cast_with_flash() {
        let mut game = GameState::new(&["P0", "P1"], 20);
        let p0 = PlayerId(0);

        let mut host = Card::new(
            CardId(0),
            "Winding Canyons".to_string(),
            p0,
            CardTypeLine::parse("Land"),
            ManaCost::parse("0"),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        host.set_s_var(
            "GiveFlash",
            "Mode$ CastWithFlash | ValidCard$ Creature | ValidSA$ Spell | Caster$ You",
        );
        let host_id = game.create_card(host);
        game.move_card(host_id, ZoneType::Battlefield, p0);

        let mut sa = SpellAbility::new_simple(
            Some(host_id),
            p0,
            "AB$ Effect | StaticAbilities$ GiveFlash | SpellDescription$ test",
        );
        sa.is_activated = true;

        let mut agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut trigger_handler = TriggerHandler::new();
        let token_templates = std::collections::HashMap::new();
        let templates_variants: std::collections::HashMap<(String, String), usize> =
            std::collections::HashMap::new();
        let token_fallback: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let edition_dates: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut mana_pools = vec![ManaPool::new(), ManaPool::new()];
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            combat: None,
            agents: &mut agents,
            trigger_handler: &mut trigger_handler,
            token_templates: &token_templates,
            token_art_variants: &templates_variants,
            token_fallback: &token_fallback,
            edition_dates: &edition_dates,
            mana_pools: &mut mana_pools,
            parent_target_card: None,
            rng: &mut rng_adapter,
        };
        resolve(&mut ctx, &sa);

        let command_cards = ctx.game.cards_in_zone(ZoneType::Command, p0).to_vec();
        assert_eq!(command_cards.len(), 1);
        let effect = ctx.game.card(command_cards[0]);
        assert!(effect.temp_effect_until_eot);
        assert_eq!(effect.effect_source, Some(host_id));

        let mut spell_abilities = Vec::new();
        spell_abilities.push("SP$ Permanent | Cost$ 1 G".to_string());
        let fake_creature = Card::new(
            CardId(999),
            "Bear".to_string(),
            p0,
            CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        assert!(
            crate::staticability::static_ability_cast_with_flash::any_with_flash(
                &ctx.game.cards,
                &fake_creature,
                p0,
                &spell_abilities
            )
        );
    }
}
