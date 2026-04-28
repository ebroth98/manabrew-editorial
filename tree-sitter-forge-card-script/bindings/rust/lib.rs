use tree_sitter::Language;

unsafe extern "C" {
    fn tree_sitter_forge_card_script() -> Language;
}

pub fn language() -> Language {
    unsafe { tree_sitter_forge_card_script() }
}

pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");
