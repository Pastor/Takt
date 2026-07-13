package org.lam.intellij.parser

import com.intellij.lang.ASTNode
import com.intellij.lang.PsiBuilder
import com.intellij.lang.PsiParser
import com.intellij.psi.tree.IElementType

/**
 * Плоский разборщик Lam (фича 0023): все токены помещаются листьями под корень.
 *
 * Синтаксическая структура не строится (ADR 0023, Option A) — достаточно листьев
 * для навигации. Разбор всегда успешен и не может «поломать» файл: любой набор
 * токенов лексера принимается как есть.
 */
class LamParser : PsiParser {
    override fun parse(root: IElementType, builder: PsiBuilder): ASTNode {
        val rootMarker = builder.mark()
        while (!builder.eof()) {
            builder.advanceLexer()
        }
        rootMarker.done(root)
        return builder.treeBuilt
    }
}
