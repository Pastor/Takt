package org.lam.intellij

import com.intellij.psi.TokenType
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.lam.intellij.highlight.LamHighlighterColors
import org.lam.intellij.highlight.LamSyntaxHighlighter
import org.lam.intellij.psi.LamTokenTypes

/**
 * Проверки маппинга токен → цветовой ключ (задача 0022-02; критерий A2).
 */
class LamSyntaxHighlighterTest : BasePlatformTestCase() {

    private val hl = LamSyntaxHighlighter()

    private fun keyOf(type: com.intellij.psi.tree.IElementType) =
        hl.getTokenHighlights(type).single()

    fun testCategoriesMapToExpectedColors() {
        assertEquals(LamHighlighterColors.KEYWORD, keyOf(LamTokenTypes.KEYWORD))
        assertEquals(LamHighlighterColors.NUMBER, keyOf(LamTokenTypes.NUMBER))
        assertEquals(LamHighlighterColors.STRING, keyOf(LamTokenTypes.STRING))
        assertEquals(LamHighlighterColors.LINE_COMMENT, keyOf(LamTokenTypes.LINE_COMMENT))
        assertEquals(LamHighlighterColors.DOC_COMMENT, keyOf(LamTokenTypes.DOC_COMMENT))
        assertEquals(LamHighlighterColors.BLOCK_COMMENT, keyOf(LamTokenTypes.BLOCK_COMMENT))
        assertEquals(LamHighlighterColors.BRACES, keyOf(LamTokenTypes.BRACES))
    }

    fun testAllOperatorsShareOperatorColor() {
        for (op in listOf(
            LamTokenTypes.OP_ASSIGN, LamTokenTypes.OP_EQ, LamTokenTypes.OP_LE,
            LamTokenTypes.OP_GE, LamTokenTypes.OP_LT, LamTokenTypes.OP_GT,
            LamTokenTypes.OPERATOR, LamTokenTypes.COLON,
        )) {
            assertEquals(LamHighlighterColors.OPERATOR, keyOf(op))
        }
    }

    fun testBadCharacterHighlighted() {
        assertEquals(LamHighlighterColors.BAD_CHARACTER, keyOf(TokenType.BAD_CHARACTER))
    }
}
