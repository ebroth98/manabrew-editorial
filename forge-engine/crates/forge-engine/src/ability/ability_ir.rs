use crate::parsing::amount::AmountExpr;
use crate::parsing::{keys, ParsedParams, SemanticParamValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbilityIr {
    DamageAll(NumericAmountIr),
    DealDamage(DealDamageIr),
    Draw(NumericAmountIr),
    GainLife(NumericAmountIr),
    LifeSet(NumericAmountIr),
    LoseLife(NumericAmountIr),
    Mill(NumericAmountIr),
    Poison(NumericAmountIr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DealDamageIr {
    pub amount: Option<AmountExpr>,
    pub valid_targets: Option<String>,
    pub damage_map: bool,
}

impl DealDamageIr {
    pub fn from_parsed(params: &ParsedParams<'_>) -> Self {
        Self {
            amount: semantic_amount_expr(params, keys::NUM_DMG),
            valid_targets: params.get(keys::VALID_TGTS).map(str::to_string),
            damage_map: params.has(keys::DAMAGE_MAP),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericAmountIr {
    pub amount: Option<AmountExpr>,
}

impl NumericAmountIr {
    pub fn from_parsed(params: &ParsedParams<'_>, key: &str) -> Self {
        Self {
            amount: semantic_amount_expr(params, key),
        }
    }
}

fn semantic_amount_expr(params: &ParsedParams<'_>, key: &str) -> Option<AmountExpr> {
    let param = params.semantic_get(key)?;
    match param.value {
        SemanticParamValue::Amount(amount) => Some(AmountExpr::from_semantic(&amount)),
        SemanticParamValue::Integer(value) => Some(AmountExpr::Literal(value)),
        SemanticParamValue::Raw(raw)
        | SemanticParamValue::Expression(raw)
        | SemanticParamValue::Text(raw)
        | SemanticParamValue::Symbol(raw) => Some(AmountExpr::parse(raw)),
        _ => param
            .raw_value
            .is_empty()
            .then_some(AmountExpr::Raw(String::new())),
    }
}

pub fn lower_ability_ir(
    api: Option<crate::ability::api_type::ApiType>,
    params: &ParsedParams<'_>,
) -> Option<AbilityIr> {
    match api {
        Some(crate::ability::api_type::ApiType::DamageAll) => Some(AbilityIr::DamageAll(
            NumericAmountIr::from_parsed(params, keys::NUM_DMG),
        )),
        Some(crate::ability::api_type::ApiType::DealDamage) => {
            Some(AbilityIr::DealDamage(DealDamageIr::from_parsed(params)))
        }
        Some(crate::ability::api_type::ApiType::Draw) => Some(AbilityIr::Draw(
            NumericAmountIr::from_parsed(params, keys::NUM_CARDS),
        )),
        Some(crate::ability::api_type::ApiType::GainLife) => Some(AbilityIr::GainLife(
            NumericAmountIr::from_parsed(params, keys::LIFE_AMOUNT),
        )),
        Some(crate::ability::api_type::ApiType::SetLife) => Some(AbilityIr::LifeSet(
            NumericAmountIr::from_parsed(params, keys::LIFE_AMOUNT),
        )),
        Some(crate::ability::api_type::ApiType::LoseLife) => Some(AbilityIr::LoseLife(
            NumericAmountIr::from_parsed(params, keys::LIFE_AMOUNT),
        )),
        Some(crate::ability::api_type::ApiType::Mill) => Some(AbilityIr::Mill(
            NumericAmountIr::from_parsed(params, keys::NUM_CARDS),
        )),
        Some(crate::ability::api_type::ApiType::Poison) => Some(AbilityIr::Poison(
            NumericAmountIr::from_parsed(params, keys::NUM),
        )),
        _ => None,
    }
}
