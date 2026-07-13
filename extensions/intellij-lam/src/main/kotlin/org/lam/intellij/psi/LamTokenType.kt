package org.lam.intellij.psi

import com.intellij.psi.tree.IElementType
import org.lam.intellij.LamLanguage
import org.jetbrains.annotations.NonNls

/**
 * Базовый тип лексического токена языка Lam.
 *
 * Конкретные токены (ключевые слова, операторы `:=`/`=`/`<=`, литералы,
 * комментарии) объявляются в задаче 0022-02 — источник истины по набору
 * см. `grammar/src/parser/lexer.rs` (таблица `KEYWORDS`) и фичу 0021.
 */
class LamTokenType(@NonNls debugName: String) : IElementType(debugName, LamLanguage) {
    override fun toString(): String = "LamTokenType." + super.toString()
}
