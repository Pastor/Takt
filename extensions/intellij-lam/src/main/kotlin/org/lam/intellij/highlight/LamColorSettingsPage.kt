package org.lam.intellij.highlight

import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighter
import com.intellij.openapi.options.colors.AttributesDescriptor
import com.intellij.openapi.options.colors.ColorDescriptor
import com.intellij.openapi.options.colors.ColorSettingsPage
import org.lam.intellij.LamIcons
import javax.swing.Icon

/**
 * Страница «Settings → Editor → Color Scheme → Lam» (задача 0022-03, R4/A5).
 *
 * Даёт пользователю переопределять цвета по группам и показывает результат на
 * демонстрационном фрагменте Lam (пост-0021 синтаксис `:=`/`=`/`<=`).
 */
class LamColorSettingsPage : ColorSettingsPage {

    override fun getDisplayName(): String = "Lam"

    override fun getIcon(): Icon = LamIcons.FILE

    override fun getHighlighter(): SyntaxHighlighter = LamSyntaxHighlighter()

    override fun getDemoText(): String = DEMO_TEXT

    override fun getAdditionalHighlightingTagToDescriptorMap(): Map<String, TextAttributesKey>? = null

    override fun getAttributeDescriptors(): Array<AttributesDescriptor> = DESCRIPTORS

    override fun getColorDescriptors(): Array<ColorDescriptor> = ColorDescriptor.EMPTY_ARRAY

    private companion object {
        val DESCRIPTORS = arrayOf(
            AttributesDescriptor("Ключевое слово", LamHighlighterColors.KEYWORD),
            AttributesDescriptor("Идентификатор", LamHighlighterColors.IDENTIFIER),
            AttributesDescriptor("Число", LamHighlighterColors.NUMBER),
            AttributesDescriptor("Строка", LamHighlighterColors.STRING),
            AttributesDescriptor("Оператор", LamHighlighterColors.OPERATOR),
            AttributesDescriptor("Строчный комментарий", LamHighlighterColors.LINE_COMMENT),
            AttributesDescriptor("Документационный комментарий", LamHighlighterColors.DOC_COMMENT),
            AttributesDescriptor("Блочный комментарий", LamHighlighterColors.BLOCK_COMMENT),
            AttributesDescriptor("Точка с запятой", LamHighlighterColors.SEMICOLON),
            AttributesDescriptor("Запятая", LamHighlighterColors.COMMA),
            AttributesDescriptor("Точка", LamHighlighterColors.DOT),
            AttributesDescriptor("Круглые скобки", LamHighlighterColors.PARENTHESES),
            AttributesDescriptor("Фигурные скобки", LamHighlighterColors.BRACES),
            AttributesDescriptor("Квадратные скобки", LamHighlighterColors.BRACKETS),
            AttributesDescriptor("Некорректный символ", LamHighlighterColors.BAD_CHARACTER),
        )

        val DEMO_TEXT = """
            // Модель светофора (демо подсветки Lam)
            import "common.lam";

            /// Счётчик тактов автомата
            model Traffic {
                var counter: u8 := 0;
                const LIMIT := 10;

                start Red = Warmup + (Idle | Ready) {
                    next Green;
                    : counter <= LIMIT;
                }

                state Green {
                    ref Red: counter = LIMIT;   /* переход по достижению предела */
                }
            }
        """.trimIndent()
    }
}
