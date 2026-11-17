package org.takt.intellij.scheme

import com.google.gson.JsonObject
import com.intellij.ide.ui.LafManagerListener
import com.intellij.openapi.Disposable
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.application.ModalityState
import com.intellij.openapi.application.ReadAction
import com.intellij.openapi.application.runWriteAction
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.editor.EditorFactory
import com.intellij.openapi.editor.ScrollType
import com.intellij.openapi.editor.event.CaretEvent
import com.intellij.openapi.editor.event.CaretListener
import com.intellij.openapi.editor.event.DocumentEvent
import com.intellij.openapi.editor.event.DocumentListener
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.FileEditorManagerListener
import com.intellij.openapi.fileEditor.FileEditorManagerEvent
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.vfs.SafeWriteRequestor
import com.intellij.openapi.vfs.VFileProperty
import com.intellij.openapi.vfs.VfsUtilCore
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.vfs.VirtualFileManager
import com.intellij.openapi.vfs.newvfs.BulkFileListener
import com.intellij.openapi.vfs.newvfs.events.VFileEvent
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.psi.PsiManager
import com.intellij.ui.JBColor
import com.intellij.ui.components.JBLabel
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.jcef.JBCefApp
import com.intellij.util.Alarm
import org.takt.intellij.TaktFileType
import java.awt.BorderLayout
import java.io.File
import java.io.IOException
import java.util.Locale
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit
import javax.swing.JPanel

class TaktSchemeToolWindowFactory : ToolWindowFactory {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val panel = JPanel(BorderLayout())
        val content = ContentFactory.getInstance().createContent(panel, "", false)
        val status = JBLabel()
        panel.add(status, BorderLayout.NORTH)
        if (JBCefApp.isSupported()) {
            val scheme = TaktSchemePanel(project, status)
            content.setDisposer(scheme)
            panel.add(scheme.browser.component, BorderLayout.CENTER)
            scheme.browser.open()
        } else {
            status.text = "Панели Takt требуется JCEF в среде выполнения IDE."
        }
        toolWindow.contentManager.addContent(content)
    }
}

