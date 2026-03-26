; ── BuT indentation rules ────────────────────────────────────────────────────
; Blocks: indent their contents, dedent on closing brace

[
  (block
    "}" @end)
  (model_declaration
    "}" @end)
  (state_declaration
    "}" @end)
  (formula_block
    "}" @end)
  (assembly_block
    (block "}" @end))
  (array_initializer
    "}" @end)
] @indent

; Named block bodies
(named_block
  (block "}" @end)) @indent

; if/else/loop/for (do-while удалён из языка)
[
  (if_statement (block "}" @end))
  (loop_statement (block "}" @end))
  (for_statement (block "}" @end))
] @indent
