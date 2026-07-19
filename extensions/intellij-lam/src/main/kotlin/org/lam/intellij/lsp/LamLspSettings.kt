package org.lam.intellij.lsp

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage

/**
 * Настройки LSP-интеграции плагина (фича 0038, задача 0038-01): путь к `lam-lsp`.
 *
 * Хранилище — [PersistentStateComponent] уровня приложения (путь к бинарнику
 * общий для всех проектов). Пустой путь ⇒ автопоиск в `PATH`
 * ([LamLspBinary.resolve]).
 */
@State(name = "LamLspSettings", storages = [Storage("lam.xml")])
class LamLspSettings : PersistentStateComponent<LamLspSettings.State> {

    /** Сериализуемое состояние настроек. */
    class State {
        /** Явный путь к `lam-lsp` (пусто ⇒ автопоиск в `PATH`). */
        @JvmField
        var serverPath: String = ""
    }

    private var state = State()

    override fun getState(): State = state

    override fun loadState(state: State) {
        this.state = state
    }

    /** Явный путь к серверу (пусто ⇒ автопоиск). */
    var serverPath: String
        get() = state.serverPath
        set(value) {
            state.serverPath = value
        }

    companion object {
        /** Экземпляр сервиса настроек уровня приложения. */
        fun getInstance(): LamLspSettings =
            ApplicationManager.getApplication().getService(LamLspSettings::class.java)
    }
}
