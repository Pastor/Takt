; ── BuT syntax highlighting queries ─────────────────────────────────────────
; Maps Tree-sitter node types to Zed theme scopes.

; ── Comments ─────────────────────────────────────────────────────────────────
(line_comment) @comment
; Doc comments (/// ...) — captured by same rule but distinguished by prefix
((line_comment) @comment.doc
 (#match? @comment.doc "^///"))

; ── Keywords: structure ───────────────────────────────────────────────────────
"model"    @keyword
"state"    @keyword
"start"    @keyword
"cond"     @keyword
"ref"      @keyword
"next"     @keyword

; ── Keywords: declarations ───────────────────────────────────────────────────
"var"      @keyword
"const"    @keyword
"port"     @keyword
"type"     @keyword
"fn"       @keyword.function
"extern"   @keyword

; ── Keywords: import ─────────────────────────────────────────────────────────
"import"   @keyword.control.import
"as"       @keyword.operator

; ── Keywords: control flow ───────────────────────────────────────────────────
"if"       @keyword.control
"else"     @keyword.control
"while"    @keyword.control
"do"       @keyword.control
"for"      @keyword.control
"return"   @keyword.control.return
"break"    @keyword.control
"continue" @keyword.control

; ── Keywords: embedded blocks ────────────────────────────────────────────────
"assembly" @keyword
"formula"  @keyword

; ── Types ────────────────────────────────────────────────────────────────────
(primitive_type) @type.builtin
(type_name)      @type
(array_type "[" @punctuation.bracket "]" @punctuation.bracket)

; Type definitions
(type_alias name: (identifier) @type.definition)
(model_declaration name: (identifier) @type.definition)

; ── Function definitions & calls ─────────────────────────────────────────────
(function_declaration name: (identifier) @function)

; Function calls (postfix call: expr(...))
; The callee is the left side of the call postfix, captured as identifier path
(primary_expression
  (identifier_path (identifier) @function.call))

; ── Variables & constants ────────────────────────────────────────────────────
(var_decl   name: (identifier) @variable)
(const_decl name: (identifier) @constant)
(port_decl  name: (identifier) @variable.special)

(parameter  name: (identifier) @variable.parameter)

; ── State names & labels ─────────────────────────────────────────────────────
(state_declaration name: (identifier) @label)
(next_transition   target: (identifier) @label)
(ref_transition    target: (identifier) @label)

; ── Condition names ──────────────────────────────────────────────────────────
(cond_declaration name: (identifier) @constant.definition)

; ── Named blocks: highlight well-known names ─────────────────────────────────
(named_block name: (identifier) @keyword
 (#any-of? @keyword "enter" "exit" "always"))

; Other named block names get a generic label highlight
(named_block name: (identifier) @label)

; ── Literals ─────────────────────────────────────────────────────────────────
(true_literal)  @boolean
(false_literal) @boolean

(integer_literal)   @number
(float_literal)     @number
(address_literal)   @number

(string_literal)    @string
(escape_sequence)   @string.escape

(array_initializer
  "{" @punctuation.bracket
  "}" @punctuation.bracket)

; ── Operators ────────────────────────────────────────────────────────────────
[
  "+"  "-"  "*"  "/"  "%"  "**"
  "&"  "|"  "^"  "~"  "<<" ">>"
  "!"  "&&" "||"
  "="  "==" "!=" "<"  ">"  "<=" ">="
] @operator

; ── Punctuation ──────────────────────────────────────────────────────────────
["{" "}"] @punctuation.bracket
["(" ")"] @punctuation.bracket
["[" "]"] @punctuation.bracket

[";" ","] @punctuation.delimiter
"."       @punctuation.delimiter
":"       @punctuation.delimiter
"->"      @punctuation.special
