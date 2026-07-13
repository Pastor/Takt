package org.lam.intellij.highlight

import com.intellij.lexer.Lexer
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighterBase
import com.intellij.psi.TokenType
import com.intellij.psi.tree.IElementType
import org.lam.intellij.lexer.LamLexer
import org.lam.intellij.psi.LamTokenTypes

/**
 * Подсветка синтаксиса Lam: сопоставляет токены [LamLexer] цветовым ключам
 * [LamHighlighterColors] (задача 0022-02).
 */
class LamSyntaxHighlighter : SyntaxHighlighterBase() {

    override fun getHighlightingLexer(): Lexer = LamLexer()

    override fun getTokenHighlights(tokenType: IElementType): Array<TextAttributesKey> {
        val key = when (tokenType) {
            LamTokenTypes.KEYWORD -> LamHighlighterColors.KEYWORD
            LamTokenTypes.IDENTIFIER -> LamHighlighterColors.IDENTIFIER
            LamTokenTypes.NUMBER -> LamHighlighterColors.NUMBER
            LamTokenTypes.STRING -> LamHighlighterColors.STRING
            LamTokenTypes.LINE_COMMENT -> LamHighlighterColors.LINE_COMMENT
            LamTokenTypes.DOC_COMMENT -> LamHighlighterColors.DOC_COMMENT
            LamTokenTypes.BLOCK_COMMENT -> LamHighlighterColors.BLOCK_COMMENT

            LamTokenTypes.OP_ASSIGN,
            LamTokenTypes.OP_EQ,
            LamTokenTypes.OP_LE,
            LamTokenTypes.OP_GE,
            LamTokenTypes.OP_LT,
            LamTokenTypes.OP_GT,
            LamTokenTypes.OPERATOR,
            LamTokenTypes.COLON -> LamHighlighterColors.OPERATOR

            LamTokenTypes.SEMICOLON -> LamHighlighterColors.SEMICOLON
            LamTokenTypes.COMMA -> LamHighlighterColors.COMMA
            LamTokenTypes.DOT -> LamHighlighterColors.DOT
            LamTokenTypes.PARENTHESES -> LamHighlighterColors.PARENTHESES
            LamTokenTypes.BRACES -> LamHighlighterColors.BRACES
            LamTokenTypes.BRACKETS -> LamHighlighterColors.BRACKETS

            TokenType.BAD_CHARACTER -> LamHighlighterColors.BAD_CHARACTER
            else -> null
        }
        return if (key == null) TextAttributesKey.EMPTY_ARRAY else pack(key)
    }
}
