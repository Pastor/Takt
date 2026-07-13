package org.lam.intellij

import com.intellij.lang.Language

/**
 * Язык Lam (Language of Automata Models) для IntelliJ Platform.
 *
 * Единственный экземпляр-синглтон, к которому привязываются [LamFileType],
 * лексер и подсветка (задачи 0022-02/03). Идентификатор совпадает с id плагина.
 */
object LamLanguage : Language("Lam") {
    private fun readResolve(): Any = LamLanguage

    override fun getDisplayName(): String = "Lam"

    override fun isCaseSensitive(): Boolean = true
}
