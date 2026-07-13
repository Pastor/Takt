package org.lam.intellij

import com.intellij.psi.TokenType
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.lam.intellij.highlight.LamColorSettingsPage
import org.lam.intellij.highlight.LamSyntaxHighlighter
import org.lam.intellij.lexer.LamLexer

/**
 * Проверки страницы настройки цветов (задача 0022-03; требование R4, критерий A5).
 */
class LamColorSettingsPageTest : BasePlatformTestCase() {

    private val page = LamColorSettingsPage()

    fun testMetadata() {
        assertEquals("Lam", page.displayName)
        assertNotNull(page.icon)
        assertTrue(page.highlighter is LamSyntaxHighlighter)
        assertTrue("Должны быть дескрипторы атрибутов", page.attributeDescriptors.isNotEmpty())
        assertTrue(page.demoText.isNotBlank())
    }

    fun testDemoTextHasNoBadCharacters() {
        // Демонстрационный фрагмент обязан быть валидным Lam: без BAD_CHARACTER
        // (в частности, без выведенного `==`).
        val lexer = LamLexer()
        lexer.start(page.demoText)
        while (lexer.tokenType != null) {
            assertNotSame(
                "Демо-текст страницы цветов содержит некорректный токен: " +
                    "'${page.demoText.substring(lexer.tokenStart, lexer.tokenEnd)}'",
                TokenType.BAD_CHARACTER,
                lexer.tokenType,
            )
            lexer.advance()
        }
    }

    fun testEveryDescriptorKeyIsUnique() {
        val keys = page.attributeDescriptors.map { it.key }
        assertEquals(keys.size, keys.toSet().size)
    }
}
