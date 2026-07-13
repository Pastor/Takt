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
 * (правила `model`/`state`/`start`/`type`/`enum`/`cond`/`var`/`const`/`fn` вида
 * `kw <Id>` и правило `Import` с `as`-переименованиями). Разрешение имён —
 * эвристика по токенам одного файла без областей видимости (осознанное
 * ограничение Option A, ADR 0023).
 */
object LamSymbolScanner {

    /** Ключевые слова, за которыми идёт имя объявляемого символа. */
    private val SIMPLE_DECL_KEYWORDS = setOf(
        "model", "state", "start", "type", "enum", "cond", "var", "const", "fn",
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
