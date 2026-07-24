package org.takt.intellij.psi

import com.intellij.extapi.psi.ASTWrapperPsiElement
import com.intellij.lang.ASTNode
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiNameIdentifierOwner
import com.intellij.psi.PsiReference
import com.intellij.psi.impl.source.tree.LeafElement

/**
 * Композитные узлы имён Takt (фича 0067, R3 — нативный rename).
 *
 * Каждый узел оборачивает **один** листовой токен `IDENTIFIER`. Декларация
 * ([TaktNameDecl]) — `PsiNamedElement` (цель rename); использование
 * ([TaktNameRef]) — носитель `PsiReference` к декларации того же файла. Роль
 * (декларация/использование) определяет **парсер** по эвристике
 * `TaktSymbolScanner` (единый источник форм `kw <Id>`/`Import`/`enum`), а не сам
 * узел — двух второй реализации грамматики не появляется.
 */

/** Общая база: замена текста единственного дочернего листа-идентификатора. */
sealed class TaktIdentifierElement(node: ASTNode) : ASTWrapperPsiElement(node) {
    /** Заменяет текст листа-идентификатора, сохраняя контекст дерева (CharTable). */
    fun setIdentifierText(newText: String) {
        (node.firstChildNode as? LeafElement)?.replaceWithText(newText)
    }
}

/** Идентификатор-декларация: `PsiNameIdentifierOwner`, цель штатного rename IDEA. */
class TaktNameDecl(node: ASTNode) : TaktIdentifierElement(node), PsiNameIdentifierOwner {
    override fun getName(): String = text
    override fun getNameIdentifier(): PsiElement? = firstChild
    override fun setName(name: String): PsiElement {
        setIdentifierText(name)
        return this
    }
}

/** Идентификатор-использование: несёт `PsiReference` к декларации в файле. */
class TaktNameRef(node: ASTNode) : TaktIdentifierElement(node) {
    override fun getReference(): PsiReference = TaktNameReference(this)
    override fun getReferences(): Array<PsiReference> = arrayOf(reference)
}
