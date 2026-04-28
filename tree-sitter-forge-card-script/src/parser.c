#include "tree_sitter/parser.h"

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#ifdef _MSC_VER
#pragma optimize("", off)
#elif defined(__clang__)
#pragma clang optimize off
#elif defined(__GNUC__)
#pragma GCC optimize ("O0")
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 70
#define LARGE_STATE_COUNT 5
#define SYMBOL_COUNT 67
#define ALIAS_COUNT 2
#define TOKEN_COUNT 43
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 0
#define MAX_ALIAS_SEQUENCE_LENGTH 6
#define PRODUCTION_ID_COUNT 3

enum ts_symbol_identifiers {
  anon_sym_ALTERNATE = 1,
  anon_sym_POUND = 2,
  aux_sym_comment_line_token1 = 3,
  anon_sym_COLON = 4,
  anon_sym_Name = 5,
  anon_sym_ManaCost = 6,
  anon_sym_Types = 7,
  anon_sym_PT = 8,
  anon_sym_Colors = 9,
  anon_sym_Defense = 10,
  anon_sym_Loyalty = 11,
  anon_sym_Oracle = 12,
  anon_sym_Text = 13,
  anon_sym_FlavorName = 14,
  anon_sym_Lights = 15,
  anon_sym_MeldPair = 16,
  anon_sym_Draft = 17,
  anon_sym_Variant = 18,
  anon_sym_AlternateMode = 19,
  aux_sym_specialize_line_token1 = 20,
  anon_sym_AI = 21,
  anon_sym_DeckHints = 22,
  anon_sym_DeckNeeds = 23,
  anon_sym_DeckHas = 24,
  anon_sym_HandLifeModifier = 25,
  anon_sym_A = 26,
  anon_sym_DOLLAR = 27,
  anon_sym_SPACE = 28,
  aux_sym_ability_body_token1 = 29,
  anon_sym_PIPE = 30,
  anon_sym_SP = 31,
  anon_sym_AB = 32,
  anon_sym_T = 33,
  anon_sym_S = 34,
  anon_sym_R = 35,
  anon_sym_SVar = 36,
  aux_sym_svar_line_token1 = 37,
  aux_sym_svar_line_token2 = 38,
  anon_sym_K = 39,
  sym__param_key = 40,
  sym__param_value = 41,
  sym__newline = 42,
  sym_card_script = 43,
  sym_face = 44,
  sym_face_separator = 45,
  sym__content_line = 46,
  sym_comment_line = 47,
  sym_field_line = 48,
  sym__field_key = 49,
  sym_alternate_mode_line = 50,
  sym_specialize_line = 51,
  sym_ignored_line = 52,
  sym__ignored_key = 53,
  sym_ability_line = 54,
  sym_ability_body = 55,
  sym_ability_record = 56,
  sym_trigger_line = 57,
  sym_static_ability_line = 58,
  sym_replacement_line = 59,
  sym_svar_line = 60,
  sym_keyword_line = 61,
  sym_param_record = 62,
  sym_param = 63,
  aux_sym_card_script_repeat1 = 64,
  aux_sym_face_repeat1 = 65,
  aux_sym_param_record_repeat1 = 66,
  alias_sym_comment_text = 67,
  alias_sym_keyword_value = 68,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [anon_sym_ALTERNATE] = "alternate_keyword",
  [anon_sym_POUND] = "#",
  [aux_sym_comment_line_token1] = "value",
  [anon_sym_COLON] = ":",
  [anon_sym_Name] = "Name",
  [anon_sym_ManaCost] = "ManaCost",
  [anon_sym_Types] = "Types",
  [anon_sym_PT] = "PT",
  [anon_sym_Colors] = "Colors",
  [anon_sym_Defense] = "Defense",
  [anon_sym_Loyalty] = "Loyalty",
  [anon_sym_Oracle] = "Oracle",
  [anon_sym_Text] = "Text",
  [anon_sym_FlavorName] = "FlavorName",
  [anon_sym_Lights] = "Lights",
  [anon_sym_MeldPair] = "MeldPair",
  [anon_sym_Draft] = "Draft",
  [anon_sym_Variant] = "Variant",
  [anon_sym_AlternateMode] = "AlternateMode",
  [aux_sym_specialize_line_token1] = "specialize_line_token1",
  [anon_sym_AI] = "AI",
  [anon_sym_DeckHints] = "DeckHints",
  [anon_sym_DeckNeeds] = "DeckNeeds",
  [anon_sym_DeckHas] = "DeckHas",
  [anon_sym_HandLifeModifier] = "HandLifeModifier",
  [anon_sym_A] = "A",
  [anon_sym_DOLLAR] = "$",
  [anon_sym_SPACE] = " ",
  [aux_sym_ability_body_token1] = "api_name",
  [anon_sym_PIPE] = "|",
  [anon_sym_SP] = "SP",
  [anon_sym_AB] = "AB",
  [anon_sym_T] = "T",
  [anon_sym_S] = "S",
  [anon_sym_R] = "R",
  [anon_sym_SVar] = "SVar",
  [aux_sym_svar_line_token1] = "svar_name",
  [aux_sym_svar_line_token2] = "svar_value",
  [anon_sym_K] = "K",
  [sym__param_key] = "param_key",
  [sym__param_value] = "param_value",
  [sym__newline] = "_newline",
  [sym_card_script] = "card_script",
  [sym_face] = "face",
  [sym_face_separator] = "face_separator",
  [sym__content_line] = "_content_line",
  [sym_comment_line] = "comment_line",
  [sym_field_line] = "field_line",
  [sym__field_key] = "key",
  [sym_alternate_mode_line] = "alternate_mode_line",
  [sym_specialize_line] = "specialize_line",
  [sym_ignored_line] = "ignored_line",
  [sym__ignored_key] = "key",
  [sym_ability_line] = "ability_line",
  [sym_ability_body] = "ability_body",
  [sym_ability_record] = "ability_record",
  [sym_trigger_line] = "trigger_line",
  [sym_static_ability_line] = "static_ability_line",
  [sym_replacement_line] = "replacement_line",
  [sym_svar_line] = "svar_line",
  [sym_keyword_line] = "keyword_line",
  [sym_param_record] = "param_record",
  [sym_param] = "param",
  [aux_sym_card_script_repeat1] = "card_script_repeat1",
  [aux_sym_face_repeat1] = "face_repeat1",
  [aux_sym_param_record_repeat1] = "param_record_repeat1",
  [alias_sym_comment_text] = "comment_text",
  [alias_sym_keyword_value] = "keyword_value",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [anon_sym_ALTERNATE] = anon_sym_ALTERNATE,
  [anon_sym_POUND] = anon_sym_POUND,
  [aux_sym_comment_line_token1] = aux_sym_comment_line_token1,
  [anon_sym_COLON] = anon_sym_COLON,
  [anon_sym_Name] = anon_sym_Name,
  [anon_sym_ManaCost] = anon_sym_ManaCost,
  [anon_sym_Types] = anon_sym_Types,
  [anon_sym_PT] = anon_sym_PT,
  [anon_sym_Colors] = anon_sym_Colors,
  [anon_sym_Defense] = anon_sym_Defense,
  [anon_sym_Loyalty] = anon_sym_Loyalty,
  [anon_sym_Oracle] = anon_sym_Oracle,
  [anon_sym_Text] = anon_sym_Text,
  [anon_sym_FlavorName] = anon_sym_FlavorName,
  [anon_sym_Lights] = anon_sym_Lights,
  [anon_sym_MeldPair] = anon_sym_MeldPair,
  [anon_sym_Draft] = anon_sym_Draft,
  [anon_sym_Variant] = anon_sym_Variant,
  [anon_sym_AlternateMode] = anon_sym_AlternateMode,
  [aux_sym_specialize_line_token1] = aux_sym_specialize_line_token1,
  [anon_sym_AI] = anon_sym_AI,
  [anon_sym_DeckHints] = anon_sym_DeckHints,
  [anon_sym_DeckNeeds] = anon_sym_DeckNeeds,
  [anon_sym_DeckHas] = anon_sym_DeckHas,
  [anon_sym_HandLifeModifier] = anon_sym_HandLifeModifier,
  [anon_sym_A] = anon_sym_A,
  [anon_sym_DOLLAR] = anon_sym_DOLLAR,
  [anon_sym_SPACE] = anon_sym_SPACE,
  [aux_sym_ability_body_token1] = aux_sym_ability_body_token1,
  [anon_sym_PIPE] = anon_sym_PIPE,
  [anon_sym_SP] = anon_sym_SP,
  [anon_sym_AB] = anon_sym_AB,
  [anon_sym_T] = anon_sym_T,
  [anon_sym_S] = anon_sym_S,
  [anon_sym_R] = anon_sym_R,
  [anon_sym_SVar] = anon_sym_SVar,
  [aux_sym_svar_line_token1] = aux_sym_svar_line_token1,
  [aux_sym_svar_line_token2] = aux_sym_svar_line_token2,
  [anon_sym_K] = anon_sym_K,
  [sym__param_key] = sym__param_key,
  [sym__param_value] = sym__param_value,
  [sym__newline] = sym__newline,
  [sym_card_script] = sym_card_script,
  [sym_face] = sym_face,
  [sym_face_separator] = sym_face_separator,
  [sym__content_line] = sym__content_line,
  [sym_comment_line] = sym_comment_line,
  [sym_field_line] = sym_field_line,
  [sym__field_key] = sym__field_key,
  [sym_alternate_mode_line] = sym_alternate_mode_line,
  [sym_specialize_line] = sym_specialize_line,
  [sym_ignored_line] = sym_ignored_line,
  [sym__ignored_key] = sym__field_key,
  [sym_ability_line] = sym_ability_line,
  [sym_ability_body] = sym_ability_body,
  [sym_ability_record] = sym_ability_record,
  [sym_trigger_line] = sym_trigger_line,
  [sym_static_ability_line] = sym_static_ability_line,
  [sym_replacement_line] = sym_replacement_line,
  [sym_svar_line] = sym_svar_line,
  [sym_keyword_line] = sym_keyword_line,
  [sym_param_record] = sym_param_record,
  [sym_param] = sym_param,
  [aux_sym_card_script_repeat1] = aux_sym_card_script_repeat1,
  [aux_sym_face_repeat1] = aux_sym_face_repeat1,
  [aux_sym_param_record_repeat1] = aux_sym_param_record_repeat1,
  [alias_sym_comment_text] = alias_sym_comment_text,
  [alias_sym_keyword_value] = alias_sym_keyword_value,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [anon_sym_ALTERNATE] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_POUND] = {
    .visible = true,
    .named = false,
  },
  [aux_sym_comment_line_token1] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_COLON] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_Name] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_ManaCost] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_Types] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_PT] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_Colors] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_Defense] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_Loyalty] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_Oracle] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_Text] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_FlavorName] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_Lights] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_MeldPair] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_Draft] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_Variant] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_AlternateMode] = {
    .visible = true,
    .named = false,
  },
  [aux_sym_specialize_line_token1] = {
    .visible = false,
    .named = false,
  },
  [anon_sym_AI] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_DeckHints] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_DeckNeeds] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_DeckHas] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_HandLifeModifier] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_A] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_DOLLAR] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_SPACE] = {
    .visible = true,
    .named = false,
  },
  [aux_sym_ability_body_token1] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_PIPE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_SP] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_AB] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_T] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_S] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_R] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_SVar] = {
    .visible = true,
    .named = false,
  },
  [aux_sym_svar_line_token1] = {
    .visible = true,
    .named = true,
  },
  [aux_sym_svar_line_token2] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_K] = {
    .visible = true,
    .named = false,
  },
  [sym__param_key] = {
    .visible = true,
    .named = true,
  },
  [sym__param_value] = {
    .visible = true,
    .named = true,
  },
  [sym__newline] = {
    .visible = false,
    .named = true,
  },
  [sym_card_script] = {
    .visible = true,
    .named = true,
  },
  [sym_face] = {
    .visible = true,
    .named = true,
  },
  [sym_face_separator] = {
    .visible = true,
    .named = true,
  },
  [sym__content_line] = {
    .visible = false,
    .named = true,
  },
  [sym_comment_line] = {
    .visible = true,
    .named = true,
  },
  [sym_field_line] = {
    .visible = true,
    .named = true,
  },
  [sym__field_key] = {
    .visible = true,
    .named = true,
  },
  [sym_alternate_mode_line] = {
    .visible = true,
    .named = true,
  },
  [sym_specialize_line] = {
    .visible = true,
    .named = true,
  },
  [sym_ignored_line] = {
    .visible = true,
    .named = true,
  },
  [sym__ignored_key] = {
    .visible = true,
    .named = true,
  },
  [sym_ability_line] = {
    .visible = true,
    .named = true,
  },
  [sym_ability_body] = {
    .visible = true,
    .named = true,
  },
  [sym_ability_record] = {
    .visible = true,
    .named = true,
  },
  [sym_trigger_line] = {
    .visible = true,
    .named = true,
  },
  [sym_static_ability_line] = {
    .visible = true,
    .named = true,
  },
  [sym_replacement_line] = {
    .visible = true,
    .named = true,
  },
  [sym_svar_line] = {
    .visible = true,
    .named = true,
  },
  [sym_keyword_line] = {
    .visible = true,
    .named = true,
  },
  [sym_param_record] = {
    .visible = true,
    .named = true,
  },
  [sym_param] = {
    .visible = true,
    .named = true,
  },
  [aux_sym_card_script_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_face_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_param_record_repeat1] = {
    .visible = false,
    .named = false,
  },
  [alias_sym_comment_text] = {
    .visible = true,
    .named = true,
  },
  [alias_sym_keyword_value] = {
    .visible = true,
    .named = true,
  },
};

