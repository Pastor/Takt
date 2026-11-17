package org.takt.intellij.scheme

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.process.CapturingProcessHandler
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.PsiFile
import com.intellij.util.concurrency.AppExecutorUtil
import org.takt.intellij.lsp.TaktLspBinary
import org.takt.intellij.lsp.TaktLspSettings
import java.nio.file.Files
import java.util.concurrent.CompletableFuture

/** Реализация LSP4IJ регистрируется только из опционального дескриптора. */
interface TaktSchemeGraphSource {
    fun request(file: PsiFile): CompletableFuture<String>
}

internal fun graphFromProcess(file: VirtualFile, source: String): CompletableFuture<String> =
    CompletableFuture.supplyAsync({
        val binary = TaktLspBinary.resolve(TaktLspSettings.getInstance().serverPath)
            ?: error("Укажите путь к takt-lsp в настройках Takt Language Server.")
        val snapshot = Files.createTempFile("takt-scheme-", ".takt")
        try {
            // CLI читает путь. Снимок включает несохранённые изменения редактора.
            Files.writeString(snapshot, source)
            val command = GeneralCommandLine(binary.absolutePath, "--graph", snapshot.toString())
                .withWorkDirectory(file.parent.path)
                .withCharset(Charsets.UTF_8)
            val result = CapturingProcessHandler(command).runProcess(5000, true)
            check(!result.isTimeout) { "takt-lsp не ответил на запрос графа." }
            check(result.exitCode == 0) {
                result.stderr.trim().ifEmpty { "takt-lsp завершился с кодом ${result.exitCode}." }
            }
            result.stdout
        } finally {
            Files.deleteIfExists(snapshot)
        }
    }, AppExecutorUtil.getAppExecutorService())
