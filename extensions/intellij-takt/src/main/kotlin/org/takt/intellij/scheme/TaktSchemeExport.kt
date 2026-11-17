package org.takt.intellij.scheme

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.process.CapturingProcessHandler
import com.intellij.openapi.progress.ProgressIndicator
import org.takt.intellij.run.TaktCommandLine
import org.takt.intellij.run.configuredTaktTools
import java.io.File

internal fun exportScheme(path: String, output: File, workDirectory: String, indicator: ProgressIndicator) {
    val built = TaktCommandLine.build(
        TaktCommandLine.Mode.EXPORT,
        TaktCommandLine.Params(filePath = path, outputDir = output.path),
        configuredTaktTools(),
    )
    val args = when (built) {
        is TaktCommandLine.Result.Ready -> built.command
        is TaktCommandLine.Result.Refused -> error(built.message)
    }
    val command = GeneralCommandLine(args).withWorkDirectory(workDirectory).withCharset(Charsets.UTF_8)
    val result = CapturingProcessHandler(command).runProcessWithProgressIndicator(indicator)
    indicator.checkCanceled()
    check(result.exitCode == 0) {
        result.stderr.trim().ifEmpty { "takt-sim завершился с кодом ${result.exitCode}." }
    }
}
