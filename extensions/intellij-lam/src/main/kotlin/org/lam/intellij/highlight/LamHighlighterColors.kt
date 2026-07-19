package org.lam.intellij.highlight

import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.HighlighterColors
import com.intellij.openapi.editor.colors.TextAttributesKey

/**
 * Ключи атрибутов текста для подсветки Lam (задача 0022-02).
 *
 * Каждый ключ наследует цвет от стандартного [DefaultLanguageHighlighterColors],
 * поэтому подсветка работает в любой цветовой схеме «из коробки». Страница
 * настройки цветов (`LamColorSettingsPage`, задача 0022-03) переиспользует эти
 * же ключи, давая пользователю переопределять цвета по группам.
 */
object LamHighlighterColors {
    private fun key(name: String, fallback: TextAttributesKey): TextAttributesKey =
        TextAttributesKey.createTextAttributesKey(name, fallback)

    @JvmField val KEYWORD = key("LAM_KEYWORD", DefaultLanguageHighlighterColors.KEYWORD)
    @JvmField val IDENTIFIER = key("LAM_IDENTIFIER", DefaultLanguageHighlighterColors.IDENTIFIER)
    @JvmField val NUMBER = key("LAM_NUMBER", DefaultLanguageHighlighterColors.NUMBER)
    @JvmField val STRING = key("LAM_STRING", DefaultLanguageHighlighterColors.STRING)
    @JvmField val LINE_COMMENT = key("LAM_LINE_COMMENT", DefaultLanguageHighlighterColors.LINE_COMMENT)
    @JvmField val DOC_COMMENT = key("LAM_DOC_COMMENT", DefaultLanguageHighlighterColors.DOC_COMMENT)
    @JvmField val BLOCK_COMMENT = key("LAM_BLOCK_COMMENT", DefaultLanguageHighlighterColors.BLOCK_COMMENT)
    @JvmField val OPERATOR = key("LAM_OPERATOR", DefaultLanguageHighlighterColors.OPERATION_SIGN)
    @JvmField val SEMICOLON = key("LAM_SEMICOLON", DefaultLanguageHighlighterColors.SEMICOLON)
    @JvmField val COMMA = key("LAM_COMMA", DefaultLanguageHighlighterColors.COMMA)
    @JvmField val DOT = key("LAM_DOT", DefaultLanguageHighlighterColors.DOT)
    @JvmField val PARENTHESES = key("LAM_PARENTHESES", DefaultLanguageHighlighterColors.PARENTHESES)
    @JvmField val BRACES = key("LAM_BRACES", DefaultLanguageHighlighterColors.BRACES)
    @JvmField val BRACKETS = key("LAM_BRACKETS", DefaultLanguageHighlighterColors.BRACKETS)
    @JvmField val BAD_CHARACTER = key("LAM_BAD_CHARACTER", HighlighterColors.BAD_CHARACTER)

    // ── Семантические ключи (фича 0038): различают имена по СМЫСЛУ, чего
    // лексический слой дать не может (лексер видит любое имя как IDENTIFIER).
    // Накладываются поверх лексики LSP4IJ-слоем (LamSemanticTokensColorsProvider),
    // наследуют цвет от стандартных семантических категорий платформы.
    @JvmField val FUNCTION = key("LAM_FUNCTION", DefaultLanguageHighlighterColors.FUNCTION_DECLARATION)
    @JvmField val TYPE = key("LAM_TYPE", DefaultLanguageHighlighterColors.CLASS_REFERENCE)
    @JvmField val ENUM_MEMBER = key("LAM_ENUM_MEMBER", DefaultLanguageHighlighterColors.STATIC_FIELD)
    @JvmField val CLASS = key("LAM_CLASS", DefaultLanguageHighlighterColors.CLASS_NAME)
}
