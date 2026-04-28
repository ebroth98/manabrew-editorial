; Line type prefixes
(ability_line "A" @keyword)
(trigger_line "T" @keyword)
(static_ability_line "S" @keyword)
(replacement_line "R" @keyword)
(keyword_line "K" @keyword)

; Ability records
(ability_record) @type.builtin

; API names
(ability_body (api_name) @function)

; Param keys and values
(param (param_key) @property)
(param (param_value) @string)

; Field lines
(field_line (key) @attribute)
(field_line (value) @string)

; SVar
(svar_line "SVar" @keyword)
(svar_line (svar_name) @variable)
(svar_line (svar_value) @string)

; Keywords
(keyword_line (keyword_value) @string.special)

; Alternate face
(face_separator (alternate_keyword) @keyword.control)
(alternate_mode_line "AlternateMode" @attribute)
(alternate_mode_line (value) @string)

; Specialize
(specialize_line) @keyword

; Ignored lines
(ignored_line (key) @comment)
(ignored_line (value) @comment)

; Comments
(comment_line) @comment
(comment_line (comment_text) @comment)

; Delimiters
":" @punctuation.delimiter
"$" @punctuation.special
"|" @punctuation.separator