static const TSSymbol ts_alias_sequences[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH] = {
  [0] = {0},
  [1] = {
    [1] = alias_sym_comment_text,
  },
  [2] = {
    [2] = alias_sym_keyword_value,
  },
};

static const uint16_t ts_non_terminal_alias_map[] = {
  0,
};

static const TSStateId ts_primary_state_ids[STATE_COUNT] = {
  [0] = 0,
  [1] = 1,
  [2] = 2,
  [3] = 3,
  [4] = 4,
  [5] = 5,
  [6] = 6,
  [7] = 7,
  [8] = 8,
  [9] = 9,
  [10] = 10,
  [11] = 11,
  [12] = 12,
  [13] = 13,
  [14] = 14,
  [15] = 15,
  [16] = 16,
  [17] = 17,
  [18] = 18,
  [19] = 19,
  [20] = 20,
  [21] = 21,
  [22] = 22,
  [23] = 23,
  [24] = 24,
  [25] = 25,
  [26] = 26,
  [27] = 27,
  [28] = 28,
  [29] = 29,
  [30] = 30,
  [31] = 31,
  [32] = 32,
  [33] = 33,
  [34] = 34,
  [35] = 35,
  [36] = 36,
  [37] = 37,
  [38] = 38,
  [39] = 39,
  [40] = 40,
  [41] = 41,
  [42] = 42,
  [43] = 43,
  [44] = 44,
  [45] = 45,
  [46] = 46,
  [47] = 47,
  [48] = 48,
  [49] = 49,
  [50] = 50,
  [51] = 51,
  [52] = 52,
  [53] = 53,
  [54] = 54,
  [55] = 55,
  [56] = 56,
  [57] = 57,
  [58] = 58,
  [59] = 59,
  [60] = 60,
  [61] = 61,
  [62] = 62,
  [63] = 63,
  [64] = 64,
  [65] = 65,
  [66] = 66,
  [67] = 67,
  [68] = 68,
  [69] = 69,
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(131);
      ADVANCE_MAP(
        '\n', 328,
        '\r', 1,
        ' ', 184,
        '#', 134,
        '$', 183,
        ':', 138,
        'A', 181,
        'C', 285,
        'D', 246,
        'F', 274,
        'H', 224,
        'K', 324,
        'L', 265,
        'M', 230,
        'N', 225,
        'O', 298,
        'P', 220,
        'R', 199,
        'S', 196,
        'T', 194,
        'V', 229,
        '|', 189,
      );
      if (('B' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 1:
      if (lookahead == '\n') ADVANCE(328);
      END_STATE();
    case 2:
      if (lookahead == '\n') ADVANCE(328);
      if (lookahead == '\r') ADVANCE(326);
      if (lookahead == ' ') ADVANCE(185);
      if (lookahead == '|') ADVANCE(189);
      if (lookahead != 0) ADVANCE(327);
      END_STATE();
    case 3:
      if (lookahead == '\n') ADVANCE(328);
      if (lookahead == '\r') ADVANCE(326);
      if (lookahead == '|') ADVANCE(189);
      if (lookahead != 0) ADVANCE(327);
      END_STATE();
    case 4:
      if (lookahead == 'A') ADVANCE(7);
      if (lookahead == 'S') ADVANCE(23);
      END_STATE();
    case 5:
      if (lookahead == 'A') ADVANCE(18);
      END_STATE();
    case 6:
      if (lookahead == 'A') ADVANCE(28);
      END_STATE();
    case 7:
      if (lookahead == 'B') ADVANCE(192);
      END_STATE();
    case 8:
      if (lookahead == 'C') ADVANCE(15);
      END_STATE();
    case 9:
      if (lookahead == 'C') ADVANCE(96);
      END_STATE();
    case 10:
      if (lookahead == 'E') ADVANCE(170);
      END_STATE();
    case 11:
      if (lookahead == 'E') ADVANCE(8);
      END_STATE();
    case 12:
      if (lookahead == 'E') ADVANCE(25);
      END_STATE();
    case 13:
      if (lookahead == 'E') ADVANCE(132);
      END_STATE();
    case 14:
      if (lookahead == 'H') ADVANCE(42);
      if (lookahead == 'N') ADVANCE(63);
      END_STATE();
    case 15:
      if (lookahead == 'I') ADVANCE(5);
      END_STATE();
    case 16:
      if (lookahead == 'I') ADVANCE(29);
      END_STATE();
    case 17:
      if (lookahead == 'L') ADVANCE(72);
      END_STATE();
    case 18:
      if (lookahead == 'L') ADVANCE(16);
      END_STATE();
    case 19:
      if (lookahead == 'M') ADVANCE(92);
      END_STATE();
    case 20:
      if (lookahead == 'M') ADVANCE(94);
      END_STATE();
    case 21:
      if (lookahead == 'N') ADVANCE(6);
      END_STATE();
    case 22:
      if (lookahead == 'N') ADVANCE(44);
      END_STATE();
    case 23:
      if (lookahead == 'P') ADVANCE(190);
      END_STATE();
    case 24:
      if (lookahead == 'P') ADVANCE(39);
      END_STATE();
    case 25:
      if (lookahead == 'R') ADVANCE(21);
      END_STATE();
    case 26:
      if (lookahead == 'T') ADVANCE(145);
      END_STATE();
    case 27:
      if (lookahead == 'T') ADVANCE(12);
      END_STATE();
    case 28:
      if (lookahead == 'T') ADVANCE(13);
      END_STATE();
    case 29:
      if (lookahead == 'Z') ADVANCE(10);
      END_STATE();
    case 30:
      if (lookahead == 'a') ADVANCE(85);
      END_STATE();
    case 31:
      if (lookahead == 'a') ADVANCE(83);
      END_STATE();
    case 32:
      if (lookahead == 'a') ADVANCE(66);
      END_STATE();
    case 33:
      if (lookahead == 'a') ADVANCE(123);
      END_STATE();
    case 34:
      if (lookahead == 'a') ADVANCE(46);
      END_STATE();
    case 35:
      if (lookahead == 'a') ADVANCE(98);
      END_STATE();
    case 36:
      if (lookahead == 'a') ADVANCE(88);
      if (lookahead == 'e') ADVANCE(79);
      END_STATE();
    case 37:
      if (lookahead == 'a') ADVANCE(9);
      END_STATE();
    case 38:
      if (lookahead == 'a') ADVANCE(99);
      END_STATE();
    case 39:
      if (lookahead == 'a') ADVANCE(74);
      END_STATE();
    case 40:
      if (lookahead == 'a') ADVANCE(81);
      END_STATE();
    case 41:
      if (lookahead == 'a') ADVANCE(87);
      END_STATE();
    case 42:
      if (lookahead == 'a') ADVANCE(109);
      if (lookahead == 'i') ADVANCE(90);
      END_STATE();
    case 43:
      if (lookahead == 'a') ADVANCE(122);
      END_STATE();
    case 44:
      if (lookahead == 'a') ADVANCE(84);
      END_STATE();
    case 45:
      if (lookahead == 'c') ADVANCE(77);
      if (lookahead == 'f') ADVANCE(60);
      END_STATE();
    case 46:
      if (lookahead == 'c') ADVANCE(82);
      END_STATE();
    case 47:
      if (lookahead == 'd') ADVANCE(17);
      END_STATE();
    case 48:
      if (lookahead == 'd') ADVANCE(24);
      END_STATE();
    case 49:
      if (lookahead == 'd') ADVANCE(73);
      END_STATE();
    case 50:
      if (lookahead == 'd') ADVANCE(111);
      END_STATE();
    case 51:
      if (lookahead == 'd') ADVANCE(59);
      END_STATE();
    case 52:
      if (lookahead == 'e') ADVANCE(45);
      if (lookahead == 'r') ADVANCE(32);
      END_STATE();
    case 53:
      if (lookahead == 'e') ADVANCE(139);
      END_STATE();
    case 54:
      if (lookahead == 'e') ADVANCE(106);
      END_STATE();
    case 55:
      if (lookahead == 'e') ADVANCE(153);
      END_STATE();
    case 56:
      if (lookahead == 'e') ADVANCE(149);
      END_STATE();
    case 57:
      if (lookahead == 'e') ADVANCE(19);
      END_STATE();
    case 58:
      if (lookahead == 'e') ADVANCE(157);
      END_STATE();
    case 59:
      if (lookahead == 'e') ADVANCE(167);
      END_STATE();
    case 60:
      if (lookahead == 'e') ADVANCE(86);
      END_STATE();
    case 61:
      if (lookahead == 'e') ADVANCE(50);
      END_STATE();
    case 62:
      if (lookahead == 'e') ADVANCE(105);
      END_STATE();
    case 63:
      if (lookahead == 'e') ADVANCE(61);
      END_STATE();
    case 64:
      if (lookahead == 'e') ADVANCE(101);
      END_STATE();
    case 65:
      if (lookahead == 'e') ADVANCE(20);
      END_STATE();
    case 66:
      if (lookahead == 'f') ADVANCE(116);
      END_STATE();
    case 67:
      if (lookahead == 'f') ADVANCE(76);
      END_STATE();
    case 68:
      if (lookahead == 'f') ADVANCE(57);
      END_STATE();
    case 69:
      if (lookahead == 'g') ADVANCE(70);
      END_STATE();
    case 70:
      if (lookahead == 'h') ADVANCE(120);
      END_STATE();
    case 71:
      if (lookahead == 'i') ADVANCE(69);
      if (lookahead == 'o') ADVANCE(126);
      END_STATE();
    case 72:
      if (lookahead == 'i') ADVANCE(68);
      END_STATE();
    case 73:
      if (lookahead == 'i') ADVANCE(67);
      END_STATE();
    case 74:
      if (lookahead == 'i') ADVANCE(100);
      END_STATE();
    case 75:
      if (lookahead == 'i') ADVANCE(41);
      END_STATE();
    case 76:
      if (lookahead == 'i') ADVANCE(64);
      END_STATE();
    case 77:
      if (lookahead == 'k') ADVANCE(14);
      END_STATE();
    case 78:
      if (lookahead == 'l') ADVANCE(93);
      END_STATE();
    case 79:
      if (lookahead == 'l') ADVANCE(48);
      END_STATE();
    case 80:
      if (lookahead == 'l') ADVANCE(33);
      END_STATE();
    case 81:
      if (lookahead == 'l') ADVANCE(119);
      END_STATE();
    case 82:
      if (lookahead == 'l') ADVANCE(55);
      END_STATE();
    case 83:
      if (lookahead == 'm') ADVANCE(53);
      END_STATE();
    case 84:
      if (lookahead == 'm') ADVANCE(58);
      END_STATE();
    case 85:
      if (lookahead == 'n') ADVANCE(47);
      END_STATE();
    case 86:
      if (lookahead == 'n') ADVANCE(113);
      END_STATE();
    case 87:
      if (lookahead == 'n') ADVANCE(117);
      END_STATE();
    case 88:
      if (lookahead == 'n') ADVANCE(37);
      END_STATE();
    case 89:
      if (lookahead == 'n') ADVANCE(43);
      END_STATE();
    case 90:
      if (lookahead == 'n') ADVANCE(121);
      END_STATE();
    case 91:
      if (lookahead == 'o') ADVANCE(78);
      END_STATE();
    case 92:
      if (lookahead == 'o') ADVANCE(49);
      END_STATE();
    case 93:
      if (lookahead == 'o') ADVANCE(102);
      END_STATE();
    case 94:
      if (lookahead == 'o') ADVANCE(51);
      END_STATE();
    case 95:
      if (lookahead == 'o') ADVANCE(103);
      END_STATE();
    case 96:
      if (lookahead == 'o') ADVANCE(112);
      END_STATE();
    case 97:
      if (lookahead == 'p') ADVANCE(54);
      END_STATE();
    case 98:
      if (lookahead == 'r') ADVANCE(75);
      END_STATE();
    case 99:
      if (lookahead == 'r') ADVANCE(200);
      END_STATE();
    case 100:
      if (lookahead == 'r') ADVANCE(161);
      END_STATE();
    case 101:
      if (lookahead == 'r') ADVANCE(179);
      END_STATE();
    case 102:
      if (lookahead == 'r') ADVANCE(107);
      END_STATE();
    case 103:
      if (lookahead == 'r') ADVANCE(22);
      END_STATE();
    case 104:
      if (lookahead == 'r') ADVANCE(34);
      END_STATE();
    case 105:
      if (lookahead == 'r') ADVANCE(89);
      END_STATE();
    case 106:
      if (lookahead == 's') ADVANCE(143);
      END_STATE();
    case 107:
      if (lookahead == 's') ADVANCE(147);
      END_STATE();
    case 108:
      if (lookahead == 's') ADVANCE(159);
      END_STATE();
    case 109:
      if (lookahead == 's') ADVANCE(177);
      END_STATE();
    case 110:
      if (lookahead == 's') ADVANCE(173);
      END_STATE();
    case 111:
      if (lookahead == 's') ADVANCE(175);
      END_STATE();
    case 112:
      if (lookahead == 's') ADVANCE(118);
      END_STATE();
    case 113:
      if (lookahead == 's') ADVANCE(56);
      END_STATE();
    case 114:
      if (lookahead == 't') ADVANCE(62);
      END_STATE();
    case 115:
      if (lookahead == 't') ADVANCE(155);
      END_STATE();
    case 116:
      if (lookahead == 't') ADVANCE(163);
      END_STATE();
    case 117:
      if (lookahead == 't') ADVANCE(165);
      END_STATE();
    case 118:
      if (lookahead == 't') ADVANCE(141);
      END_STATE();
    case 119:
      if (lookahead == 't') ADVANCE(125);
      END_STATE();
    case 120:
      if (lookahead == 't') ADVANCE(108);
      END_STATE();
    case 121:
      if (lookahead == 't') ADVANCE(110);
      END_STATE();
    case 122:
      if (lookahead == 't') ADVANCE(65);
      END_STATE();
    case 123:
      if (lookahead == 'v') ADVANCE(95);
      END_STATE();
    case 124:
      if (lookahead == 'x') ADVANCE(115);
      END_STATE();
    case 125:
      if (lookahead == 'y') ADVANCE(151);
      END_STATE();
    case 126:
      if (lookahead == 'y') ADVANCE(40);
      END_STATE();
    case 127:
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 128:
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '$' &&
          lookahead != '|') ADVANCE(325);
      END_STATE();
    case 129:
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(322);
      END_STATE();
    case 130:
      if (eof) ADVANCE(131);
      ADVANCE_MAP(
        '\n', 328,
        '\r', 1,
        '#', 134,
        'A', 182,
        'C', 91,
        'D', 52,
        'F', 80,
        'H', 30,
        'K', 323,
        'L', 71,
        'M', 36,
        'N', 31,
        'O', 104,
        'P', 26,
        'R', 198,
        'S', 197,
        'T', 195,
        'V', 35,
      );
      END_STATE();
    case 131:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 132:
      ACCEPT_TOKEN(anon_sym_ALTERNATE);
      END_STATE();
    case 133:
      ACCEPT_TOKEN(anon_sym_ALTERNATE);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 134:
      ACCEPT_TOKEN(anon_sym_POUND);
      END_STATE();
    case 135:
      ACCEPT_TOKEN(aux_sym_comment_line_token1);
      if (lookahead == '\n') ADVANCE(328);
      if (lookahead == '\r') ADVANCE(136);
      if (lookahead != 0) ADVANCE(137);
      END_STATE();
    case 136:
      ACCEPT_TOKEN(aux_sym_comment_line_token1);
      if (lookahead == '\n') ADVANCE(328);
      if (lookahead != 0) ADVANCE(137);
      END_STATE();
    case 137:
      ACCEPT_TOKEN(aux_sym_comment_line_token1);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(137);
      END_STATE();
    case 138:
      ACCEPT_TOKEN(anon_sym_COLON);
      END_STATE();
    case 139:
      ACCEPT_TOKEN(anon_sym_Name);
      END_STATE();
    case 140:
      ACCEPT_TOKEN(anon_sym_Name);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 141:
      ACCEPT_TOKEN(anon_sym_ManaCost);
      END_STATE();
    case 142:
      ACCEPT_TOKEN(anon_sym_ManaCost);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 143:
      ACCEPT_TOKEN(anon_sym_Types);
      END_STATE();
    case 144:
      ACCEPT_TOKEN(anon_sym_Types);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 145:
      ACCEPT_TOKEN(anon_sym_PT);
      END_STATE();
    case 146:
      ACCEPT_TOKEN(anon_sym_PT);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 147:
      ACCEPT_TOKEN(anon_sym_Colors);
      END_STATE();
    case 148:
      ACCEPT_TOKEN(anon_sym_Colors);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 149:
      ACCEPT_TOKEN(anon_sym_Defense);
      END_STATE();
    case 150:
      ACCEPT_TOKEN(anon_sym_Defense);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 151:
      ACCEPT_TOKEN(anon_sym_Loyalty);
      END_STATE();
    case 152:
      ACCEPT_TOKEN(anon_sym_Loyalty);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 153:
      ACCEPT_TOKEN(anon_sym_Oracle);
      END_STATE();
    case 154:
      ACCEPT_TOKEN(anon_sym_Oracle);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 155:
      ACCEPT_TOKEN(anon_sym_Text);
      END_STATE();
    case 156:
      ACCEPT_TOKEN(anon_sym_Text);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 157:
      ACCEPT_TOKEN(anon_sym_FlavorName);
      END_STATE();
    case 158:
      ACCEPT_TOKEN(anon_sym_FlavorName);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 159:
      ACCEPT_TOKEN(anon_sym_Lights);
      END_STATE();
    case 160:
      ACCEPT_TOKEN(anon_sym_Lights);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 161:
      ACCEPT_TOKEN(anon_sym_MeldPair);
      END_STATE();
    case 162:
      ACCEPT_TOKEN(anon_sym_MeldPair);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 163:
      ACCEPT_TOKEN(anon_sym_Draft);
      END_STATE();
    case 164:
      ACCEPT_TOKEN(anon_sym_Draft);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 165:
      ACCEPT_TOKEN(anon_sym_Variant);
      END_STATE();
    case 166:
      ACCEPT_TOKEN(anon_sym_Variant);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 167:
      ACCEPT_TOKEN(anon_sym_AlternateMode);
      END_STATE();
    case 168:
      ACCEPT_TOKEN(anon_sym_AlternateMode);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 169:
      ACCEPT_TOKEN(aux_sym_specialize_line_token1);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(169);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          (lookahead < '0' || ':' < lookahead)) ADVANCE(170);
      END_STATE();
    case 170:
      ACCEPT_TOKEN(aux_sym_specialize_line_token1);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != ':') ADVANCE(170);
      END_STATE();
    case 171:
      ACCEPT_TOKEN(anon_sym_AI);
      END_STATE();
    case 172:
      ACCEPT_TOKEN(anon_sym_AI);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 173:
      ACCEPT_TOKEN(anon_sym_DeckHints);
      END_STATE();
    case 174:
      ACCEPT_TOKEN(anon_sym_DeckHints);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 175:
      ACCEPT_TOKEN(anon_sym_DeckNeeds);
      END_STATE();
    case 176:
      ACCEPT_TOKEN(anon_sym_DeckNeeds);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 177:
      ACCEPT_TOKEN(anon_sym_DeckHas);
      END_STATE();
    case 178:
      ACCEPT_TOKEN(anon_sym_DeckHas);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 179:
      ACCEPT_TOKEN(anon_sym_HandLifeModifier);
      END_STATE();
    case 180:
      ACCEPT_TOKEN(anon_sym_HandLifeModifier);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 181:
      ACCEPT_TOKEN(anon_sym_A);
      if (lookahead == 'B') ADVANCE(193);
      if (lookahead == 'I') ADVANCE(172);
      if (lookahead == 'L') ADVANCE(221);
      if (lookahead == 'l') ADVANCE(308);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 182:
      ACCEPT_TOKEN(anon_sym_A);
      if (lookahead == 'I') ADVANCE(171);
      if (lookahead == 'L') ADVANCE(27);
      if (lookahead == 'l') ADVANCE(114);
      END_STATE();
    case 183:
      ACCEPT_TOKEN(anon_sym_DOLLAR);
      END_STATE();
    case 184:
      ACCEPT_TOKEN(anon_sym_SPACE);
      END_STATE();
    case 185:
      ACCEPT_TOKEN(anon_sym_SPACE);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '|') ADVANCE(327);
      END_STATE();
    case 186:
      ACCEPT_TOKEN(anon_sym_SPACE);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '|') ADVANCE(188);
      END_STATE();
    case 187:
      ACCEPT_TOKEN(aux_sym_ability_body_token1);
      if (lookahead == ' ') ADVANCE(186);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '|') ADVANCE(188);
      END_STATE();
    case 188:
      ACCEPT_TOKEN(aux_sym_ability_body_token1);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '|') ADVANCE(188);
      END_STATE();
    case 189:
      ACCEPT_TOKEN(anon_sym_PIPE);
      END_STATE();
    case 190:
      ACCEPT_TOKEN(anon_sym_SP);
      END_STATE();
    case 191:
      ACCEPT_TOKEN(anon_sym_SP);
      if (lookahead == 'E') ADVANCE(204);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 192:
      ACCEPT_TOKEN(anon_sym_AB);
      END_STATE();
    case 193:
      ACCEPT_TOKEN(anon_sym_AB);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 194:
      ACCEPT_TOKEN(anon_sym_T);
      if (lookahead == 'e') ADVANCE(318);
      if (lookahead == 'y') ADVANCE(291);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 195:
      ACCEPT_TOKEN(anon_sym_T);
      if (lookahead == 'e') ADVANCE(124);
      if (lookahead == 'y') ADVANCE(97);
      END_STATE();
    case 196:
      ACCEPT_TOKEN(anon_sym_S);
      if (lookahead == 'P') ADVANCE(191);
      if (lookahead == 'V') ADVANCE(232);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 197:
      ACCEPT_TOKEN(anon_sym_S);
      if (lookahead == 'P') ADVANCE(11);
      if (lookahead == 'V') ADVANCE(38);
      END_STATE();
    case 198:
      ACCEPT_TOKEN(anon_sym_R);
      END_STATE();
    case 199:
      ACCEPT_TOKEN(anon_sym_R);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 200:
      ACCEPT_TOKEN(anon_sym_SVar);
      END_STATE();
    case 201:
      ACCEPT_TOKEN(anon_sym_SVar);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 202:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'A') ADVANCE(213);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('B' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 203:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'A') ADVANCE(222);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('B' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 204:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'C') ADVANCE(210);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 205:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'C') ADVANCE(290);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 206:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'E') ADVANCE(219);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 207:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'E') ADVANCE(133);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 208:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'E') ADVANCE(169);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 209:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'H') ADVANCE(236);
      if (lookahead == 'N') ADVANCE(257);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 210:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'I') ADVANCE(202);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 211:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'I') ADVANCE(223);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 212:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'L') ADVANCE(266);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 213:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'L') ADVANCE(211);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 214:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'M') ADVANCE(286);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 215:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'M') ADVANCE(288);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 216:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'N') ADVANCE(203);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 217:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'N') ADVANCE(238);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 218:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'P') ADVANCE(233);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 219:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'R') ADVANCE(216);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 220:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'T') ADVANCE(146);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 221:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'T') ADVANCE(206);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 222:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'T') ADVANCE(207);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 223:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'Z') ADVANCE(208);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Y') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 224:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(279);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 225:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(277);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 226:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(260);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 227:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(317);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 228:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(240);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 229:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(292);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 230:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(282);
      if (lookahead == 'e') ADVANCE(273);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 231:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(205);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 232:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(293);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 233:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(268);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 234:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(275);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 235:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(281);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 236:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(303);
      if (lookahead == 'i') ADVANCE(284);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 237:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(316);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 238:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'a') ADVANCE(278);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('b' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 239:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'c') ADVANCE(271);
      if (lookahead == 'f') ADVANCE(254);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 240:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'c') ADVANCE(276);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 241:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'd') ADVANCE(212);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 242:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'd') ADVANCE(218);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 243:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'd') ADVANCE(267);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 244:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'd') ADVANCE(305);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 245:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'd') ADVANCE(253);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 246:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(239);
      if (lookahead == 'r') ADVANCE(226);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 247:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(140);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 248:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(300);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 249:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(154);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 250:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(150);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 251:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(214);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 252:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(158);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 253:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(168);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 254:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(280);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 255:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(244);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 256:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(299);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 257:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(255);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 258:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(295);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 259:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'e') ADVANCE(215);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 260:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'f') ADVANCE(310);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 261:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'f') ADVANCE(270);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 262:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'f') ADVANCE(251);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 263:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'g') ADVANCE(264);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 264:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'h') ADVANCE(314);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 265:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'i') ADVANCE(263);
      if (lookahead == 'o') ADVANCE(320);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 266:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'i') ADVANCE(262);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 267:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'i') ADVANCE(261);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 268:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'i') ADVANCE(294);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 269:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'i') ADVANCE(235);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 270:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'i') ADVANCE(258);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 271:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'k') ADVANCE(209);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 272:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'l') ADVANCE(287);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 273:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'l') ADVANCE(242);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 274:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'l') ADVANCE(227);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 275:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'l') ADVANCE(313);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 276:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'l') ADVANCE(249);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 277:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'm') ADVANCE(247);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 278:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'm') ADVANCE(252);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 279:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'n') ADVANCE(241);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 280:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'n') ADVANCE(307);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 281:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'n') ADVANCE(311);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 282:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'n') ADVANCE(231);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 283:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'n') ADVANCE(237);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 284:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'n') ADVANCE(315);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 285:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'o') ADVANCE(272);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 286:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'o') ADVANCE(243);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 287:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'o') ADVANCE(296);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 288:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'o') ADVANCE(245);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 289:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'o') ADVANCE(297);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 290:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'o') ADVANCE(306);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 291:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'p') ADVANCE(248);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 292:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'r') ADVANCE(269);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 293:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'r') ADVANCE(201);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 294:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'r') ADVANCE(162);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 295:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'r') ADVANCE(180);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 296:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'r') ADVANCE(301);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 297:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'r') ADVANCE(217);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 298:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'r') ADVANCE(228);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 299:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'r') ADVANCE(283);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 300:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 's') ADVANCE(144);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 301:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 's') ADVANCE(148);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 302:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 's') ADVANCE(160);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 303:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 's') ADVANCE(178);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 304:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 's') ADVANCE(174);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 305:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 's') ADVANCE(176);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 306:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 's') ADVANCE(312);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 307:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 's') ADVANCE(250);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 308:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 't') ADVANCE(256);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 309:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 't') ADVANCE(156);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 310:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 't') ADVANCE(164);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 311:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 't') ADVANCE(166);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 312:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 't') ADVANCE(142);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 313:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 't') ADVANCE(319);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 314:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 't') ADVANCE(302);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 315:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 't') ADVANCE(304);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 316:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 't') ADVANCE(259);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 317:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'v') ADVANCE(289);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 318:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'x') ADVANCE(309);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 319:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'y') ADVANCE(152);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 320:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (lookahead == 'y') ADVANCE(234);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 321:
      ACCEPT_TOKEN(aux_sym_svar_line_token1);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 322:
      ACCEPT_TOKEN(aux_sym_svar_line_token2);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(322);
      END_STATE();
    case 323:
      ACCEPT_TOKEN(anon_sym_K);
      END_STATE();
    case 324:
      ACCEPT_TOKEN(anon_sym_K);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(321);
      END_STATE();
    case 325:
      ACCEPT_TOKEN(sym__param_key);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '$' &&
          lookahead != '|') ADVANCE(325);
      END_STATE();
    case 326:
      ACCEPT_TOKEN(sym__param_value);
      if (lookahead == '\n') ADVANCE(328);
      if (lookahead != 0 &&
          lookahead != '|') ADVANCE(327);
      END_STATE();
    case 327:
      ACCEPT_TOKEN(sym__param_value);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '|') ADVANCE(327);
      END_STATE();
    case 328:
      ACCEPT_TOKEN(sym__newline);
      END_STATE();
    default:
      return false;
  }
}

