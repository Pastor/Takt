package org.takt.intellij

import com.intellij.openapi.util.IconLoader
import javax.swing.Icon

/** Иконки плагина Takt. */
object TaktIcons {
    /** Иконка типа файла `.takt` (см. `resources/icons/takt.svg`). */
    @JvmField
    val FILE: Icon = IconLoader.getIcon("/icons/takt.svg", TaktIcons::class.java)
}
