package org.takt.intellij.lsp

import com.intellij.openapi.options.Configurable
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import java.awt.BorderLayout
import java.awt.FlowLayout
import javax.swing.JComponent
import javax.swing.JLabel
import javax.swing.JPanel

/**
 * Страница настроек LSP-слоя (фича 0038, задача 0038-01, R3): путь к `takt-lsp`.
 *
 * Settings → Tools → **Takt Language Server**. Пустой путь ⇒ автопоиск в `PATH`
 * ([TaktLspBinary.resolve]). Простая Swing-панель (без UI-DSL) ради минимальной
 * зависимости от версии платформенного API.
 */
class TaktLspConfigurable : Configurable {

    private var pathField: TextFieldWithBrowseButton? = null

    override fun getDisplayName(): String = "Takt Language Server"

    override fun createComponent(): JComponent {
        val field = TextFieldWithBrowseButton()
        pathField = field
        field.text = TaktLspSettings.getInstance().serverPath

        val row = JPanel(FlowLayout(FlowLayout.LEFT, 0, 0))
        row.add(JLabel("Путь к takt-lsp (пусто → поиск в PATH): "))
        row.add(field)

        val panel = JPanel(BorderLayout())
        panel.add(row, BorderLayout.NORTH)
        return panel
    }

    override fun isModified(): Boolean =
        (pathField?.text ?: "") != TaktLspSettings.getInstance().serverPath

    override fun apply() {
        TaktLspSettings.getInstance().serverPath = pathField?.text?.trim().orEmpty()
    }

    override fun reset() {
        pathField?.text = TaktLspSettings.getInstance().serverPath
    }

    override fun disposeUIResources() {
        pathField = null
    }
}
