# Грамматика (EBNF)

Формальный синтаксис языка Takt в нотации **ISO EBNF** (ISO/IEC 14977). Это
**нормативный** источник по структуре программы: текстовые разделы описывают смысл
конструкций, а грамматика — их допустимую форму.

## Обозначения

| Запись | Значение |
|---|---|
| `rule = … ;` | определение правила |
| `a b` | конкатенация (последовательность) |
| `a \| b` | альтернатива |
| `[ a ]` | опционально (0 или 1 раз) |
| `{ a }` | повторение (0 и более раз) |
| `( a \| b )` | группировка |
| `"текст"` | терминал (литерал) |
| `lowercase` | нетерминал |

## Точка входа

Программа — последовательность элементов верхнего уровня.

```ebnf
source_unit = { model_element } ;
```

## Элементы верхнего уровня и состояния

`model_element` допустим на верхнем уровне и внутри модели; `state_element` — только
в теле состояния (там лишь управление автоматом, без объявлений).

```ebnf
model_element
    = import_define | type_define | variable_define | function_define
    | model | state_define | formula_define | inline_formula_define
    | condition_define | named_block | enum_define | struct_define | ";" ;

state_element
    = named_block
    | "next" identifier ";"
    | "ref" identifier [ ":" condition ] ";"
    | inline_formula_define
    | ";" ;
```

## Импорт

```ebnf
import_define
    = "import" import_path ";"
    | "import" import_path "as" identifier ";"
    | "import" "*" "as" identifier "from" import_path ";"
    | "import" "{" import_rename { "," import_rename } "}" "from" import_path ";" ;

import_path     = string_literal | identifier_path ;
import_rename   = identifier | identifier "as" identifier ;
identifier_path = identifier { "::" identifier } ;
```

## Типы

```ebnf
type_define = "type" identifier "=" type ";" ;

type = identifier                      (* bit, u8, MyType *)
     | "[" type ";" integer "]" ;      (* массив: [bit;8] *)
```

## Переменные, константы, порты

Инициализатор — через `:=`. `const` требует его; инициализатор `in`/`out` задаёт
адрес порта (`0xADDR` или `0xADDR:бит`).

```ebnf
variable_define
    = "var"   identifier [ ":" type ] [ ":=" expression ] ";"
    | "const" identifier [ ":" type ] ":=" expression ";"
    | "in"    identifier [ ":" type ] [ ":=" expression ] ";"
    | "out"   identifier [ ":" type ] [ ":=" expression ] ";"
    | "inout" identifier [ ":" type ] [ ":=" expression ] ";" ;
```

## Функции

```ebnf
function_define
    = [ "extern" ] "fn" identifier parameter_list [ "->" type ]
      ( ";" | block_statement ) ;

parameter_list      = "(" [ parameter { "," parameter } ] ")" ;
parameter           = identifier ":" parameter_type_expr | parameter_type_expr ;
parameter_type_expr = "[" type ";" integer "]" | expression ;
```

`extern fn` — объявление без тела; `fn` — определение с телом.

## Модель, состояние, перечисление, структура

```ebnf
model = "model" identifier [ "=" expression ] "{" { model_element } "}" ;

state_define = state_kind identifier [ "=" expression ]
               ( "{" { state_element } "}" | ";" ) ;
state_kind   = "start" | "state" ;

enum_define  = "enum" identifier "{" enum_variant { "," enum_variant } "}" ;
enum_variant = identifier [ "=" integer ] ;

struct_define = "struct" identifier "{" [ struct_field { "," struct_field } ] "}" ;
struct_field  = identifier ":" type ;
```

## Именованные условия и блоки

```ebnf
condition_define = "cond" identifier "=" condition ;
named_block      = identifier block_statement ;   (* enter / exit / always *)
```

Условие перехода — отдельная грамматика (в ней `=` означает **проверку равенства**).
Приоритет (от низшего к высшему): `|` · `&` · `= !=` · `< > <= >=` · `+ -` ·
унарный `!` · первичные.

```ebnf
condition
    = condition "|" condition  | condition "&" condition
    | condition "=" condition  | condition "!=" condition
    | condition "<" condition  | condition ">" condition
    | condition "<=" condition | condition ">=" condition
    | condition "+" condition  | condition "-" condition
    | "!" condition | "(" condition ")"
    | condition "." member
    | identifier "[" integer "]"
    | identifier "(" [ condition { "," condition } ] ")"
    | "true" | "false" | integer | rational | identifier ;
```

## Встроенные формулы: Guard и LTL

```ebnf
inline_formula_define
    = ":" condition { "," condition } ";"
    | ":" "[" "Guard" "]" condition { "," condition } ";"
    | ":" "[" "LTL"   "]" ltl_expr  { "," ltl_expr  } ";" ;
```

LTL-выражения (приоритет от низшего к высшему): `->` (правоассоциативная) · `|` ·
`&` · `U R` · унарные `! X F G`.

