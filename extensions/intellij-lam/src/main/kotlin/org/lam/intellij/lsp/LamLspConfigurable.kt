package org.lam.intellij.lsp

import com.intellij.openapi.options.Configurable
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import java.awt.BorderLayout
import java.awt.FlowLayout
import javax.swing.JComponent
import javax.swing.JLabel
import javax.swing.JPanel

/**
 * Страница настроек LSP-слоя (фича 0038, задача 0038-01, R3): путь к `lam-lsp`.
 *
 * Settings → Tools → **Lam Language Server**. Пустой путь ⇒ автопоиск в `PATH`
 * ([LamLspBinary.resolve]). Простая Swing-панель (без UI-DSL) ради минимальной
 * зависимости от версии платформенного API.
 */
class LamLspConfigurable : Configurable {

    private var pathField: TextFieldWithBrowseButton? = null

    override fun getDisplayName(): String = "Lam Language Server"

    override fun createComponent(): JComponent {
        val field = TextFieldWithBrowseButton()
        pathField = field
        field.text = LamLspSettings.getInstance().serverPath

        val row = JPanel(FlowLayout(FlowLayout.LEFT, 0, 0))
        row.add(JLabel("Путь к lam-lsp (пусто → поиск в PATH): "))
        row.add(field)

        val panel = JPanel(BorderLayout())
        panel.add(row, BorderLayout.NORTH)
        return panel
    }

    override fun isModified(): Boolean =
        (pathField?.text ?: "") != LamLspSettings.getInstance().serverPath

    override fun apply() {
        LamLspSettings.getInstance().serverPath = pathField?.text?.trim().orEmpty()
    }

    override fun reset() {
        pathField?.text = LamLspSettings.getInstance().serverPath
    }

    override fun disposeUIResources() {
        pathField = null
    }
}
