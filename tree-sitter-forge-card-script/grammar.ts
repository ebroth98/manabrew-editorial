/// <reference types="tree-sitter-cli/dsl" />

export default grammar({
  name: "forge_card_script",

  extras: (_) => [],

  rules: {
    card_script: ($) => seq($.face, repeat(seq($.face_separator, $.face))),

    face: ($) => repeat1(seq(optional($._content_line), $._newline)),

    face_separator: ($) => seq(alias("ALTERNATE", $.alternate_keyword), $._newline),

    _content_line: ($) =>
      choice(
        $.ability_line,
        $.trigger_line,
        $.static_ability_line,
        $.replacement_line,
        $.svar_line,
        $.keyword_line,
        $.alternate_mode_line,
        $.specialize_line,
        $.ignored_line,
        $.field_line,
        $.comment_line,
      ),

    comment_line: ($) => seq("#", optional(alias(/[^\n]*/, $.comment_text))),

    field_line: ($) =>
      seq(alias($._field_key, $.key), ":", optional(alias(/[^\n]*/, $.value))),

    _field_key: (_) =>
      choice(
        "Name",
        "ManaCost",
        "Types",
        "PT",
        "Colors",
        "Defense",
        "Loyalty",
        "Oracle",
        "Text",
        "FlavorName",
        "Lights",
        "MeldPair",
        "Draft",
        "Variant",
      ),

    alternate_mode_line: ($) => seq("AlternateMode", ":", optional(alias(/[^\n]*/, $.value))),

    specialize_line: ($) => seq(/SPECIALIZE[^:\n]*/, ":", optional(alias(/[^\n]*/, $.value))),

    ignored_line: ($) =>
      seq(alias($._ignored_key, $.key), ":", optional(alias(/[^\n]*/, $.value))),

    _ignored_key: (_) =>
      choice("AI", "DeckHints", "DeckNeeds", "DeckHas", "HandLifeModifier"),

    ability_line: ($) => seq("A", ":", $.ability_body),

    ability_body: ($) =>
      seq(
        $.ability_record,
        "$",
        optional(" "),
        alias(/[^|\n]*/, $.api_name),
        optional(seq("|", $.param_record)),
      ),

    ability_record: (_) => choice("SP", "AB"),

    trigger_line: ($) => seq("T", ":", $.param_record),

    static_ability_line: ($) => seq("S", ":", $.param_record),

    replacement_line: ($) => seq("R", ":", $.param_record),

    svar_line: ($) =>
      seq(
        "SVar",
        ":",
        alias(/[A-Za-z_][A-Za-z0-9_]*/, $.svar_name),
        ":",
        alias(/[^\n]+/, $.svar_value),
      ),

    keyword_line: ($) => seq("K", ":", alias(/[^\n]+/, $.keyword_value)),

    param_record: ($) => seq($.param, repeat(seq("|", $.param))),

    param: ($) =>
      seq(
        alias($._param_key, $.param_key),
        "$",
        optional(" "),
        optional(alias($._param_value, $.param_value)),
      ),

    _param_key: (_) => /[^$|\n]+/,
    _param_value: (_) => /[^|\n]+/,

    _newline: (_) => /\r?\n/,
  },
});