static const TSLexMode ts_lex_modes[STATE_COUNT] = {
  [0] = {.lex_state = 0},
  [1] = {.lex_state = 130},
  [2] = {.lex_state = 130},
  [3] = {.lex_state = 130},
  [4] = {.lex_state = 130},
  [5] = {.lex_state = 130},
  [6] = {.lex_state = 130},
  [7] = {.lex_state = 4},
  [8] = {.lex_state = 130},
  [9] = {.lex_state = 130},
  [10] = {.lex_state = 130},
  [11] = {.lex_state = 2},
  [12] = {.lex_state = 128},
  [13] = {.lex_state = 128},
  [14] = {.lex_state = 128},
  [15] = {.lex_state = 0},
  [16] = {.lex_state = 0},
  [17] = {.lex_state = 0},
  [18] = {.lex_state = 3},
  [19] = {.lex_state = 128},
  [20] = {.lex_state = 128},
  [21] = {.lex_state = 135},
  [22] = {.lex_state = 135},
  [23] = {.lex_state = 135},
  [24] = {.lex_state = 135},
  [25] = {.lex_state = 135},
  [26] = {.lex_state = 187},
  [27] = {.lex_state = 130},
  [28] = {.lex_state = 0},
  [29] = {.lex_state = 0},
  [30] = {.lex_state = 0},
  [31] = {.lex_state = 128},
  [32] = {.lex_state = 0},
  [33] = {.lex_state = 0},
  [34] = {.lex_state = 0},
  [35] = {.lex_state = 0},
  [36] = {.lex_state = 0},
  [37] = {.lex_state = 0},
  [38] = {.lex_state = 0},
  [39] = {.lex_state = 0},
  [40] = {.lex_state = 0},
  [41] = {.lex_state = 0},
  [42] = {.lex_state = 0},
  [43] = {.lex_state = 0},
  [44] = {.lex_state = 0},
  [45] = {.lex_state = 0},
  [46] = {.lex_state = 0},
  [47] = {.lex_state = 0},
  [48] = {.lex_state = 0},
  [49] = {.lex_state = 0},
  [50] = {.lex_state = 0},
  [51] = {.lex_state = 0},
  [52] = {.lex_state = 0},
  [53] = {.lex_state = 0},
  [54] = {.lex_state = 0},
  [55] = {.lex_state = 129},
  [56] = {.lex_state = 188},
  [57] = {.lex_state = 127},
  [58] = {.lex_state = 129},
  [59] = {.lex_state = 0},
  [60] = {.lex_state = 0},
  [61] = {.lex_state = 0},
  [62] = {.lex_state = 0},
  [63] = {.lex_state = 0},
  [64] = {.lex_state = 0},
  [65] = {.lex_state = 0},
  [66] = {.lex_state = 0},
  [67] = {.lex_state = 0},
  [68] = {.lex_state = 0},
  [69] = {.lex_state = 0},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [anon_sym_ALTERNATE] = ACTIONS(1),
    [anon_sym_POUND] = ACTIONS(1),
    [anon_sym_COLON] = ACTIONS(1),
    [anon_sym_Name] = ACTIONS(1),
    [anon_sym_ManaCost] = ACTIONS(1),
    [anon_sym_Types] = ACTIONS(1),
    [anon_sym_PT] = ACTIONS(1),
    [anon_sym_Colors] = ACTIONS(1),
    [anon_sym_Defense] = ACTIONS(1),
    [anon_sym_Loyalty] = ACTIONS(1),
    [anon_sym_Oracle] = ACTIONS(1),
    [anon_sym_Text] = ACTIONS(1),
    [anon_sym_FlavorName] = ACTIONS(1),
    [anon_sym_Lights] = ACTIONS(1),
    [anon_sym_MeldPair] = ACTIONS(1),
    [anon_sym_Draft] = ACTIONS(1),
    [anon_sym_Variant] = ACTIONS(1),
    [anon_sym_AlternateMode] = ACTIONS(1),
    [aux_sym_specialize_line_token1] = ACTIONS(1),
    [anon_sym_AI] = ACTIONS(1),
    [anon_sym_DeckHints] = ACTIONS(1),
    [anon_sym_DeckNeeds] = ACTIONS(1),
    [anon_sym_DeckHas] = ACTIONS(1),
    [anon_sym_HandLifeModifier] = ACTIONS(1),
    [anon_sym_A] = ACTIONS(1),
    [anon_sym_DOLLAR] = ACTIONS(1),
    [anon_sym_SPACE] = ACTIONS(1),
    [anon_sym_PIPE] = ACTIONS(1),
    [anon_sym_SP] = ACTIONS(1),
    [anon_sym_AB] = ACTIONS(1),
    [anon_sym_T] = ACTIONS(1),
    [anon_sym_S] = ACTIONS(1),
    [anon_sym_R] = ACTIONS(1),
    [anon_sym_SVar] = ACTIONS(1),
    [aux_sym_svar_line_token1] = ACTIONS(1),
    [anon_sym_K] = ACTIONS(1),
    [sym__newline] = ACTIONS(1),
  },
  [1] = {
    [sym_card_script] = STATE(63),
    [sym_face] = STATE(9),
    [sym__content_line] = STATE(65),
    [sym_comment_line] = STATE(65),
    [sym_field_line] = STATE(65),
    [sym__field_key] = STATE(66),
    [sym_alternate_mode_line] = STATE(65),
    [sym_specialize_line] = STATE(65),
    [sym_ignored_line] = STATE(65),
    [sym__ignored_key] = STATE(38),
    [sym_ability_line] = STATE(65),
    [sym_trigger_line] = STATE(65),
    [sym_static_ability_line] = STATE(65),
    [sym_replacement_line] = STATE(65),
    [sym_svar_line] = STATE(65),
    [sym_keyword_line] = STATE(65),
    [aux_sym_face_repeat1] = STATE(2),
    [anon_sym_POUND] = ACTIONS(3),
    [anon_sym_Name] = ACTIONS(5),
    [anon_sym_ManaCost] = ACTIONS(5),
    [anon_sym_Types] = ACTIONS(5),
    [anon_sym_PT] = ACTIONS(5),
    [anon_sym_Colors] = ACTIONS(5),
    [anon_sym_Defense] = ACTIONS(5),
    [anon_sym_Loyalty] = ACTIONS(5),
    [anon_sym_Oracle] = ACTIONS(5),
    [anon_sym_Text] = ACTIONS(5),
    [anon_sym_FlavorName] = ACTIONS(5),
    [anon_sym_Lights] = ACTIONS(5),
    [anon_sym_MeldPair] = ACTIONS(5),
    [anon_sym_Draft] = ACTIONS(5),
    [anon_sym_Variant] = ACTIONS(5),
    [anon_sym_AlternateMode] = ACTIONS(7),
    [aux_sym_specialize_line_token1] = ACTIONS(9),
    [anon_sym_AI] = ACTIONS(11),
    [anon_sym_DeckHints] = ACTIONS(11),
    [anon_sym_DeckNeeds] = ACTIONS(11),
    [anon_sym_DeckHas] = ACTIONS(11),
    [anon_sym_HandLifeModifier] = ACTIONS(11),
    [anon_sym_A] = ACTIONS(13),
    [anon_sym_T] = ACTIONS(15),
    [anon_sym_S] = ACTIONS(17),
    [anon_sym_R] = ACTIONS(19),
    [anon_sym_SVar] = ACTIONS(21),
    [anon_sym_K] = ACTIONS(23),
    [sym__newline] = ACTIONS(25),
  },
  [2] = {
    [sym__content_line] = STATE(65),
    [sym_comment_line] = STATE(65),
    [sym_field_line] = STATE(65),
    [sym__field_key] = STATE(66),
    [sym_alternate_mode_line] = STATE(65),
    [sym_specialize_line] = STATE(65),
    [sym_ignored_line] = STATE(65),
    [sym__ignored_key] = STATE(38),
    [sym_ability_line] = STATE(65),
    [sym_trigger_line] = STATE(65),
    [sym_static_ability_line] = STATE(65),
    [sym_replacement_line] = STATE(65),
    [sym_svar_line] = STATE(65),
    [sym_keyword_line] = STATE(65),
    [aux_sym_face_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(27),
    [anon_sym_ALTERNATE] = ACTIONS(27),
    [anon_sym_POUND] = ACTIONS(3),
    [anon_sym_Name] = ACTIONS(5),
    [anon_sym_ManaCost] = ACTIONS(5),
    [anon_sym_Types] = ACTIONS(5),
    [anon_sym_PT] = ACTIONS(5),
    [anon_sym_Colors] = ACTIONS(5),
    [anon_sym_Defense] = ACTIONS(5),
    [anon_sym_Loyalty] = ACTIONS(5),
    [anon_sym_Oracle] = ACTIONS(5),
    [anon_sym_Text] = ACTIONS(5),
    [anon_sym_FlavorName] = ACTIONS(5),
    [anon_sym_Lights] = ACTIONS(5),
    [anon_sym_MeldPair] = ACTIONS(5),
    [anon_sym_Draft] = ACTIONS(5),
    [anon_sym_Variant] = ACTIONS(5),
    [anon_sym_AlternateMode] = ACTIONS(7),
    [aux_sym_specialize_line_token1] = ACTIONS(9),
    [anon_sym_AI] = ACTIONS(11),
    [anon_sym_DeckHints] = ACTIONS(11),
    [anon_sym_DeckNeeds] = ACTIONS(11),
    [anon_sym_DeckHas] = ACTIONS(11),
    [anon_sym_HandLifeModifier] = ACTIONS(11),
    [anon_sym_A] = ACTIONS(13),
    [anon_sym_T] = ACTIONS(15),
    [anon_sym_S] = ACTIONS(17),
    [anon_sym_R] = ACTIONS(19),
    [anon_sym_SVar] = ACTIONS(21),
    [anon_sym_K] = ACTIONS(23),
    [sym__newline] = ACTIONS(29),
  },
  [3] = {
    [sym__content_line] = STATE(65),
    [sym_comment_line] = STATE(65),
    [sym_field_line] = STATE(65),
    [sym__field_key] = STATE(66),
    [sym_alternate_mode_line] = STATE(65),
    [sym_specialize_line] = STATE(65),
    [sym_ignored_line] = STATE(65),
    [sym__ignored_key] = STATE(38),
    [sym_ability_line] = STATE(65),
    [sym_trigger_line] = STATE(65),
    [sym_static_ability_line] = STATE(65),
    [sym_replacement_line] = STATE(65),
    [sym_svar_line] = STATE(65),
    [sym_keyword_line] = STATE(65),
    [aux_sym_face_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(31),
    [anon_sym_ALTERNATE] = ACTIONS(31),
    [anon_sym_POUND] = ACTIONS(33),
    [anon_sym_Name] = ACTIONS(36),
    [anon_sym_ManaCost] = ACTIONS(36),
    [anon_sym_Types] = ACTIONS(36),
    [anon_sym_PT] = ACTIONS(36),
    [anon_sym_Colors] = ACTIONS(36),
    [anon_sym_Defense] = ACTIONS(36),
    [anon_sym_Loyalty] = ACTIONS(36),
    [anon_sym_Oracle] = ACTIONS(36),
    [anon_sym_Text] = ACTIONS(36),
    [anon_sym_FlavorName] = ACTIONS(36),
    [anon_sym_Lights] = ACTIONS(36),
    [anon_sym_MeldPair] = ACTIONS(36),
    [anon_sym_Draft] = ACTIONS(36),
    [anon_sym_Variant] = ACTIONS(36),
    [anon_sym_AlternateMode] = ACTIONS(39),
    [aux_sym_specialize_line_token1] = ACTIONS(42),
    [anon_sym_AI] = ACTIONS(45),
    [anon_sym_DeckHints] = ACTIONS(45),
    [anon_sym_DeckNeeds] = ACTIONS(45),
    [anon_sym_DeckHas] = ACTIONS(45),
    [anon_sym_HandLifeModifier] = ACTIONS(45),
    [anon_sym_A] = ACTIONS(48),
    [anon_sym_T] = ACTIONS(51),
    [anon_sym_S] = ACTIONS(54),
    [anon_sym_R] = ACTIONS(57),
    [anon_sym_SVar] = ACTIONS(60),
    [anon_sym_K] = ACTIONS(63),
    [sym__newline] = ACTIONS(66),
  },
  [4] = {
    [sym_face] = STATE(27),
    [sym__content_line] = STATE(65),
    [sym_comment_line] = STATE(65),
    [sym_field_line] = STATE(65),
    [sym__field_key] = STATE(66),
    [sym_alternate_mode_line] = STATE(65),
    [sym_specialize_line] = STATE(65),
    [sym_ignored_line] = STATE(65),
    [sym__ignored_key] = STATE(38),
    [sym_ability_line] = STATE(65),
    [sym_trigger_line] = STATE(65),
    [sym_static_ability_line] = STATE(65),
    [sym_replacement_line] = STATE(65),
    [sym_svar_line] = STATE(65),
    [sym_keyword_line] = STATE(65),
    [aux_sym_face_repeat1] = STATE(2),
    [anon_sym_POUND] = ACTIONS(3),
    [anon_sym_Name] = ACTIONS(5),
    [anon_sym_ManaCost] = ACTIONS(5),
    [anon_sym_Types] = ACTIONS(5),
    [anon_sym_PT] = ACTIONS(5),
    [anon_sym_Colors] = ACTIONS(5),
    [anon_sym_Defense] = ACTIONS(5),
    [anon_sym_Loyalty] = ACTIONS(5),
    [anon_sym_Oracle] = ACTIONS(5),
    [anon_sym_Text] = ACTIONS(5),
    [anon_sym_FlavorName] = ACTIONS(5),
    [anon_sym_Lights] = ACTIONS(5),
    [anon_sym_MeldPair] = ACTIONS(5),
    [anon_sym_Draft] = ACTIONS(5),
    [anon_sym_Variant] = ACTIONS(5),
    [anon_sym_AlternateMode] = ACTIONS(7),
    [aux_sym_specialize_line_token1] = ACTIONS(9),
    [anon_sym_AI] = ACTIONS(11),
    [anon_sym_DeckHints] = ACTIONS(11),
    [anon_sym_DeckNeeds] = ACTIONS(11),
    [anon_sym_DeckHas] = ACTIONS(11),
    [anon_sym_HandLifeModifier] = ACTIONS(11),
    [anon_sym_A] = ACTIONS(13),
    [anon_sym_T] = ACTIONS(15),
    [anon_sym_S] = ACTIONS(17),
    [anon_sym_R] = ACTIONS(19),
    [anon_sym_SVar] = ACTIONS(21),
    [anon_sym_K] = ACTIONS(23),
    [sym__newline] = ACTIONS(25),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 2,
    ACTIONS(69), 3,
      anon_sym_A,
      anon_sym_T,
      anon_sym_S,
    ACTIONS(31), 28,
      ts_builtin_sym_end,
      anon_sym_ALTERNATE,
      anon_sym_POUND,
      anon_sym_Name,
      anon_sym_ManaCost,
      anon_sym_Types,
      anon_sym_PT,
      anon_sym_Colors,
      anon_sym_Defense,
      anon_sym_Loyalty,
      anon_sym_Oracle,
      anon_sym_Text,
      anon_sym_FlavorName,
      anon_sym_Lights,
      anon_sym_MeldPair,
      anon_sym_Draft,
      anon_sym_Variant,
      anon_sym_AlternateMode,
      aux_sym_specialize_line_token1,
      anon_sym_AI,
      anon_sym_DeckHints,
      anon_sym_DeckNeeds,
      anon_sym_DeckHas,
      anon_sym_HandLifeModifier,
      anon_sym_R,
      anon_sym_SVar,
      anon_sym_K,
      sym__newline,
  [36] = 2,
    ACTIONS(73), 3,
      anon_sym_A,
      anon_sym_T,
      anon_sym_S,
    ACTIONS(71), 26,
      anon_sym_POUND,
      anon_sym_Name,
      anon_sym_ManaCost,
      anon_sym_Types,
      anon_sym_PT,
      anon_sym_Colors,
      anon_sym_Defense,
      anon_sym_Loyalty,
      anon_sym_Oracle,
      anon_sym_Text,
      anon_sym_FlavorName,
      anon_sym_Lights,
      anon_sym_MeldPair,
      anon_sym_Draft,
      anon_sym_Variant,
      anon_sym_AlternateMode,
      aux_sym_specialize_line_token1,
      anon_sym_AI,
      anon_sym_DeckHints,
      anon_sym_DeckNeeds,
      anon_sym_DeckHas,
      anon_sym_HandLifeModifier,
      anon_sym_R,
      anon_sym_SVar,
      anon_sym_K,
      sym__newline,
  [70] = 3,
    STATE(34), 1,
      sym_ability_record,
    STATE(37), 1,
      sym_ability_body,
    ACTIONS(75), 2,
      anon_sym_SP,
      anon_sym_AB,
  [81] = 4,
    ACTIONS(77), 1,
      ts_builtin_sym_end,
    ACTIONS(79), 1,
      anon_sym_ALTERNATE,
    STATE(4), 1,
      sym_face_separator,
    STATE(10), 1,
      aux_sym_card_script_repeat1,
  [94] = 4,
    ACTIONS(79), 1,
      anon_sym_ALTERNATE,
    ACTIONS(81), 1,
      ts_builtin_sym_end,
    STATE(4), 1,
      sym_face_separator,
    STATE(8), 1,
      aux_sym_card_script_repeat1,
  [107] = 4,
    ACTIONS(83), 1,
      ts_builtin_sym_end,
    ACTIONS(85), 1,
      anon_sym_ALTERNATE,
    STATE(4), 1,
      sym_face_separator,
    STATE(10), 1,
      aux_sym_card_script_repeat1,
  [120] = 3,
    ACTIONS(88), 1,
      anon_sym_SPACE,
    ACTIONS(92), 1,
      sym__param_value,
    ACTIONS(90), 2,
      anon_sym_PIPE,
      sym__newline,
  [131] = 3,
    ACTIONS(94), 1,
      sym__param_key,
    STATE(15), 1,
      sym_param,
    STATE(40), 1,
      sym_param_record,
  [141] = 3,
    ACTIONS(94), 1,
      sym__param_key,
    STATE(15), 1,
      sym_param,
    STATE(42), 1,
      sym_param_record,
  [151] = 3,
    ACTIONS(94), 1,
      sym__param_key,
    STATE(15), 1,
      sym_param,
    STATE(43), 1,
      sym_param_record,
  [161] = 3,
    ACTIONS(96), 1,
      anon_sym_PIPE,
    ACTIONS(98), 1,
      sym__newline,
    STATE(16), 1,
      aux_sym_param_record_repeat1,
  [171] = 3,
    ACTIONS(96), 1,
      anon_sym_PIPE,
    ACTIONS(100), 1,
      sym__newline,
    STATE(17), 1,
      aux_sym_param_record_repeat1,
  [181] = 3,
    ACTIONS(102), 1,
      anon_sym_PIPE,
    ACTIONS(105), 1,
      sym__newline,
    STATE(17), 1,
      aux_sym_param_record_repeat1,
  [191] = 2,
    ACTIONS(109), 1,
      sym__param_value,
    ACTIONS(107), 2,
      anon_sym_PIPE,
      sym__newline,
  [199] = 3,
    ACTIONS(94), 1,
      sym__param_key,
    STATE(15), 1,
      sym_param,
    STATE(67), 1,
      sym_param_record,
  [209] = 3,
    ACTIONS(94), 1,
      sym__param_key,
    STATE(15), 1,
      sym_param,
    STATE(68), 1,
      sym_param_record,
  [219] = 2,
    ACTIONS(111), 1,
      aux_sym_comment_line_token1,
    ACTIONS(113), 1,
      sym__newline,
  [226] = 2,
    ACTIONS(115), 1,
      aux_sym_comment_line_token1,
    ACTIONS(117), 1,
      sym__newline,
  [233] = 2,
    ACTIONS(119), 1,
      aux_sym_comment_line_token1,
    ACTIONS(121), 1,
      sym__newline,
  [240] = 2,
    ACTIONS(123), 1,
      aux_sym_comment_line_token1,
    ACTIONS(125), 1,
      sym__newline,
  [247] = 2,
    ACTIONS(127), 1,
      aux_sym_comment_line_token1,
    ACTIONS(129), 1,
      sym__newline,
  [254] = 2,
    ACTIONS(131), 1,
      anon_sym_SPACE,
    ACTIONS(133), 1,
      aux_sym_ability_body_token1,
  [261] = 1,
    ACTIONS(83), 2,
      ts_builtin_sym_end,
      anon_sym_ALTERNATE,
  [266] = 2,
    ACTIONS(135), 1,
      anon_sym_PIPE,
    ACTIONS(137), 1,
      sym__newline,
  [273] = 1,
    ACTIONS(107), 2,
      anon_sym_PIPE,
      sym__newline,
  [278] = 1,
    ACTIONS(105), 2,
      anon_sym_PIPE,
      sym__newline,
  [283] = 2,
    ACTIONS(94), 1,
      sym__param_key,
    STATE(30), 1,
      sym_param,
  [290] = 2,
    ACTIONS(139), 1,
      anon_sym_PIPE,
    ACTIONS(141), 1,
      sym__newline,
  [297] = 1,
    ACTIONS(143), 2,
      anon_sym_PIPE,
      sym__newline,
  [302] = 1,
    ACTIONS(145), 1,
      anon_sym_DOLLAR,
  [306] = 1,
    ACTIONS(147), 1,
      sym__newline,
  [310] = 1,
    ACTIONS(149), 1,
      anon_sym_DOLLAR,
  [314] = 1,
    ACTIONS(151), 1,
      sym__newline,
  [318] = 1,
    ACTIONS(153), 1,
      anon_sym_COLON,
  [322] = 1,
    ACTIONS(155), 1,
      anon_sym_DOLLAR,
  [326] = 1,
    ACTIONS(157), 1,
      sym__newline,
  [330] = 1,
    ACTIONS(159), 1,
      anon_sym_COLON,
  [334] = 1,
    ACTIONS(161), 1,
      sym__newline,
  [338] = 1,
    ACTIONS(163), 1,
      sym__newline,
  [342] = 1,
    ACTIONS(165), 1,
      anon_sym_COLON,
  [346] = 1,
    ACTIONS(167), 1,
      sym__newline,
  [350] = 1,
    ACTIONS(169), 1,
      sym__newline,
  [354] = 1,
    ACTIONS(171), 1,
      anon_sym_COLON,
  [358] = 1,
    ACTIONS(173), 1,
      anon_sym_COLON,
  [362] = 1,
    ACTIONS(175), 1,
      sym__newline,
  [366] = 1,
    ACTIONS(177), 1,
      sym__newline,
  [370] = 1,
    ACTIONS(179), 1,
      anon_sym_COLON,
  [374] = 1,
    ACTIONS(181), 1,
      anon_sym_COLON,
  [378] = 1,
    ACTIONS(183), 1,
      anon_sym_COLON,
  [382] = 1,
    ACTIONS(185), 1,
      anon_sym_COLON,
  [386] = 1,
    ACTIONS(187), 1,
      aux_sym_svar_line_token2,
  [390] = 1,
    ACTIONS(189), 1,
      aux_sym_ability_body_token1,
  [394] = 1,
    ACTIONS(191), 1,
      aux_sym_svar_line_token1,
  [398] = 1,
    ACTIONS(193), 1,
      aux_sym_svar_line_token2,
  [402] = 1,
    ACTIONS(195), 1,
      sym__newline,
  [406] = 1,
    ACTIONS(197), 1,
      anon_sym_COLON,
  [410] = 1,
    ACTIONS(199), 1,
      anon_sym_COLON,
  [414] = 1,
    ACTIONS(201), 1,
      sym__newline,
  [418] = 1,
    ACTIONS(203), 1,
      ts_builtin_sym_end,
  [422] = 1,
    ACTIONS(205), 1,
      anon_sym_COLON,
  [426] = 1,
    ACTIONS(207), 1,
      sym__newline,
  [430] = 1,
    ACTIONS(209), 1,
      anon_sym_COLON,
  [434] = 1,
    ACTIONS(211), 1,
      sym__newline,
  [438] = 1,
    ACTIONS(213), 1,
      sym__newline,
  [442] = 1,
    ACTIONS(215), 1,
      sym__newline,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(5)] = 0,
  [SMALL_STATE(6)] = 36,
  [SMALL_STATE(7)] = 70,
  [SMALL_STATE(8)] = 81,
  [SMALL_STATE(9)] = 94,
  [SMALL_STATE(10)] = 107,
  [SMALL_STATE(11)] = 120,
  [SMALL_STATE(12)] = 131,
  [SMALL_STATE(13)] = 141,
  [SMALL_STATE(14)] = 151,
  [SMALL_STATE(15)] = 161,
  [SMALL_STATE(16)] = 171,
  [SMALL_STATE(17)] = 181,
  [SMALL_STATE(18)] = 191,
  [SMALL_STATE(19)] = 199,
  [SMALL_STATE(20)] = 209,
  [SMALL_STATE(21)] = 219,
  [SMALL_STATE(22)] = 226,
  [SMALL_STATE(23)] = 233,
  [SMALL_STATE(24)] = 240,
  [SMALL_STATE(25)] = 247,
  [SMALL_STATE(26)] = 254,
  [SMALL_STATE(27)] = 261,
  [SMALL_STATE(28)] = 266,
  [SMALL_STATE(29)] = 273,
  [SMALL_STATE(30)] = 278,
  [SMALL_STATE(31)] = 283,
  [SMALL_STATE(32)] = 290,
  [SMALL_STATE(33)] = 297,
  [SMALL_STATE(34)] = 302,
  [SMALL_STATE(35)] = 306,
  [SMALL_STATE(36)] = 310,
  [SMALL_STATE(37)] = 314,
  [SMALL_STATE(38)] = 318,
  [SMALL_STATE(39)] = 322,
  [SMALL_STATE(40)] = 326,
  [SMALL_STATE(41)] = 330,
  [SMALL_STATE(42)] = 334,
  [SMALL_STATE(43)] = 338,
  [SMALL_STATE(44)] = 342,
  [SMALL_STATE(45)] = 346,
  [SMALL_STATE(46)] = 350,
  [SMALL_STATE(47)] = 354,
  [SMALL_STATE(48)] = 358,
  [SMALL_STATE(49)] = 362,
  [SMALL_STATE(50)] = 366,
  [SMALL_STATE(51)] = 370,
  [SMALL_STATE(52)] = 374,
  [SMALL_STATE(53)] = 378,
  [SMALL_STATE(54)] = 382,
  [SMALL_STATE(55)] = 386,
  [SMALL_STATE(56)] = 390,
  [SMALL_STATE(57)] = 394,
  [SMALL_STATE(58)] = 398,
  [SMALL_STATE(59)] = 402,
  [SMALL_STATE(60)] = 406,
  [SMALL_STATE(61)] = 410,
  [SMALL_STATE(62)] = 414,
  [SMALL_STATE(63)] = 418,
  [SMALL_STATE(64)] = 422,
  [SMALL_STATE(65)] = 426,
  [SMALL_STATE(66)] = 430,
  [SMALL_STATE(67)] = 434,
  [SMALL_STATE(68)] = 438,
  [SMALL_STATE(69)] = 442,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, SHIFT(22),
  [5] = {.entry = {.count = 1, .reusable = true}}, SHIFT(64),
  [7] = {.entry = {.count = 1, .reusable = true}}, SHIFT(41),
  [9] = {.entry = {.count = 1, .reusable = true}}, SHIFT(47),
  [11] = {.entry = {.count = 1, .reusable = true}}, SHIFT(48),
  [13] = {.entry = {.count = 1, .reusable = false}}, SHIFT(51),
  [15] = {.entry = {.count = 1, .reusable = false}}, SHIFT(52),
  [17] = {.entry = {.count = 1, .reusable = false}}, SHIFT(53),
  [19] = {.entry = {.count = 1, .reusable = true}}, SHIFT(54),
  [21] = {.entry = {.count = 1, .reusable = true}}, SHIFT(60),
  [23] = {.entry = {.count = 1, .reusable = true}}, SHIFT(61),
  [25] = {.entry = {.count = 1, .reusable = true}}, SHIFT(2),
  [27] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_face, 1, 0, 0),
  [29] = {.entry = {.count = 1, .reusable = true}}, SHIFT(3),
  [31] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0),
  [33] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(22),
  [36] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(64),
  [39] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(41),
  [42] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(47),
  [45] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(48),
  [48] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(51),
  [51] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(52),
  [54] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(53),
  [57] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(54),
  [60] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(60),
  [63] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(61),
  [66] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0), SHIFT_REPEAT(3),
  [69] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym_face_repeat1, 2, 0, 0),
  [71] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_face_separator, 2, 0, 0),
  [73] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_face_separator, 2, 0, 0),
  [75] = {.entry = {.count = 1, .reusable = true}}, SHIFT(36),
  [77] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_card_script, 2, 0, 0),
  [79] = {.entry = {.count = 1, .reusable = true}}, SHIFT(59),
  [81] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_card_script, 1, 0, 0),
  [83] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_card_script_repeat1, 2, 0, 0),
  [85] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_card_script_repeat1, 2, 0, 0), SHIFT_REPEAT(59),
  [88] = {.entry = {.count = 1, .reusable = false}}, SHIFT(18),
  [90] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_param, 2, 0, 0),
  [92] = {.entry = {.count = 1, .reusable = false}}, SHIFT(29),
  [94] = {.entry = {.count = 1, .reusable = true}}, SHIFT(39),
  [96] = {.entry = {.count = 1, .reusable = true}}, SHIFT(31),
  [98] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_param_record, 1, 0, 0),
  [100] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_param_record, 2, 0, 0),
  [102] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_param_record_repeat1, 2, 0, 0), SHIFT_REPEAT(31),
  [105] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_param_record_repeat1, 2, 0, 0),
  [107] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_param, 3, 0, 0),
  [109] = {.entry = {.count = 1, .reusable = false}}, SHIFT(33),
  [111] = {.entry = {.count = 1, .reusable = false}}, SHIFT(35),
  [113] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_specialize_line, 2, 0, 0),
  [115] = {.entry = {.count = 1, .reusable = false}}, SHIFT(46),
  [117] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_comment_line, 1, 0, 0),
  [119] = {.entry = {.count = 1, .reusable = false}}, SHIFT(50),
  [121] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_alternate_mode_line, 2, 0, 0),
  [123] = {.entry = {.count = 1, .reusable = false}}, SHIFT(49),
  [125] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_field_line, 2, 0, 0),
  [127] = {.entry = {.count = 1, .reusable = false}}, SHIFT(69),
  [129] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_ignored_line, 2, 0, 0),
  [131] = {.entry = {.count = 1, .reusable = false}}, SHIFT(56),
  [133] = {.entry = {.count = 1, .reusable = false}}, SHIFT(28),
  [135] = {.entry = {.count = 1, .reusable = true}}, SHIFT(19),
  [137] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_ability_body, 3, 0, 0),
  [139] = {.entry = {.count = 1, .reusable = true}}, SHIFT(20),
  [141] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_ability_body, 4, 0, 0),
  [143] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_param, 4, 0, 0),
  [145] = {.entry = {.count = 1, .reusable = true}}, SHIFT(26),
  [147] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_specialize_line, 3, 0, 0),
  [149] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_ability_record, 1, 0, 0),
  [151] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_ability_line, 3, 0, 0),
  [153] = {.entry = {.count = 1, .reusable = true}}, SHIFT(25),
  [155] = {.entry = {.count = 1, .reusable = true}}, SHIFT(11),
  [157] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_trigger_line, 3, 0, 0),
  [159] = {.entry = {.count = 1, .reusable = true}}, SHIFT(23),
  [161] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_static_ability_line, 3, 0, 0),
  [163] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_replacement_line, 3, 0, 0),
  [165] = {.entry = {.count = 1, .reusable = true}}, SHIFT(55),
  [167] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_keyword_line, 3, 0, 2),
  [169] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_comment_line, 2, 0, 1),
  [171] = {.entry = {.count = 1, .reusable = true}}, SHIFT(21),
  [173] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__ignored_key, 1, 0, 0),
  [175] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_field_line, 3, 0, 0),
  [177] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_alternate_mode_line, 3, 0, 0),
  [179] = {.entry = {.count = 1, .reusable = true}}, SHIFT(7),
  [181] = {.entry = {.count = 1, .reusable = true}}, SHIFT(12),
  [183] = {.entry = {.count = 1, .reusable = true}}, SHIFT(13),
  [185] = {.entry = {.count = 1, .reusable = true}}, SHIFT(14),
  [187] = {.entry = {.count = 1, .reusable = true}}, SHIFT(62),
  [189] = {.entry = {.count = 1, .reusable = true}}, SHIFT(32),
  [191] = {.entry = {.count = 1, .reusable = true}}, SHIFT(44),
  [193] = {.entry = {.count = 1, .reusable = true}}, SHIFT(45),
  [195] = {.entry = {.count = 1, .reusable = true}}, SHIFT(6),
  [197] = {.entry = {.count = 1, .reusable = true}}, SHIFT(57),
  [199] = {.entry = {.count = 1, .reusable = true}}, SHIFT(58),
  [201] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_svar_line, 5, 0, 0),
  [203] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [205] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__field_key, 1, 0, 0),
  [207] = {.entry = {.count = 1, .reusable = true}}, SHIFT(5),
  [209] = {.entry = {.count = 1, .reusable = true}}, SHIFT(24),
  [211] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_ability_body, 5, 0, 0),
  [213] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_ability_body, 6, 0, 0),
  [215] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_ignored_line, 3, 0, 0),
};

