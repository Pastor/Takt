package org.lam.intellij

import com.intellij.psi.TokenType
import com.intellij.psi.tree.IElementType
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.lam.intellij.lexer.LamLexer
import org.lam.intellij.psi.LamTokenTypes

/**
 * Проверки лексера Lam (задача 0022-02; требования R2, критерии A2/A4).
 * Ключевой контрпример — `==` подсвечивается как BAD_CHARACTER (0021).
 */
class LamLexerTest : BasePlatformTestCase() {

    /** Токены без пробелов: (тип, текст). */
    private fun lex(text: String): List<Pair<IElementType, String>> {
        val lexer = LamLexer()
        lexer.start(text)
        val result = ArrayList<Pair<IElementType, String>>()
        while (lexer.tokenType != null) {
            val type = lexer.tokenType!!
            if (type != TokenType.WHITE_SPACE) {
                result.add(type to text.substring(lexer.tokenStart, lexer.tokenEnd))
            }
            lexer.advance()
        }
        return result
    }

    private fun types(text: String) = lex(text).map { it.first }

    fun testAssignOperator() {
        assertEquals(
            listOf(LamTokenTypes.IDENTIFIER, LamTokenTypes.OP_ASSIGN, LamTokenTypes.NUMBER, LamTokenTypes.SEMICOLON),
            types("x := 1;"),
        )
    }

    fun testEqualityOperator() {
        // `cond C = x = y;` — оба `=` это сравнение (OP_EQ), а не присваивание.
        val eq = lex("a = b").filter { it.first == LamTokenTypes.OP_EQ }
        assertEquals(1, eq.size)
        assertEquals("=", eq[0].second)
    }

    fun testRelationalLessEqual() {
        val le = lex("x <= 3").filter { it.first == LamTokenTypes.OP_LE }
        assertEquals(1, le.size)
        assertEquals("<=", le[0].second)
    }

    fun testDoubleEqualsIsBadCharacter() {
        // Контрпример CT1: `==` выведен из языка в 0021 — не валидный оператор.
        val bad = lex("x == y").filter { it.first == TokenType.BAD_CHARACTER }
        assertEquals(1, bad.size)
        assertEquals("==", bad[0].second)
    }

    fun testArbitraryNonAlphaIsBadCharacter() {
        // CT3 (0022, остаточная проверка 0089): произвольный неалфавитный символ
        // вне операторов/пунктуации языка — BAD_CHARACTER. Лексер покрывает весь
        // ввод, не «проглатывая» чужой символ (инвариант подсветки).
        for (ch in listOf("@", "$", "`", "\\", "№")) {
            val bad = lex(ch).filter { it.first == TokenType.BAD_CHARACTER }
            assertEquals("символ '$ch' должен быть BAD_CHARACTER", 1, bad.size)
            assertEquals(ch, bad[0].second)
        }
    }

    fun testKeywordsHighlighted() {
        val src = "model state start ref next cond var fn type enum struct import from as if else"
        assertTrue(types(src).all { it == LamTokenTypes.KEYWORD })
    }

    fun testLtlKeywords() {
        // Односимвольные LTL-операторы X F G U R и LTL/Guard — ключевые слова.
        assertTrue(types("X F G U R LTL Guard").all { it == LamTokenTypes.KEYWORD })
    }

    fun testIdentifierVsKeyword() {
        assertEquals(listOf(LamTokenTypes.IDENTIFIER), types("myState"))
        assertEquals(listOf(LamTokenTypes.KEYWORD), types("state"))
    }

    fun testNumbers() {
        assertEquals(LamTokenTypes.NUMBER, types("42").single())
        assertEquals(LamTokenTypes.NUMBER, types("0xFF").single())
        assertEquals(LamTokenTypes.NUMBER, types("3.14").single())
        assertEquals(LamTokenTypes.NUMBER, types("1e10").single())
        assertEquals(LamTokenTypes.NUMBER, types("2.5E-3").single())
    }

    fun testStringLiteral() {
        val toks = lex("""import "util.lam";""")
        val str = toks.single { it.first == LamTokenTypes.STRING }
        assertEquals("\"util.lam\"", str.second)
    }

    fun testComments() {
        assertEquals(LamTokenTypes.LINE_COMMENT, types("// комментарий").single())
        assertEquals(LamTokenTypes.DOC_COMMENT, types("/// doc").single())
        assertEquals(LamTokenTypes.BLOCK_COMMENT, types("/* блок */").single())
    }

    fun testBlockCommentMultiline() {
        assertEquals(LamTokenTypes.BLOCK_COMMENT, types("/* строка1\nстрока2 */").single())
    }

    fun testBracesParensBrackets() {
        // start Start = A + B + (C | D) + E { next Next; }
        val src = "A + (C | D) { [x] };"
        val types = types(src)
        assertTrue(types.contains(LamTokenTypes.LPAREN))
        assertTrue(types.contains(LamTokenTypes.RPAREN))
        assertTrue(types.contains(LamTokenTypes.LBRACE))
        assertTrue(types.contains(LamTokenTypes.RBRACE))
        assertTrue(types.contains(LamTokenTypes.LBRACKET))
        assertTrue(types.contains(LamTokenTypes.RBRACKET))
        assertTrue(types.contains(LamTokenTypes.SEMICOLON))
        assertTrue(types.contains(LamTokenTypes.OPERATOR)) // + и |
    }

    fun testTokenOffsetsCoverInput() {
        // Лексер обязан покрыть весь ввод без разрывов (инвариант подсветки).
        val text = "var x := 0xFF; // c\nnext End;"
        val lexer = LamLexer()
        lexer.start(text)
        var pos = 0
        while (lexer.tokenType != null) {
            assertEquals(pos, lexer.tokenStart)
            assertTrue(lexer.tokenEnd > lexer.tokenStart)
            pos = lexer.tokenEnd
            lexer.advance()
        }
        assertEquals(text.length, pos)
    }
}
