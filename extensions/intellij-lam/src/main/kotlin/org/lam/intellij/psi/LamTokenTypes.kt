package org.lam.intellij.psi

/**
 * Категории лексических токенов Lam для подсветки (задача 0022-02).
 *
 * Источник истины по набору ключевых слов и операторов — Rust-лексер
 * `grammar/src/parser/lexer.rs` (таблица `KEYWORDS`) и операторы фичи 0021
 * (`:=` присваивание, `=` сравнение, `<=` реляционный; `==` выведен из языка —
 * подсвечивается как `BAD_CHARACTER`). Соответствие [KEYWORDS] эталону из
 * `parser/lexer.rs` проверяется регресс-тестом `LamKeywordSyncTest`.
 */
object LamTokenTypes {
    @JvmField val IDENTIFIER = LamTokenType("IDENTIFIER")
    @JvmField val KEYWORD = LamTokenType("KEYWORD")

    @JvmField val NUMBER = LamTokenType("NUMBER")
    @JvmField val STRING = LamTokenType("STRING")

    @JvmField val LINE_COMMENT = LamTokenType("LINE_COMMENT")
    @JvmField val DOC_COMMENT = LamTokenType("DOC_COMMENT")
    @JvmField val BLOCK_COMMENT = LamTokenType("BLOCK_COMMENT")

    // Операторы фичи 0021 — различимы для тестов (T3–T5); все раскрашиваются
    // как «знак операции».
    @JvmField val OP_ASSIGN = LamTokenType("OP_ASSIGN") // :=
    @JvmField val OP_EQ = LamTokenType("OP_EQ")         // =
    @JvmField val OP_LE = LamTokenType("OP_LE")         // <=
    @JvmField val OP_GE = LamTokenType("OP_GE")         // >=
    @JvmField val OP_LT = LamTokenType("OP_LT")         // <
    @JvmField val OP_GT = LamTokenType("OP_GT")         // >

    // Прочие операторы: + - * ** / % ! != && || | & ^ ~ -> --> => << >> ? #
    @JvmField val OPERATOR = LamTokenType("OPERATOR")

    @JvmField val SEMICOLON = LamTokenType("SEMICOLON")
    @JvmField val COMMA = LamTokenType("COMMA")
    @JvmField val DOT = LamTokenType("DOT")
    @JvmField val COLON = LamTokenType("COLON")

    // Скобки — раздельные типы для открывающих/закрывающих, чтобы работал
    // подсветчик парных скобок (`LamBraceMatcher`, задача 0022-03).
    @JvmField val LPAREN = LamTokenType("LPAREN")     // (
    @JvmField val RPAREN = LamTokenType("RPAREN")     // )
    @JvmField val LBRACE = LamTokenType("LBRACE")     // {
    @JvmField val RBRACE = LamTokenType("RBRACE")     // }
    @JvmField val LBRACKET = LamTokenType("LBRACKET") // [
    @JvmField val RBRACKET = LamTokenType("RBRACKET") // ]

    /**
     * Ключевые слова Lam — зеркало таблицы `KEYWORDS` из
     * `grammar/src/parser/lexer.rs` (правило: источник истины — лексер языка).
     * При добавлении/удалении ключевого слова в языке этот набор обязан
     * измениться синхронно (ловится `LamKeywordSyncTest`).
     */
    @JvmField
    val KEYWORDS: Set<String> = setOf(
        "break", "const", "continue", "else", "false", "for", "fn", "if", "match",
        "_", "import", "loop", "while", "return", "string", "true", "type", "as",
        "assembly", "formula", "in", "out", "inout", "address", "model", "state",
        "start", "ref", "template", "cond", "var", "next", "extern", "enum",
        "struct", "from", "X", "F", "G", "U", "R", "LTL", "Guard",
        // `invariant` — сахар над `cond`+Guard (фича 0044); был пропущен в плагине.
        "invariant",
    )
}
