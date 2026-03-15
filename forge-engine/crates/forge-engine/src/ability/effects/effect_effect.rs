use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

use crate::card::CardInstance;
use crate::ids::CardId;
use crate::spellability::SpellAbility;
use crate::staticability::parse_static_ability;

use super::{resolve_defined_player, resolve_defined_players, EffectContext};

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
        .get("StaticAbilities")
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

    let duration = sa.params.get("Duration").map(String::as_str);
    let effect_name = sa
        .params
        .get("Name")
        .cloned()
        .unwrap_or_else(|| format!("{} Effect", host_name));
    let effect_owner_defined = sa
        .params
        .get("EffectOwner")
        .map(String::as_str)
        .unwrap_or("You");

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
        let mut effect = CardInstance::new(
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
        effect.controller = owner;
        effect.effect_source = Some(source_id);
        effect.static_abilities = parsed_static_abilities.clone();
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
        ctx.game.move_card(effect_id, ZoneType::Command, owner);
    }
}

fn apply_duration_flags(effect: &mut CardInstance, duration: Option<&str>, source_id: CardId) {
    match duration {
        Some("Permanent") => {}
        Some("UntilHostLeavesPlay") => {
            effect.temp_effect_host = Some(source_id);
        }
        Some("UntilHostLeavesPlayOrEOT") => {
            effect.temp_effect_host = Some(source_id);
            effect.temp_effect_until_eot = true;
        }
        _ => {
            // Java default for EffectEffect: expire at end of turn.
            effect.temp_effect_until_eot = true;
        }
    }
}

fn apply_forget_on_moved_flags(effect: &mut CardInstance, sa: &SpellAbility) {
    if let Some(zone) = sa
        .params
        .get("ForgetOnMoved")
        .and_then(|z| parse_zone_name(z))
    {
        effect.forget_on_moved_origin = Some(zone);
        // Java forget flow exiles effect when no remembered objects remain.
        effect.exile_when_no_remembered = true;
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
    effect: &mut CardInstance,
    sa: &SpellAbility,
    ctx: &EffectContext,
    host_remembered_cards: &[CardId],
    host_remembered_players: &[crate::ids::PlayerId],
) {
    let Some(remember) = sa.params.get("RememberObjects").map(String::as_str) else {
        return;
    };
    for token in remember.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match token {
            // Copy the host card's remembered state.
            "Remembered" => {
                effect
                    .remembered_cards
                    .extend(host_remembered_cards.iter().copied());
                effect
                    .remembered_players
                    .extend(host_remembered_players.iter().copied());
            }
            // Remember targeted card (or parent targeted card for sub-abilities).
            "Targeted" => {
                if let Some(cid) = sa.target_chosen.target_card.or(ctx.parent_target_card) {
                    effect.remembered_cards.push(cid);
                }
            }
            // Remember targeted player.
            "TargetedPlayer" => {
                if let Some(pid) = sa.target_chosen.target_player {
                    effect.remembered_players.push(pid);
                }
            }
            // Basic Defined$ fallback for common player references.
            other => {
                if let Some(pid) = resolve_defined_player(other, sa.activating_player, ctx.game) {
                    effect.remembered_players.push(pid);
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
    use crate::card::CardInstance;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;

    #[test]
    fn effect_adds_command_static_for_cast_with_flash() {
        let mut game = GameState::new(&["P0", "P1"], 20);
        let p0 = PlayerId(0);

        let mut host = CardInstance::new(
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
        host.svars.insert(
            "GiveFlash".to_string(),
            "Mode$ CastWithFlash | ValidCard$ Creature | ValidSA$ Spell | Caster$ You".to_string(),
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
        let mut mana_pools = vec![ManaPool::new(), ManaPool::new()];
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            agents: &mut agents,
            trigger_handler: &mut trigger_handler,
            token_templates: &token_templates,
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
        let fake_creature = CardInstance::new(
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
