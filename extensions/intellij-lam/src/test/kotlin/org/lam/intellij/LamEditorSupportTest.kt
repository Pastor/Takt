package org.lam.intellij

import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.lam.intellij.editor.LamBraceMatcher
import org.lam.intellij.editor.LamCommenter
import org.lam.intellij.psi.LamTokenTypes

/**
 * Проверки эргономики редактора (задача 0022-03; требование R5, критерий A6):
 * комментирование и парные скобки.
 */
class LamEditorSupportTest : BasePlatformTestCase() {

    fun testCommenterPrefixes() {
        val c = LamCommenter()
        assertEquals("//", c.lineCommentPrefix)
        assertEquals("/*", c.blockCommentPrefix)
        assertEquals("*/", c.blockCommentSuffix)
    }

    fun testBracePairs() {
        val pairs = LamBraceMatcher().pairs
        assertEquals(3, pairs.size)

        val braces = pairs.single { it.leftBraceType == LamTokenTypes.LBRACE }
        assertEquals(LamTokenTypes.RBRACE, braces.rightBraceType)
        assertTrue("Фигурные скобки должны быть структурными", braces.isStructural)

        val parens = pairs.single { it.leftBraceType == LamTokenTypes.LPAREN }
        assertEquals(LamTokenTypes.RPAREN, parens.rightBraceType)

        val brackets = pairs.single { it.leftBraceType == LamTokenTypes.LBRACKET }
        assertEquals(LamTokenTypes.RBRACKET, brackets.rightBraceType)
    }
}
