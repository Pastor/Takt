package org.lam.intellij

import com.intellij.openapi.fileTypes.LanguageFileType
import javax.swing.Icon

/**
 * Тип файла Lam — связывает расширение `.lam` с [LamLanguage].
 *
 * Регистрируется в `plugin.xml` (`com.intellij.fileType`), после чего IDE
 * распознаёт `*.lam` как язык Lam (критерий приёмки A1, требование R1).
 */
object LamFileType : LanguageFileType(LamLanguage) {
    override fun getName(): String = "Lam"

    override fun getDescription(): String = "Lam FSM specification"

    override fun getDefaultExtension(): String = "lam"

    override fun getIcon(): Icon = LamIcons.FILE
}
