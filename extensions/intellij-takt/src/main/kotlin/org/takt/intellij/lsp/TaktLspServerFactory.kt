package org.takt.intellij.lsp

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerFactory
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider

/**
 * Фабрика языкового сервера Takt для LSP4IJ (фича 0038, задача 0038-01).
 *
 * Регистрируется точкой расширения `com.redhat.devtools.lsp4ij.server`
 * (`takt-lsp4ij.xml`) и сопоставляется языку Takt через `languageMapping`. Поднимает
 * готовый сервер `takt-lsp` (фича 0011) по stdio — семантическую подсветку,
 * форматирование, hover и пр. отдаёт он, дублирования семантики на Kotlin нет
 * (единый источник — крейт `takt-lang`, драйвер 2 ADR).
 *
 * Реализуется только обязательный [createConnectionProvider]; `createLanguageClient`
 * и `getServerInterface` берутся из умолчаний LSP4IJ (стандартный клиент,
 * интерфейс `org.eclipse.lsp4j.services.LanguageServer`).
 */
class TaktLspServerFactory : LanguageServerFactory {

    override fun createConnectionProvider(project: Project): StreamConnectionProvider =
        TaktLspConnectionProvider()
}

/**
 * Транспорт запуска `takt-lsp` (stdio) поверх [OSProcessStreamConnectionProvider]
 * (рекомендован LSP4IJ — использует `OSProcessHandler` для отслеживания процесса).
 *
 * Тихая деградация (R3): если бинарник не разрешён ([TaktLspBinary.resolve] →
 * `null`), командная строка **не** задаётся, и `start()` бросает
 * `CannotStartProcessException`, которую LSP4IJ обрабатывает сама (сервер
 * показывается остановленным в консоли LSP, **без** модального диалога). Базовая
 * подсветка 0022 при этом не затронута.
 */
class TaktLspConnectionProvider : OSProcessStreamConnectionProvider() {
    init {
        val binary = TaktLspBinary.resolve(TaktLspSettings.getInstance().serverPath)
        if (binary != null) {
            commandLine = GeneralCommandLine(binary.absolutePath)
        }
    }
}