#ifdef __cplusplus
extern "C" {
#endif
#ifdef TREE_SITTER_HIDE_SYMBOLS
#define TS_PUBLIC
#elif defined(_WIN32)
#define TS_PUBLIC __declspec(dllexport)
#else
#define TS_PUBLIC __attribute__((visibility("default")))
#endif

TS_PUBLIC const TSLanguage *tree_sitter_forge_card_script(void) {
  static const TSLanguage language = {
    .version = LANGUAGE_VERSION,
    .symbol_count = SYMBOL_COUNT,
    .alias_count = ALIAS_COUNT,
    .token_count = TOKEN_COUNT,
    .external_token_count = EXTERNAL_TOKEN_COUNT,
    .state_count = STATE_COUNT,
    .large_state_count = LARGE_STATE_COUNT,
    .production_id_count = PRODUCTION_ID_COUNT,
    .field_count = FIELD_COUNT,
    .max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,
    .parse_table = &ts_parse_table[0][0],
    .small_parse_table = ts_small_parse_table,
    .small_parse_table_map = ts_small_parse_table_map,
    .parse_actions = ts_parse_actions,
    .symbol_names = ts_symbol_names,
    .symbol_metadata = ts_symbol_metadata,
    .public_symbol_map = ts_symbol_map,
    .alias_map = ts_non_terminal_alias_map,
    .alias_sequences = &ts_alias_sequences[0][0],
    .lex_modes = ts_lex_modes,
    .lex_fn = ts_lex,
    .primary_state_ids = ts_primary_state_ids,
  };
  return &language;
}
#ifdef __cplusplus
}
#endif
