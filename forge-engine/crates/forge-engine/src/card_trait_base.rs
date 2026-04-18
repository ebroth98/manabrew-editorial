use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::card::Card;
use crate::card::card_state::CardState;
use crate::core::{HasSVars, Identifiable};
use crate::game::GameState;
use crate::keyword::keyword_interface::KeywordInterface;
use crate::parsing::Params;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CardTraitBase {
    id: i32,
    #[serde(skip)]
    host_card: Option<Card>,
    #[serde(skip)]
    card_state: Option<CardState>,
    #[serde(skip)]
    keyword: Option<KeywordInterface>,

    original_map_params: HashMap<String, String>,
    map_params: HashMap<String, String>,

    intrinsic: bool,
    suppressed: bool,

    svars: HashMap<String, String>,

    intrinsic_changed_text_colors: HashMap<String, String>,
    intrinsic_changed_text_types: HashMap<String, String>,
    changed_text_colors: HashMap<String, String>,
    changed_text_types: HashMap<String, String>,
}

impl CardTraitBase {
    pub fn set_id(&mut self, id: i32) {
        self.id = id;
    }

    pub fn get_map_params(&self) -> &HashMap<String, String> {
        &self.map_params
    }

    pub fn get_original_map_params(&self) -> &HashMap<String, String> {
        &self.original_map_params
    }

    pub fn has_param(&self, key: &str) -> bool {
        self.map_params.contains_key(key)
    }

    pub fn get_param(&self, key: &str) -> Option<&str> {
        self.map_params.get(key).map(String::as_str)
    }

    pub fn get_original_param(&self, key: &str) -> Option<&str> {
        self.original_map_params.get(key).map(String::as_str)
    }

    pub fn put_param(&mut self, key: String, value: String) -> Option<String> {
        self.map_params.insert(key, value)
    }

    pub fn remove_param(&mut self, key: &str) {
        self.map_params.remove(key);
    }

    pub fn get_host_card(&self) -> &Card {
        self.host_card
            .as_ref()
            .expect("CardTraitBase host_card must be bound before use")
    }

    pub fn set_host_card(&mut self, card: Card) {
        self.host_card = Some(card);
    }

    pub fn get_card_state(&self) -> Option<&CardState> {
        self.card_state.as_ref()
    }

    pub fn set_card_state(&mut self, state: CardState) {
        self.card_state = Some(state);
    }

    pub fn get_keyword(&self) -> Option<&KeywordInterface> {
        self.keyword.as_ref()
    }

    pub fn set_keyword(&mut self, keyword: KeywordInterface) {
        self.keyword = Some(keyword);
    }

    pub fn is_keyword(&self, keyword: crate::keyword::keyword_instance::Keyword) -> bool {
        self.keyword
            .as_ref()
            .map(|current| current.get_keyword() == keyword)
            .unwrap_or(false)
    }

    pub fn is_intrinsic(&self) -> bool {
        self.intrinsic
    }

    pub fn set_intrinsic(&mut self, intrinsic: bool) {
        self.intrinsic = intrinsic;
    }

    pub fn is_suppressed(&self) -> bool {
        self.suppressed
    }

    pub fn set_suppressed(&mut self, suppressed: bool) {
        self.suppressed = suppressed;
    }

    pub fn change_text(&mut self) {
        self.changed_text_colors = self.intrinsic_changed_text_colors.clone();
        self.changed_text_types = self.intrinsic_changed_text_types.clone();
    }

    pub fn changed_text_pairs(&self) -> Vec<(String, String)> {
        self.changed_text_colors
            .iter()
            .chain(self.changed_text_types.iter())
            .map(|(original, replacement)| (original.clone(), replacement.clone()))
            .collect()
    }

    pub fn change_text_intrinsic(
        &mut self,
        color_map: HashMap<String, String>,
        type_map: HashMap<String, String>,
    ) {
        self.intrinsic_changed_text_colors = color_map.clone();
        self.intrinsic_changed_text_types = type_map.clone();
        self.changed_text_colors = color_map;
        self.changed_text_types = type_map;
    }

    pub fn meets_common_requirements(&self, game: &GameState, params: &Params) -> bool {
        crate::card::valid_filter::meets_common_requirements_with_svars(
            game,
            params,
            self.get_host_card(),
            self,
        )
    }

    pub fn copy_helper(&self, copy: &mut CardTraitBase, host: Card) {
        self.copy_helper_with_text(copy, host, false);
    }

    pub fn copy_helper_with_text(
        &self,
        copy: &mut CardTraitBase,
        host: Card,
        keep_text_changes: bool,
    ) {
        copy.original_map_params = self.original_map_params.clone();
        copy.map_params = if keep_text_changes {
            self.map_params.clone()
        } else {
            self.original_map_params.clone()
        };
        copy.set_svars(self.svars.clone());
        copy.card_state = self.card_state.clone();
        // Mirrors Java copyHelper: assign host directly instead of using set_host_card.
        copy.host_card = Some(host);
        copy.keyword = self.keyword.clone();
    }

    pub fn copy_helper_keep_text(
        &self,
        copy: &mut CardTraitBase,
        host: Card,
        keep_text_changes: bool,
    ) {
        self.copy_helper_with_text(copy, host, keep_text_changes);
    }
}

impl Identifiable for CardTraitBase {
    fn id(&self) -> i32 {
        self.id
    }
}

impl HasSVars for CardTraitBase {
    fn get_svar(&self, name: &str) -> Option<&str> {
        self.svars
            .get(name)
            .map(String::as_str)
            .or_else(|| {
                self.keyword
                    .as_ref()
                    .and_then(|keyword| keyword.get_static())
                    .and_then(|st| st.params.get(name))
            })
            .or_else(|| self.card_state.as_ref().and_then(|state| state.get_svar(name)))
            .or_else(|| self.host_card.as_ref().and_then(|card| card.get_s_var(name)))
    }
    fn has_svar(&self, name: &str) -> bool {
        self.svars.contains_key(name)
    }
    fn set_svar(&mut self, name: String, value: String) {
        self.svars.insert(name, value);
    }
    fn set_svars(&mut self, new: HashMap<String, String>) {
        self.svars = new;
    }
    fn get_svars(&self) -> &HashMap<String, String> {
        &self.svars
    }
    fn remove_svar(&mut self, var: &str) {
        self.svars.remove(var);
    }
}
