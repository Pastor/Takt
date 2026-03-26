/**
 * Tree-sitter grammar for the BuT FSM language.
 * BuT — DSL for finite-state machine specification.
 *
 * Operator precedence (matches grammar.lalrpop Precedence0-14):
 *   14  assignment        =       right
 *   13  logical or        ||      left
 *   12  logical and       &&      left
 *   11  equality          == !=   left
 *   10  relational        < > <= >=  left
 *    9  bitwise or        |       left
 *    8  bitwise xor       ^       left
 *    7  bitwise and  &    and cast  as   left
 *    6  shift             << >>   left
 *    5  additive          + -     left
 *    4  multiplicative    * / %   left
 *    3  power             **      right
 *    2  unary             ! ~ + - prefix
 *    1  postfix           . [] () member/call/subscript
 */

const PREC = {
  ASSIGN:  1,
  OR:      2,
  AND:     3,
  EQUAL:   4,
  COMPARE: 5,
  BIT_OR:  6,
  BIT_XOR: 7,
  BIT_AND: 8,
  CAST:    8,
  SHIFT:   9,
  ADD:     10,
  MUL:     11,
  POWER:   12,
  UNARY:   13,
  POSTFIX: 14,
};

module.exports = grammar({
  name: 'but',

  extras: $ => [
    /\s+/,
    $.line_comment,
  ],

  // Inline hidden rules for cleaner CST
  inline: $ => [
    $._top_level_item,
    $._model_element,
    $._state_element,
    $._statement,
    $._type,
  ],

  conflicts: $ => [],

  rules: {

    // ── Entry point ─────────────────────────────────────────────────────────
    source_file: $ => repeat($._top_level_item),

    // ── Comments ────────────────────────────────────────────────────────────
    line_comment: _ => token(choice(
      seq('///', /[^\n]*/),   // doc comment
      seq('//',  /[^\n]*/),   // regular comment
    )),

    // ── Top-level items ─────────────────────────────────────────────────────
    // SourceUnit shares the same element set as ModelElement (see grammar.lalrpop).
    _top_level_item: $ => choice(
      $.import_declaration,
      $.type_alias,
      $.variable_declaration,
      $.function_declaration,
      $.cond_declaration,
      $.model_declaration,
      $.state_declaration,
      $.formula_block,
      $.named_block,
      $.stray_semicolon,
    ),

    // ── Import ───────────────────────────────────────────────────────────────
    import_declaration: $ => choice(
      // import "path.but";
      seq('import', field('path', $.string_literal), ';'),
      // import "path.but" as Name;
      seq('import', field('path', $.string_literal), 'as', field('alias', $.identifier), ';'),
      // import * as Name from "path.but";
      seq('import', '*', 'as', field('alias', $.identifier),
          field('from_kw', $.identifier), field('path', $.string_literal), ';'),
      // import { A, B as C } from "path.but";
      seq('import', '{', commaSep1($.import_specifier), '}',
          field('from_kw', $.identifier), field('path', $.string_literal), ';'),
    ),

    import_specifier: $ => choice(
      field('name', $.identifier),
      seq(field('name', $.identifier), 'as', field('alias', $.identifier)),
    ),

    // ── Type alias ───────────────────────────────────────────────────────────
    type_alias: $ => seq(
      'type', field('name', $.identifier), '=', field('type', $._type), ';'
    ),

    // ── Types ────────────────────────────────────────────────────────────────
    _type: $ => choice(
      $.primitive_type,
      $.array_type,
      $.type_name,
    ),

    primitive_type: _ => 'bit',

    // [ElementType; Count]
    array_type: $ => seq(
      '[', field('element', $._type), ';', field('size', $.integer_literal), ']'
    ),

    // Named type (alias reference)
    type_name: $ => $.identifier,

    // ── Variable declarations ─────────────────────────────────────────────────
    variable_declaration: $ => choice(
      $.var_decl,
      $.const_decl,
      $.port_decl,
    ),

    var_decl: $ => seq(
      'var', field('name', $.identifier),
      optional(seq(':', field('type', $._type))),
      optional(seq('=', field('value', $._expression))),
      ';'
    ),

    const_decl: $ => seq(
      'const', field('name', $.identifier),
      optional(seq(':', field('type', $._type))),
      '=', field('value', $._expression), ';'
    ),

    port_decl: $ => seq(
      'port', field('name', $.identifier),
      optional(seq(':', field('type', $._type))),
      optional(seq('=', field('address', $._expression))),
      ';'
    ),

    // ── Function declaration ──────────────────────────────────────────────────
    function_declaration: $ => seq(
      optional(field('extern_kw', 'extern')),
      'fn', field('name', $.identifier),
      '(', field('params', commaSep($.parameter)), ')',
      optional(seq('->', field('return_type', $._type))),
      field('body', choice($.block, ';'))
    ),

    parameter: $ => seq(
      optional(seq(field('name', $.identifier), ':')),
      field('type', $._type)
    ),

    // ── Condition declaration ─────────────────────────────────────────────────
    cond_declaration: $ => seq(
      'cond', field('name', $.identifier), '=', field('condition', $._condition), ';'
    ),

    // ── Model declaration ─────────────────────────────────────────────────────
    model_declaration: $ => seq(
      'model', field('name', $.identifier),
      optional(seq('=', field('implements', $._composition))),
      '{', repeat($._model_element), '}'
    ),

    // Model/state composition expressions: A + B (sequential) | A | B (parallel)
    // Uses the full expression since implements clause is an Expression in LALRPOP.
    _composition: $ => choice(
      $.identifier,
      prec.left(1, seq($._composition, '+', $._composition)),
      prec.left(1, seq($._composition, '|', $._composition)),
      seq('(', $._composition, ')'),
    ),

    _model_element: $ => choice(
      $.import_declaration,
      $.type_alias,
      $.variable_declaration,
      $.function_declaration,
      $.cond_declaration,
      $.model_declaration,
      $.state_declaration,
      $.formula_block,
      $.named_block,
      $.stray_semicolon,
    ),

    // ── State declaration ─────────────────────────────────────────────────────
    state_declaration: $ => seq(
      field('kind', choice('state', 'start')),
      field('name', $.identifier),
      optional(seq('=', field('implements', $._composition))),
      choice(
        seq('{', repeat($._state_element), '}'),
        ';',
      )
    ),

    _state_element: $ => choice(
      $.next_transition,
      $.ref_transition,
      $.named_block,
      $.stray_semicolon,
    ),

    next_transition: $ => seq('next', field('target', $.identifier), ';'),

    ref_transition: $ => seq(
      'ref', field('target', $.identifier),
      optional(seq(':', field('condition', $._condition))),
      ';'
    ),

    // ── Named blocks (enter/exit/always/custom) ───────────────────────────────
    // A named block is: identifier { statements... }
    named_block: $ => seq(
      field('name', $.identifier),
      field('body', $.block)
    ),

    // ── Statements ────────────────────────────────────────────────────────────
    block: $ => seq('{', repeat($._statement), '}'),

    _statement: $ => choice(
      $.block,
      $.variable_declaration,
      $.if_statement,
      $.loop_statement,
      $.for_statement,
      $.return_statement,
      $.break_statement,
      $.continue_statement,
      $.assembly_block,
      $.formula_block,
      $.expression_statement,
      $.stray_semicolon,
    ),

    // NOTE: BuT `if` использует условие без скобок
    if_statement: $ => prec.right(seq(
      'if', field('condition', $._expression),
      field('consequence', $.block),
      optional(seq('else', field('alternative', choice($.block, $.if_statement))))
    )),

    // loop без условия — бесконечный цикл; loop условие { } — условный цикл.
    // do-while удалён из языка.
    loop_statement: $ => choice(
      // Бесконечный цикл: loop { тело }
      seq('loop', field('body', $.block)),
      // Цикл с условием: loop условие { тело }
      seq('loop', field('condition', $._expression), field('body', $.block)),
    ),

    for_statement: $ => seq(
      'for',
      optional(field('init', $.simple_statement)), ';',
      optional(field('condition', $._expression)), ';',
      optional(field('update', $._expression)),
      field('body', $.block)
    ),

    // SimpleStatement inside for-init: variable or expression
    simple_statement: $ => choice(
      $.variable_declaration,
      $._expression,
    ),

    return_statement: $ => seq('return', optional(field('value', $._expression)), ';'),
    break_statement:    _ => seq('break', ';'),
    continue_statement: _ => seq('continue', ';'),
    stray_semicolon:    _ => ';',

    // assembly "dialect"? { statements }
    assembly_block: $ => seq(
      'assembly',
      optional(field('dialect', $.string_literal)),
      field('body', $.block)
    ),

    // formula "dialect"? { formula_statements }
    formula_block: $ => seq(
      'formula',
      optional(field('dialect', $.string_literal)),
      '{', repeat($.formula_statement), '}'
    ),

    formula_statement: $ => choice(
      $.formula_block,
      $.formula_function_call,
    ),

    formula_function_call: $ => seq(
      field('name', $.formula_identifier),
      '(', commaSep($.formula_expression), ')', optional(';')
    ),

    formula_expression: $ => choice(
      $.formula_identifier,
      prec.left(1, seq($.formula_expression, '.', $.formula_identifier)),
      seq($.formula_function_call),
      $.true_literal,
      $.false_literal,
      $.integer_literal,
      $.string_literal,
    ),

    formula_identifier: _ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    expression_statement: $ => seq($._expression, ';'),

    // ── Conditions (used in ref/cond, `=` means equality here) ───────────────
    _condition: $ => choice(
      prec.left(PREC.OR,      seq($._condition, '||', $._condition)),
      prec.left(PREC.AND,     seq($._condition, '&',  $._condition)),
      // In conditions: single `=` is equality, `!=` is not-equal
      prec.left(PREC.EQUAL,   seq($._condition, '=',  $._condition)),
      prec.left(PREC.EQUAL,   seq($._condition, '!=', $._condition)),
      prec.left(PREC.COMPARE, seq($._condition, '<',  $._condition)),
      prec.left(PREC.COMPARE, seq($._condition, '>',  $._condition)),
      prec.left(PREC.COMPARE, seq($._condition, '<=', $._condition)),
      prec.left(PREC.COMPARE, seq($._condition, '>=', $._condition)),
      prec.left(PREC.ADD,     seq($._condition, '+',  $._condition)),
      prec.left(PREC.ADD,     seq($._condition, '-',  $._condition)),
      prec.left(PREC.BIT_OR,  seq($._condition, '|',  $._condition)),
      prec(PREC.UNARY,        seq('!', $._condition)),
      // Postfix in conditions: member access and array subscript
      prec.left(PREC.POSTFIX, seq($._condition, '.', choice($.identifier, $.integer_literal))),
      prec.left(PREC.POSTFIX, seq($.identifier, '[', $.integer_literal, ']')),
      // Function call in conditions
      seq($.identifier, '(', commaSep($._condition), ')'),
      $.true_literal,
      $.false_literal,
      $.integer_literal,
      $.float_literal,
      $.identifier,
      seq('(', $._condition, ')'),
    ),

    // ── Expressions ───────────────────────────────────────────────────────────
    _expression: $ => choice(
      // Assignment (right-associative)
      prec.right(PREC.ASSIGN,  seq($._expression, '=',   $._expression)),
      // Logical
      prec.left(PREC.OR,       seq($._expression, '||',  $._expression)),
      prec.left(PREC.AND,      seq($._expression, '&&',  $._expression)),
      // Equality (uses == in expressions, unlike conditions which use =)
      prec.left(PREC.EQUAL,    seq($._expression, '==',  $._expression)),
      prec.left(PREC.EQUAL,    seq($._expression, '!=',  $._expression)),
      // Relational
      prec.left(PREC.COMPARE,  seq($._expression, '<',   $._expression)),
      prec.left(PREC.COMPARE,  seq($._expression, '>',   $._expression)),
      prec.left(PREC.COMPARE,  seq($._expression, '<=',  $._expression)),
      prec.left(PREC.COMPARE,  seq($._expression, '>=',  $._expression)),
      // Bitwise
      prec.left(PREC.BIT_OR,   seq($._expression, '|',   $._expression)),
      prec.left(PREC.BIT_XOR,  seq($._expression, '^',   $._expression)),
      prec.left(PREC.BIT_AND,  seq($._expression, '&',   $._expression)),
      // Cast (same precedence as bitwise and)
      prec.left(PREC.CAST,     seq($._expression, 'as',  $._type)),
      // Shift
      prec.left(PREC.SHIFT,    seq($._expression, '<<',  $._expression)),
      prec.left(PREC.SHIFT,    seq($._expression, '>>',  $._expression)),
      // Arithmetic
      prec.left(PREC.ADD,      seq($._expression, '+',   $._expression)),
      prec.left(PREC.ADD,      seq($._expression, '-',   $._expression)),
      prec.left(PREC.MUL,      seq($._expression, '*',   $._expression)),
      prec.left(PREC.MUL,      seq($._expression, '/',   $._expression)),
      prec.left(PREC.MUL,      seq($._expression, '%',   $._expression)),
      // Power (right-associative)
      prec.right(PREC.POWER,   seq($._expression, '**',  $._expression)),
      // Unary prefix
      prec(PREC.UNARY,         seq('!',  $._expression)),
      prec(PREC.UNARY,         seq('~',  $._expression)),
      prec(PREC.UNARY,         seq('+',  $._expression)),
      prec(PREC.UNARY,         seq('-',  $._expression)),
      // Postfix: bit/member access  expr.member  or  expr.0
      prec.left(PREC.POSTFIX,  seq($._expression, '.', choice($.identifier, $.integer_literal))),
      // Array subscript (LALRPOP: identifier only):  ident[n]
      prec.left(PREC.POSTFIX,  seq($.identifier, '[', $.integer_literal, ']')),
      // Array slice (LALRPOP: identifier only):  ident[lo?:hi?]
      prec.left(PREC.POSTFIX,  seq(
        $.identifier, '[',
        optional($.integer_literal), ':',
        optional($.integer_literal),
        ']'
      )),
      // Function call:  expr(args)
      prec.left(PREC.POSTFIX,  seq($._expression, '(', commaSep($._expression), ')')),
      // Terminals
      $.primary_expression,
    ),

    primary_expression: $ => choice(
      $.true_literal,
      $.false_literal,
      // address_literal MUST be tried before integer_literal because both start with a number.
      // Precedence ensures address_literal wins when followed by `:`.
      prec(1, $.address_literal),
      $.integer_literal,
      $.float_literal,
      $.string_literal,
      $.array_initializer,
      $.identifier_path,
      seq('(', $._expression, ')'),
    ),

    // { e1, e2, ... }  — array/struct initializer
    array_initializer: $ => seq('{', commaSep1($._expression), '}'),

    // 0x00100000:0  — hardware address with bit index
    // The `prec(1, ...)` in primary_expression ensures this wins over bare integer_literal.
    address_literal: $ => seq($.integer_literal, ':', $.integer_literal),

    // A::B::C  — namespace path (two `:` tokens, not `::`)
    identifier_path: $ => seq(
      $.identifier,
      repeat(seq(':', ':', $.identifier))
    ),

    // ── Terminals ─────────────────────────────────────────────────────────────
    true_literal:  _ => 'true',
    false_literal: _ => 'false',

    integer_literal: _ => token(choice(
      /0[xX][0-9a-fA-F][0-9a-fA-F_]*/,   // hex
      /[0-9][0-9_]*/,                      // decimal
    )),

    float_literal: _ => /[0-9][0-9_]*\.[0-9][0-9_]*/,

    string_literal: $ => seq(
      optional('unicode'),
      choice(
        seq('"', repeat(choice($.escape_sequence, /[^"\\]+/)), '"'),
        seq("'", repeat(choice($.escape_sequence, /[^'\\]+/)), "'"),
      )
    ),

    escape_sequence: _ => /\\[ntr\\'"/\\0]/,

    identifier: _ => /[a-zA-Z_][a-zA-Z0-9_]*/,
  },
});

// ── Helper combinators ───────────────────────────────────────────────────────

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)), optional(','));
}

function commaSep(rule) {
  return optional(commaSep1(rule));
}
