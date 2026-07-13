package org.lam.intellij.psi

import com.intellij.extapi.psi.PsiFileBase
import com.intellij.openapi.fileTypes.FileType
import com.intellij.psi.FileViewProvider
import org.lam.intellij.LamFileType
import org.lam.intellij.LamLanguage

/**
 * PSI-файл языка Lam (фича 0023).
 *
 * Дерево — «плоское» (см. [org.lam.intellij.parser.LamParserDefinition]): все
 * токены лексера становятся листьями под корнем. Полноценного синтаксического
 * дерева нет (осознанное решение ADR 0023, Option A) — его достаточно, чтобы у
 * элементов под кареткой были реальные `PsiElement` для навигации к декларации
 * и ссылок на файлы `import`.
 */
class LamFile(viewProvider: FileViewProvider) : PsiFileBase(viewProvider, LamLanguage) {
    override fun getFileType(): FileType = LamFileType
    override fun toString(): String = "Lam File"
}
