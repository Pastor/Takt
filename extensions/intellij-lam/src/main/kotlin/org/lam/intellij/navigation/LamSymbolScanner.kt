package org.lam.intellij.navigation

import com.intellij.openapi.util.TextRange
import com.intellij.psi.TokenType
import com.intellij.psi.tree.IElementType
import org.lam.intellij.lexer.LamLexer
import org.lam.intellij.psi.LamTokenTypes

/** Объявление символа Lam: имя, диапазон объявляющего идентификатора, вид. */
data class LamDeclaration(val name: String, val range: TextRange, val kind: String)

/**
 * Сканер деклараций Lam поверх [LamLexer] (фича 0023, задача 0023-01).
 *
 * Источник истины по формам деклараций — грамматика `grammar/src/grammar.lalrpop`
 * (правила `model`/`state`/`start`/`type`/`cond`/`var`/`const`/`fn` вида
 * `kw <Id>`; порты `in`/`out`/`inout <Id>`; правило `EnumDefine` с константами-
 * вариантами в `{ … }`; правило `Import` с `as`-переименованиями). Разрешение
 * имён — эвристика по токенам одного файла без областей видимости (осознанное
 * ограничение Option A, ADR 0023).
 */
object LamSymbolScanner {

    /**
     * Ключевые слова, за которыми идёт имя объявляемого символа: `kw <Id>`.
     * Порты `in`/`out`/`inout` объявляют имя так же (`in name: Type := …;`), поэтому
     * их использования (в т.ч. как часть `port.N` — `BitAccess`) резолвятся к порту.
     */
    private val SIMPLE_DECL_KEYWORDS = setOf(
        "model", "state", "start", "type", "cond", "var", "const", "fn",
        "in", "out", "inout",
    )

    private data class Tok(val type: IElementType, val start: Int, val end: Int, val text: String)

    /** Строит список деклараций в порядке появления в тексте. */
    fun scan(text: CharSequence): List<LamDeclaration> {
        val toks = tokenize(text)
        val decls = ArrayList<LamDeclaration>()
        var i = 0
        while (i < toks.size) {
            val t = toks[i]
            if (t.type == LamTokenTypes.KEYWORD) {
                when {
                    t.text == "import" -> {
                        i = scanImport(toks, i, decls)
                        continue
                    }
                    t.text == "enum" -> {
                        i = scanEnum(toks, i, decls)
                        continue
                    }
                    t.text in SIMPLE_DECL_KEYWORDS -> {
                        val next = toks.getOrNull(i + 1)
                        if (next != null && next.type == LamTokenTypes.IDENTIFIER) {
                            decls.add(LamDeclaration(next.text, TextRange(next.start, next.end), t.text))
                        }
                    }
                }
            }
            i++
        }
        return decls
    }

    /**
     * Разбирает `import`-выражение от индекса `from` (токен `import`) до `;`.
     * Собирает **локально введённые** имена: алиасы после `as` и «голые» имена в
     * фигурных скобках `{ A, B }`. Возвращает индекс токена сразу за `;`.
     */
    private fun scanImport(toks: List<Tok>, from: Int, decls: MutableList<LamDeclaration>): Int {
        var j = from + 1
        var braceDepth = 0
        while (j < toks.size) {
            val tk = toks[j]
            if (tk.type == LamTokenTypes.SEMICOLON) {
                j++
                break
            }
            when (tk.type) {
                LamTokenTypes.LBRACE -> braceDepth++
                LamTokenTypes.RBRACE -> if (braceDepth > 0) braceDepth--
                LamTokenTypes.KEYWORD ->
                    if (tk.text == "as") {
                        val alias = toks.getOrNull(j + 1)
                        if (alias != null && alias.type == LamTokenTypes.IDENTIFIER) {
                            decls.add(LamDeclaration(alias.text, TextRange(alias.start, alias.end), "import"))
                        }
                    }
                LamTokenTypes.IDENTIFIER ->
                    // Голое имя в списке `{ A, B }` вводится под своим именем —
                    // если оно не источник переименования (`A as C`) и не сам алиас.
                    if (braceDepth > 0) {
                        val prev = toks.getOrNull(j - 1)
                        val next = toks.getOrNull(j + 1)
                        val isAliasTarget = prev?.type == LamTokenTypes.KEYWORD && prev.text == "as"
                        val isRenameSource = next?.type == LamTokenTypes.KEYWORD && next.text == "as"
                        if (!isAliasTarget && !isRenameSource) {
                            decls.add(LamDeclaration(tk.text, TextRange(tk.start, tk.end), "import"))
                        }
                    }
                else -> {}
            }
            j++
        }
        return j
    }

    /**
     * Разбирает `enum <Name> { V1 = n, V2, … }` от индекса `from` (токен `enum`).
     * Записывает имя перечисления и имена его констант-вариантов, чтобы переход к
     * декларации работал и от использования варианта (`… := Closing;`). Возвращает
     * индекс токена сразу за закрывающей `}` (или конец при неполной структуре).
     */
    private fun scanEnum(toks: List<Tok>, from: Int, decls: MutableList<LamDeclaration>): Int {
        var j = from + 1
        val nameTok = toks.getOrNull(j)
        if (nameTok != null && nameTok.type == LamTokenTypes.IDENTIFIER) {
            decls.add(LamDeclaration(nameTok.text, TextRange(nameTok.start, nameTok.end), "enum"))
            j++
        }
        // Без тела `{ … }` вариантов нет.
        if (toks.getOrNull(j)?.type != LamTokenTypes.LBRACE) return j
        j++
        var depth = 1
        // Вариант начинается сразу после `{` или `,`; значение `= n` — это NUMBER,
        // а не идентификатор, поэтому за вариант не принимается.
        var atVariantStart = true
        while (j < toks.size && depth > 0) {
            val tk = toks[j]
            when (tk.type) {
                LamTokenTypes.LBRACE -> depth++
                LamTokenTypes.RBRACE -> depth--
                LamTokenTypes.COMMA -> if (depth == 1) atVariantStart = true
                LamTokenTypes.IDENTIFIER ->
                    if (depth == 1 && atVariantStart) {
                        decls.add(LamDeclaration(tk.text, TextRange(tk.start, tk.end), "enum-variant"))
                    }
                else -> {}
            }
            if (tk.type != LamTokenTypes.COMMA) atVariantStart = false
            j++
        }
        return j
    }

    /** Значимые токены (без пробелов и комментариев). */
    private fun tokenize(text: CharSequence): List<Tok> {
        val lexer = LamLexer()
        lexer.start(text)
        val result = ArrayList<Tok>()
        while (true) {
            val type = lexer.tokenType ?: break
            if (type != TokenType.WHITE_SPACE &&
                type != LamTokenTypes.LINE_COMMENT &&
                type != LamTokenTypes.DOC_COMMENT &&
                type != LamTokenTypes.BLOCK_COMMENT
            ) {
                val s = lexer.tokenStart
                val e = lexer.tokenEnd
                result.add(Tok(type, s, e, text.subSequence(s, e).toString()))
            }
            lexer.advance()
        }
        return result
    }
}
