use std::collections::BTreeMap;
use std::path::PathBuf;

use forge_carddb::{CardDatabase, CardFace};
use forge_engine_core::parsing::{
    parse_semantic_param_value, CompiledSelector, Params, SemanticParamValue,
};

fn main() {
    let cards_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../forge/forge-gui/res/cardsfolder")
        });
    let (db, result) = CardDatabase::load_from_directory(&cards_dir);
    println!(
        "loaded={} failed={} cards_dir={}",
        result.loaded,
        result.failed,
        cards_dir.display()
    );

    let mut raw_predicates = BTreeMap::<String, usize>::new();
    let mut valid_non_selectors = BTreeMap::<String, BTreeMap<String, usize>>::new();

    for (_key, rules) in db.iter() {
        scan_face(
            &rules.main_part,
            &mut raw_predicates,
            &mut valid_non_selectors,
        );
        if let Some(face) = &rules.other_part {
            scan_face(face, &mut raw_predicates, &mut valid_non_selectors);
        }
        for face in rules.specialized_parts.values() {
            scan_face(face, &mut raw_predicates, &mut valid_non_selectors);
        }
    }

    println!("\nraw selector predicates:");
    let mut raw_predicates = raw_predicates.into_iter().collect::<Vec<_>>();
    raw_predicates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (raw, count) in raw_predicates.iter().take(80) {
        println!("{count:6} {raw}");
    }

    println!("\nValid* params that are not selector/reference classified:");
    let mut valid_non_selectors = valid_non_selectors.into_iter().collect::<Vec<_>>();
    valid_non_selectors.sort_by(|a, b| {
        let a_total = a.1.values().sum::<usize>();
        let b_total = b.1.values().sum::<usize>();
        b_total.cmp(&a_total).then_with(|| a.0.cmp(&b.0))
    });
    for (key, values) in valid_non_selectors {
        let total = values.values().sum::<usize>();
        println!("{total:6} {key}");
        let mut values = values.into_iter().collect::<Vec<_>>();
        values.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        for (kind, count) in values.iter().take(12) {
            println!("       {count:6} {kind}");
        }
    }
}

fn scan_face(
    face: &CardFace,
    raw_predicates: &mut BTreeMap<String, usize>,
    valid_non_selectors: &mut BTreeMap<String, BTreeMap<String, usize>>,
) {
    for line in face
        .abilities
        .iter()
        .chain(&face.static_abilities)
        .chain(&face.triggers)
        .chain(&face.replacements)
        .chain(face.svars.values())
    {
        scan_params(line, raw_predicates, valid_non_selectors);
    }
}

fn scan_params(
    raw: &str,
    raw_predicates: &mut BTreeMap<String, usize>,
    valid_non_selectors: &mut BTreeMap<String, BTreeMap<String, usize>>,
) {
    let params = Params::from_raw(raw);
    for (key, value) in params.iter() {
        match parse_semantic_param_value(key, value) {
            SemanticParamValue::Selector(_) | SemanticParamValue::Reference(_) => {
                let selector = CompiledSelector::parse(value);
                for raw in selector.raw_predicates() {
                    if !raw.is_empty() {
                        *raw_predicates.entry(raw.to_string()).or_default() += 1;
                    }
                }
            }
            semantic if key.starts_with("Valid") => {
                let kind = format!("{:?}", semantic.kind());
                *valid_non_selectors
                    .entry(key.to_string())
                    .or_default()
                    .entry(kind)
                    .or_default() += 1;
            }
            _ => {}
        }
    }
}
