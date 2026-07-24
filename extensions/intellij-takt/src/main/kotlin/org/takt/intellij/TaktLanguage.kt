package org.takt.intellij

import com.intellij.lang.Language

/**
 * Язык Takt (Typed, Automata, Known Timing) для IntelliJ Platform.
 *
 * Единственный экземпляр-синглтон, к которому привязываются [TaktFileType],
 * лексер и подсветка (задачи 0022-02/03). Идентификатор совпадает с id плагина.
 */
object TaktLanguage : Language("Takt") {
    private fun readResolve(): Any = TaktLanguage

    override fun getDisplayName(): String = "Takt"

    override fun isCaseSensitive(): Boolean = true
}