private class TaktSchemePanel(
    private val project: Project,
    private val status: JBLabel,
) : Disposable, SafeWriteRequestor {
    val browser = TaktSchemeBrowser(
        onMessage = { message -> onEdt { receive(message) } },
        onLoadFailure = { error -> onEdt { showStatus("Не удалось загрузить панель схемы: $error") } },
    )
    private val refresh = Alarm(Alarm.ThreadToUse.SWING_THREAD, this)
    private var disposed = false
    private var ready = false
    private var target: Target? = null
    private var session = 0L
    private var revision = 0L
    private var paintedRevision = -1L

    private data class Target(
        val editor: Editor,
        val file: VirtualFile,
        val directory: VirtualFile,
        val layoutName: String,
    ) {
        val layoutPath get() = "${directory.path}/$layoutName"
        fun layout() = directory.findChild(layoutName)
    }

    init {
        project.messageBus.connect(this).apply {
            subscribe(FileEditorManagerListener.FILE_EDITOR_MANAGER, object : FileEditorManagerListener {
                override fun selectionChanged(event: FileEditorManagerEvent) = onEdt { bindEditor() }
            })
        }
        ApplicationManager.getApplication().messageBus.connect(this).apply {
            subscribe(LafManagerListener.TOPIC, LafManagerListener { onEdt(::sendTheme) })
            subscribe(VirtualFileManager.VFS_CHANGES, object : BulkFileListener {
                override fun after(events: List<VFileEvent>) {
                    val active = target ?: return
                    if (events.any { it.requestor !== this@TaktSchemePanel && it.file == active.file }) {
                        onEdt(::bindEditor)
                    }
                    if (events.any { it.requestor !== this@TaktSchemePanel && it.path == active.layoutPath }) {
                        onEdt { if (target == active) showLayout(active) }
                    }
                }
            })
        }
        EditorFactory.getInstance().eventMulticaster.apply {
            addDocumentListener(object : DocumentListener {
                override fun documentChanged(event: DocumentEvent) {
                    if (event.document == target?.editor?.document) scheduleGraph()
                }
            }, this@TaktSchemePanel)
            addCaretListener(object : CaretListener {
                override fun caretPositionChanged(event: CaretEvent) {
                    if (event.editor == target?.editor && event.caret == event.editor.caretModel.primaryCaret) sendCursor()
                }
            }, this@TaktSchemePanel)
        }
    }

    private fun onEdt(action: () -> Unit) {
        ApplicationManager.getApplication().invokeLater({
            if (!disposed && !project.isDisposed) action()
        }, ModalityState.nonModal())
    }

    private fun receive(raw: JsonObject) {
        val message = TaktSchemeProtocol.parse(raw) ?: return
        if (message == TaktSchemeProtocol.Incoming.Ready) {
            ready = true
            sendTheme()
            send(TaktSchemeProtocol.langMessage(Locale.getDefault().language))
            bindEditor(force = true)
            return
        }
        val active = target ?: return
        if (TaktSchemeProtocol.string(raw, "session") != session.toString()) return
        if (!active.file.isValid || active.editor.isDisposed) return
        if (active.editor != FileEditorManager.getInstance(project).selectedTextEditor ||
            active.file.parent != active.directory ||
            TaktSchemeProtocol.layoutFileOf(File(active.file.name))?.name != active.layoutName
        ) return
        when (message) {
            TaktSchemeProtocol.Incoming.Ready -> Unit
            is TaktSchemeProtocol.Incoming.Layout -> saveLayout(active, message.text)
            is TaktSchemeProtocol.Incoming.Select -> {
                if (paintedRevision == revision) select(active.editor, message)
            }
        }
    }

    private fun bindEditor(force: Boolean = false) {
        val editor = FileEditorManager.getInstance(project).selectedTextEditor
        val file = editor?.let { FileDocumentManager.getInstance().getFile(it.document) }
        val directory = file?.parent
        val next = if (editor != null && file != null && directory != null && file.isValid &&
            file.isInLocalFileSystem && file.fileType == TaktFileType
        ) {
            TaktSchemeProtocol.layoutFileOf(File(file.name))?.let { Target(editor, file, directory, it.name) }
        } else null
        if (!force && next == target) return
        target = next
        session++
        send(TaktSchemeProtocol.noGraphMessage())
        showLayout(next)
        scheduleGraph()
    }

    private fun scheduleGraph() {
        val request = ++revision
        refresh.cancelAllRequests()
        val active = target
        if (active == null) {
            showStatus("Откройте файл .takt.")
            return
        }
        if (!ready) return
        refresh.addRequest({
            val result = try {
                graph(active)
            } catch (error: Exception) {
                CompletableFuture.failedFuture(error)
            }
            result.orTimeout(10, TimeUnit.SECONDS).whenComplete { text, failure ->
                onEdt {
                    if (request == revision && active.file.isValid) {
                        val graph = text?.let(TaktSchemeProtocol::graphObject)
                        paintedRevision = if (graph != null) request else -1
                        send(graph?.let { TaktSchemeProtocol.graphMessage(it.toString()) }
                            ?: TaktSchemeProtocol.noGraphMessage())
                        showStatus(if (graph != null) null else
                            failure?.let { it.cause?.message ?: it.message } ?: "Сервер не вернул граф модели.")
                        sendCursor()
                    }
                }
            }
        }, 250)
    }

    private fun graph(active: Target): CompletableFuture<String> {
        val lsp = project.getService(TaktSchemeGraphSource::class.java)
        return if (lsp == null) {
            graphFromProcess(active.file, active.editor.document.immutableCharSequence.toString())
        } else {
            ReadAction.compute<CompletableFuture<String>, RuntimeException> {
                val psi = PsiManager.getInstance(project).findFile(active.file)
                    ?: error("Модель недоступна языковому серверу.")
                lsp.request(psi)
            }
        }
    }

    private fun showLayout(active: Target?) {
        try {
            val file = active?.layout()
            if (file != null) requireLayoutFile(file)
            send(TaktSchemeProtocol.layoutMessage(file?.let(VfsUtilCore::loadText) ?: ""))
        } catch (error: IOException) {
            send(TaktSchemeProtocol.layoutMessage(""))
            showStatus("Не удалось прочитать раскладку: ${error.localizedMessage}")
        }
    }

    private fun saveLayout(active: Target, text: String) {
        try {
            runWriteAction {
                val existing = active.layout()
                val documents = FileDocumentManager.getInstance()
                val document = existing?.let(documents::getCachedDocument)
                if (document != null && documents.isDocumentUnsaved(document)) {
                    throw IOException("Сначала сохраните изменения в ${active.layoutName}.")
                }
                val file = existing ?: active.directory.createChildData(this, active.layoutName)
                requireLayoutFile(file)
                try {
                    file.setBinaryContent(text.toByteArray(Charsets.UTF_8), -1, -1, this)
                } catch (error: IOException) {
                    if (existing == null) {
                        try {
                            file.delete(this)
                        } catch (cleanup: IOException) {
                            error.addSuppressed(cleanup)
                        }
                    }
                    throw error
                }
            }
        } catch (error: IOException) {
            showLayout(active)
            showStatus("Не удалось сохранить раскладку: ${error.localizedMessage}")
        }
    }

    private fun requireLayoutFile(file: VirtualFile) {
        if (file.isDirectory || file.`is`(VFileProperty.SYMLINK)) {
            throw IOException("${file.name} должен быть обычным файлом.")
        }
    }

    private fun select(editor: Editor, message: TaktSchemeProtocol.Incoming.Select) {
        val document = editor.document
        if (message.line >= document.lineCount) return
        val start = document.getLineStartOffset(message.line)
        if (message.character > document.getLineEndOffset(message.line) - start) return
        editor.caretModel.moveToOffset(start + message.character)
        editor.scrollingModel.scrollToCaret(ScrollType.CENTER)
    }

    private fun sendCursor() {
        if (paintedRevision != revision) return
        val editor = target?.editor ?: return
        val offset = editor.caretModel.offset
        val line = editor.document.getLineNumber(offset)
        send(TaktSchemeProtocol.cursorMessage(line, offset - editor.document.getLineStartOffset(line)))
    }

    private fun sendTheme() = send(TaktSchemeProtocol.themeMessage(!JBColor.isBright()))

    private fun send(message: String) {
        if (ready) browser.send(message, session.toString())
    }

    private fun showStatus(message: String?) {
        status.text = message?.replace('<', '‹')
        status.isVisible = message != null
    }

    override fun dispose() {
        disposed = true
        Disposer.dispose(browser)
    }
}
