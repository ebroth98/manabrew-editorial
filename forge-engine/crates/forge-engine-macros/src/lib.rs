use proc_macro::TokenStream;

/// Generate the stateless `SpellAbilityEffect` unit struct and trait impl
/// around a resolve function body.
///
/// Usage:
/// `#[spell_effect(FooEffect)] fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) { ... }`
#[proc_macro_attribute]
pub fn spell_effect(attr: TokenStream, item: TokenStream) -> TokenStream {
    let effect_name = attr.to_string();
    if effect_name.trim().is_empty() || effect_name.contains(',') {
        return compile_error("expected a single effect type name");
    }

    let item = item.to_string();
    let expanded = format!(
        "pub struct {effect_name};
         impl crate::ability::spell_ability_effect::SpellAbilityEffect for {effect_name} {{
             {item}
         }}"
    );
    expanded
        .parse()
        .unwrap_or_else(|_| compile_error("failed to generate SpellAbilityEffect implementation"))
}

fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!({message:?});")
        .parse()
        .expect("compile_error output should parse")
}