```ebnf
ltl_expr    = ltl_implies ;
ltl_implies = ltl_or "->" ltl_implies | ltl_or ;
ltl_or      = ltl_or "|" ltl_and | ltl_and ;
ltl_and     = ltl_and "&" ltl_until_release | ltl_until_release ;
ltl_until_release
    = ltl_until_release "U" ltl_unary | ltl_until_release "R" ltl_unary | ltl_unary ;
ltl_unary   = "!" ltl_unary | "X" ltl_unary | "F" ltl_unary | "G" ltl_unary | ltl_primary ;
ltl_primary = "true" | "false" | identifier | "(" ltl_expr ")" ;
```

`X F G U R LTL Guard` — обычные идентификаторы вне аннотации `: [LTL] … ;`.

## Выражения

**Присваивание — `:=`; равенство — `=`** (не `==`, он изъят из языка). Приоритет
(от низшего к высшему): `:=`/`?:` · `||` · `&&` · `= !=` · `< > <= >=` · `|` · `^` ·
`& as` · `<< >>` · `+ -` · `* / %` · `**` · унарные `! ~ + -` · адресный литерал ·
первичные.

```ebnf
expression
    = expression ":=" expression                             (* присваивание *)
    | expression "?" expression ":" expression               (* тернарный *)
    | expression "||" expression | expression "&&" expression
    | expression "=" expression  | expression "!=" expression (* равенство *)
    | expression "<" expression  | expression ">" expression
    | expression "<=" expression | expression ">=" expression
    | expression "|" expression  | expression "^" expression | expression "&" expression
    | expression "<<" expression | expression ">>" expression
    | expression "as" type
    | expression "+" expression  | expression "-" expression
    | expression "*" expression  | expression "/" expression | expression "%" expression
    | expression "**" expression
    | "!" expression | "~" expression | "+" expression | "-" expression
    | address_literal
    | "{" expression { "," expression } "}"                  (* инициализатор массива *)
    | identifier "(" [ expression { "," expression } ] ")"   (* вызов функции *)
    | identifier "[" integer "]"                             (* индекс *)
    | identifier "[" [ integer ] ":" [ integer ] "]"         (* срез *)
    | expression "." member
    | "(" expression ")"
    | "true" | "false" | integer | rational
    | string_literal { string_literal } | identifier ;

member = identifier | integer ;    (* x.flag или x.0 (бит) *)
```

`address_literal` — единый лексический токен `0xADDR` или `0xADDR:бит` (не
`integer ":" integer`), что снимает конфликт с тернарным `?:`.

## Операторы

```ebnf
statement
    = "if" expression block_statement
    | "if" expression block_statement "else" statement
    | "loop" block_statement | "loop" loop_cond block_statement
    | "for" [ simple_statement ] ";" [ expression ] ";" [ expression ]
      ( block_statement | ";" )
    | block_statement
    | "assembly" [ string_literal ] block_statement
    | "formula"  [ string_literal ] formula_block
    | inline_formula_define
    | simple_statement ";"
    | "continue" ";" | "break" ";" | "return" [ expression ] ";" ;

block_statement  = "{" { statement } "}" ;
simple_statement = local_variable_define | expression ;
local_variable_define
    = "var"   identifier [ ":" type ] [ ":=" expression ]
    | "const" identifier [ ":" type ] ":=" expression ;
```

`if` без `else` — «открытый» оператор; `else` привязывается к ближайшему `if`.
`loop_cond` — как `expression`, но первый токен не `{` (снимает конфликт `loop {
тело }` vs `loop {init} { тело }`; для инициализатора-массива в условии — скобки:
`loop ({1, 2}) { }`).

## Лексические элементы

```ebnf
identifier      = ( xid_start | "_" | "$" ) { xid_continue | "$" } ;

integer         = decimal_integer | hex_integer | "-" decimal_integer ;
decimal_integer = digit { digit | "_" } ;
hex_integer     = "0x" hex_digit { hex_digit | "_" } ;
rational        = decimal_integer "." decimal_integer [ exponent ]
                | decimal_integer exponent ;
exponent        = ( "e" | "E" ) [ "-" ] decimal_integer ;

string_literal  = '"' { string_char } '"' | "'" { string_char } "'" ;
line_comment    = "//"  { any_char_except_newline } ( newline | eof ) ;
doc_comment     = "///" { any_char_except_newline } ( newline | eof ) ;
```

Идентификаторы — по Unicode (XID_Start/XID_Continue); ключевые слова
идентификаторами быть не могут. Блочные комментарии `/* … */` не поддерживаются.

**Ключевые слова:** `as` `assembly` `break` `cond` `const` `continue` `else`
`enum` `extern` `false` `fn` `for` `formula` `if` `import` `in` `inout` `invariant`
`loop` `model` `next` `out` `ref` `return` `start` `state` `struct` `true` `type`
`var`.

**Операторы и пунктуация:** присваивание `:=`; арифметические `+ - * / % **`;
побитовые `& | ^ ~ << >>`; логические `&& || !`; равенство/сравнение `= != < <= > >=`;
прочие `?` `.` `:` `,` `;` `( )` `{ }` `[ ]` `->` `-->`.
