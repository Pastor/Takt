package org.lam.intellij.refactoring

import com.intellij.lang.refactoring.NamesValidator
import com.intellij.openapi.project.Project
import org.lam.intellij.psi.LamTokenTypes

/**
 * Валидатор имён для rename Lam (фича 0067, R3).
 *
 * Отвергает ключевые слова Lam как имена (в т.ч. жёсткое `address`) — набор
 * берётся из [LamTokenTypes.KEYWORDS] (сторож синхронизации с лексером языка —
 * `LamKeywordSyncTest`). Идентификатор: буква/`_` в начале, далее буквы/цифры/`_`.
 */
class LamNamesValidator : NamesValidator {
    override fun isKeyword(name: String, project: Project?): Boolean =
        name in LamTokenTypes.KEYWORDS

    override fun isIdentifier(name: String, project: Project?): Boolean {
        if (name.isEmpty() || name in LamTokenTypes.KEYWORDS) return false
        if (!(name[0].isLetter() || name[0] == '_')) return false
        return name.all { it.isLetterOrDigit() || it == '_' }
    }
}
