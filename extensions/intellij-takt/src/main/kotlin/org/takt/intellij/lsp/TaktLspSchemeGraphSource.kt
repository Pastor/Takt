package org.takt.intellij.lsp

import com.google.gson.JsonObject
import com.intellij.psi.PsiFile
import com.redhat.devtools.lsp4ij.LanguageServiceAccessor
import com.redhat.devtools.lsp4ij.client.features.FileUriSupport
import org.eclipse.lsp4j.TextDocumentIdentifier
import org.eclipse.lsp4j.jsonrpc.services.JsonRequest
import org.eclipse.lsp4j.services.LanguageServer
import org.takt.intellij.scheme.TaktSchemeGraphSource
import java.util.concurrent.CompletableFuture

interface TaktGraphLanguageServer : LanguageServer {
    @JsonRequest("takt/graph")
    fun graph(params: TaktGraphParams): CompletableFuture<JsonObject>
}

data class TaktGraphParams(val textDocument: TextDocumentIdentifier)

class TaktLspSchemeGraphSource : TaktSchemeGraphSource {
    override fun request(file: PsiFile): CompletableFuture<String> {
        val virtualFile = file.virtualFile
        // Вариант API с PsiFile подключает документ и дожидается didOpen.
        return LanguageServiceAccessor.getInstance(file.project)
            .getLanguageServers(file, { it.serverDefinition.id == TAKT_LSP_ID }, null)
            .thenCompose { servers ->
                val item = servers.firstOrNull() ?: error("Запустите Takt Language Server в LSP4IJ.")
                val uri = FileUriSupport.getFileUri(virtualFile, item.clientFeatures)
                    ?: error("LSP4IJ не определил URI модели.")
                val synchronizer = item.serverWrapper.getOpenedDocument(uri)?.synchronizer
                    ?: error("Модель не открыта на сервере Takt.")
                val server = item.server as TaktGraphLanguageServer
                synchronizer.flushPendingChanges()
                    .thenCompose { server.graph(TaktGraphParams(TextDocumentIdentifier(uri.toASCIIString()))) }
            }
            .thenApply { it.toString() }
    }
}
