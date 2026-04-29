use std::path::PathBuf;
use std::sync::OnceLock;

use forge_foundation::edition::loader::load_editions_dir;
use forge_foundation::edition::EditionsRegistry;
use forge_foundation::sealed_product::booster_template_registry;

const BOOSTERS_SPECIAL_BODY: &str =
    include_str!("../../forge/forge-gui/res/blockdata/boosters-special.txt");

static EDITIONS: OnceLock<EditionsRegistry> = OnceLock::new();

pub fn editions() -> &'static EditionsRegistry {
    EDITIONS.get_or_init(|| {
        let mut registry = EditionsRegistry::new();

        if let Ok(dir) = std::env::var("EDITIONS_DIR") {
            let path = PathBuf::from(&dir);
            match load_editions_dir(&path, &mut registry) {
                Ok(report) => {
                    eprintln!(
                        "[limited_bootstrap] loaded {} editions from {} ({} errors)",
                        report.loaded,
                        path.display(),
                        report.errors.len()
                    );
                    for err in report.errors.iter().take(5) {
                        eprintln!("[limited_bootstrap]   {err}");
                    }
                }
                Err(e) => eprintln!("[limited_bootstrap] EDITIONS_DIR scan failed: {e}"),
            }
        } else {
            eprintln!("[limited_bootstrap] EDITIONS_DIR unset — per-set templates disabled");
        }

        let card_db = crate::card_db::get_card_db();
        registry.install_print_sheets(|code, entry| {
            let mut pc = forge_foundation::sealed_product::PaperCard::new(
                &entry.name,
                code,
                &entry.collector_number,
                entry.rarity,
            );
            if let Some(rules) = card_db.get(&entry.name) {
                pc = pc
                    .with_colors(rules.color())
                    .with_double_faced(rules.split_type.is_dual_faced());
            }
            pc
        });
        booster_template_registry::install(booster_template_registry::parse_boosters_special(
            BOOSTERS_SPECIAL_BODY,
        ));

        eprintln!(
            "[limited_bootstrap] sheet registry ready · {} edition templates known",
            registry.len()
        );
        registry
    })
}

pub fn edition_info(code: &str) -> Option<crate::limited_dto::EditionInfoDto> {
    use crate::limited_dto::{EditionInfoDto, EditionSlotDto};
    let editions = editions();
    let edition = editions.get(code)?;
    let template = edition.to_sealed_template()?;
    let slots = template
        .slots
        .iter()
        .map(|(label, count)| EditionSlotDto {
            label: label.clone(),
            count: *count,
        })
        .collect();
    let foil_type = match template.foil_type {
        forge_foundation::sealed_product::FoilType::Modern => "Modern",
        forge_foundation::sealed_product::FoilType::OldStyle => "OldStyle",
        forge_foundation::sealed_product::FoilType::NotSupported => "NotSupported",
    };
    let has_replacement_hooks = template.booster_must_contain.is_some()
        || template.booster_replace_slot_from_print_sheet.is_some()
        || template.sheet_replace_card_from_sheet.is_some()
        || template.sheet_replace_card_from_sheet2.is_some()
        || template.chance_replace_common_with > 0.0;
    Some(EditionInfoDto {
        code: edition.code.clone(),
        name: edition.name.clone(),
        edition_type: format!("{:?}", edition.edition_type),
        date: edition.date.clone(),
        slots,
        foil_chance: template.foil_chance,
        foil_type: foil_type.to_string(),
        variants: edition.variant_names(),
        has_replacement_hooks,
        booster_covers: edition.booster_covers,
        prerelease: edition.prerelease.clone(),
        alias: edition.alias.clone(),
    })
}

pub fn dominant_set_code(pool: &[forge_foundation::sealed_product::PaperCard]) -> Option<String> {
    use std::collections::HashMap;
    if pool.is_empty() {
        return None;
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    for card in pool {
        if card.set_code.is_empty() {
            continue;
        }
        *counts
            .entry(card.set_code.to_ascii_uppercase())
            .or_insert(0) += 1;
    }
    counts.into_iter().max_by_key(|(_, n)| *n).map(|(k, _)| k)
}
