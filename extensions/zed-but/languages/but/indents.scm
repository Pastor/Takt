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

; if/else/while/for/do-while
[
  (if_statement (block "}" @end))
  (while_statement (block "}" @end))
  (do_while_statement (block "}" @end))
  (for_statement (block "}" @end))
] @indent
