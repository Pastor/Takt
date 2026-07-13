package org.lam.intellij.psi

import com.intellij.psi.TokenType
import com.intellij.psi.tree.TokenSet

/**
 * Наборы токенов для [org.lam.intellij.parser.LamParserDefinition] (фича 0023).
 *
 * Комментарии/строки/пробелы объявляются платформе, чтобы флаговый PSI-разбор
 * корректно относил их к тривиям и строковым литералам (важно для навигации и
 * пропуска тривий при поиске значимых токенов).
 */
object LamTokenSets {
    val COMMENTS: TokenSet = TokenSet.create(
        LamTokenTypes.LINE_COMMENT,
        LamTokenTypes.DOC_COMMENT,
        LamTokenTypes.BLOCK_COMMENT,
    )

    val STRINGS: TokenSet = TokenSet.create(LamTokenTypes.STRING)

    val WHITESPACES: TokenSet = TokenSet.create(TokenType.WHITE_SPACE)
}
