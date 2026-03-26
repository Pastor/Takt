; ── BuT outline queries ───────────────────────────────────────────────────────
; These nodes appear in the file outline / symbol panel in Zed.

; Model declarations
(model_declaration
  name: (identifier) @name) @item

; State declarations
(state_declaration
  kind: _ @context
  name: (identifier) @name) @item

; Function declarations
(function_declaration
  name: (identifier) @name) @item

; Type aliases
(type_alias
  name: (identifier) @name) @item

; Condition declarations
(cond_declaration
  name: (identifier) @name) @item

; Named blocks (enter/exit/always shown in outline)
(named_block
  name: (identifier) @name) @item
