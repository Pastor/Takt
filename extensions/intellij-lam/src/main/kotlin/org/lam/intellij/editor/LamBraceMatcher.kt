package org.lam.intellij.editor

import com.intellij.lang.BracePair
import com.intellij.lang.PairedBraceMatcher
import com.intellij.psi.PsiFile
import com.intellij.psi.tree.IElementType
import org.lam.intellij.psi.LamTokenTypes

/**
 * Подсветка парных скобок Lam: `{}`, `()`, `[]` (задача 0022-03).
 * Фигурные скобки — структурные (влияют на навигацию по блокам).
 */
class LamBraceMatcher : PairedBraceMatcher {
    override fun getPairs(): Array<BracePair> = PAIRS

    override fun isPairedBracesAllowedBeforeType(lbrace: IElementType, contextType: IElementType?): Boolean = true

    override fun getCodeConstructStart(file: PsiFile?, openingBraceOffset: Int): Int = openingBraceOffset

    private companion object {
        val PAIRS = arrayOf(
            BracePair(LamTokenTypes.LBRACE, LamTokenTypes.RBRACE, true),
            BracePair(LamTokenTypes.LPAREN, LamTokenTypes.RPAREN, false),
            BracePair(LamTokenTypes.LBRACKET, LamTokenTypes.RBRACKET, false),
        )
    }
}
