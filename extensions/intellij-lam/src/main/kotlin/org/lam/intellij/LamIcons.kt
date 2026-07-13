package org.lam.intellij

import com.intellij.openapi.util.IconLoader
import javax.swing.Icon

/** Иконки плагина Lam. */
object LamIcons {
    /** Иконка типа файла `.lam` (см. `resources/icons/lam.svg`). */
    @JvmField
    val FILE: Icon = IconLoader.getIcon("/icons/lam.svg", LamIcons::class.java)
}
