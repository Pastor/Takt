package org.lam.intellij.psi

import com.intellij.psi.tree.IElementType
import org.lam.intellij.LamLanguage
import org.jetbrains.annotations.NonNls

/**
 * Базовый тип узла PSI-дерева языка Lam.
 *
 * Каркас под будущее PSI (навигация/инспекции — отдельные фичи). Для
 * лексической подсветки (0022-02) PSI не требуется, но тип заведён заранее,
 * чтобы точки расширения были единообразны.
 */
class LamElementType(@NonNls debugName: String) : IElementType(debugName, LamLanguage)
