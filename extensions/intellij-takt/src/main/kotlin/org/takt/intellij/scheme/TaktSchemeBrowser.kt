package org.takt.intellij.scheme

import com.google.gson.JsonObject
import com.intellij.openapi.util.Disposer
import com.intellij.ui.jcef.JBCefBrowser
import com.intellij.ui.jcef.JBCefBrowserBase
import com.intellij.ui.jcef.JBCefJSQuery
import org.cef.browser.CefBrowser
import org.cef.browser.CefFrame
import org.cef.callback.CefCallback
import org.cef.handler.CefLifeSpanHandlerAdapter
import org.cef.handler.CefLoadHandler.ErrorCode
import org.cef.handler.CefLoadHandlerAdapter
import org.cef.handler.CefRequestHandlerAdapter
import org.cef.handler.CefResourceHandlerAdapter
import org.cef.handler.CefResourceRequestHandlerAdapter
import org.cef.misc.BoolRef
import org.cef.misc.IntRef
import org.cef.misc.StringRef
import org.cef.network.CefRequest
import org.cef.network.CefResponse

private const val ORIGIN = "https://takt-scheme.invalid"
private const val PAGE = "$ORIGIN/scheme-host.html"
private val ASSET_PATH = Regex("[a-zA-Z0-9_-]+(?:/[a-zA-Z0-9_-]+)?(?:\\.[a-zA-Z0-9_-]+)+")

internal class TaktSchemeBrowser(
    onMessage: (JsonObject) -> Unit,
    onLoadFailure: (String) -> Unit,
) : JBCefBrowser() {
    private val query = JBCefJSQuery.create(this as JBCefBrowserBase)

    init {
        query.addHandler { raw ->
            TaktSchemeProtocol.parseObject(raw)?.let(onMessage)
            null
        }
        jbCefClient.addLifeSpanHandler(object : CefLifeSpanHandlerAdapter() {
            override fun onBeforePopup(browser: CefBrowser, frame: CefFrame, targetUrl: String, targetFrameName: String) =
                true
        }, cefBrowser)
        jbCefClient.addRequestHandler(object : CefRequestHandlerAdapter() {
            override fun onBeforeBrowse(
                browser: CefBrowser, frame: CefFrame, request: CefRequest, userGesture: Boolean, isRedirect: Boolean,
            ): Boolean = !frame.isMain || request.url != PAGE

            override fun getResourceRequestHandler(
                browser: CefBrowser, frame: CefFrame?, request: CefRequest, isNavigation: Boolean,
                isDownload: Boolean, requestInitiator: String?, disableDefaultHandling: BoolRef,
            ): CefResourceRequestHandlerAdapter {
                // Все ответы приходят из JAR. У браузера панели нет сетевых запросов.
                disableDefaultHandling.set(true)
                return object : CefResourceRequestHandlerAdapter() {
                    override fun getResourceHandler(
                        browser: CefBrowser, frame: CefFrame?, request: CefRequest,
                    ): CefResourceHandlerAdapter? = resource(request)
                }
            }
        }, cefBrowser)
        val connectBrowser = """
            window.taktSchemePost = (message) => {
              ${query.inject("JSON.stringify({...message, session: window.taktSchemeSession})")}
            };
            window.dispatchEvent(new Event('takt-scheme-connect'));
            """.trimIndent()
        jbCefClient.addLoadHandler(object : CefLoadHandlerAdapter() {
            override fun onLoadEnd(browser: CefBrowser, frame: CefFrame, httpStatusCode: Int) {
                if (!frame.isMain || frame.url != PAGE || httpStatusCode != 200) return
                browser.executeJavaScript(connectBrowser, PAGE, 0)
            }

            override fun onLoadError(
                browser: CefBrowser, frame: CefFrame, errorCode: ErrorCode, errorText: String, failedUrl: String,
            ) {
                if (frame.isMain && failedUrl == PAGE) onLoadFailure(errorText)
            }
        }, cefBrowser)
    }

    fun open() = loadURL(PAGE)

    fun send(message: String, session: String) {
        val json = TaktSchemeProtocol.quote(message)
        val sessionJson = TaktSchemeProtocol.quote(session)
        cefBrowser.executeJavaScript(
            """
            window.taktSchemeSession = $sessionJson;
            window.dispatchEvent(new MessageEvent('message', {
              data: JSON.parse($json), source: window, origin: location.origin
            }));
            """.trimIndent(), PAGE, 0,
        )
    }

    override fun dispose() {
        Disposer.dispose(query)
        super.dispose()
    }

    private fun resource(request: CefRequest): CefResourceHandlerAdapter? {
        if (request.method != "GET" || !request.url.startsWith("$ORIGIN/")) return null
        val path = request.url.removePrefix("$ORIGIN/")
        if (!ASSET_PATH.matches(path)) return null
        val mime = when (path.substringAfterLast('.')) {
            "html" -> "text/html"
            "js" -> "text/javascript"
            "css" -> "text/css"
            "json" -> "application/json"
            "woff2" -> "font/woff2"
            "woff" -> "font/woff"
            else -> return null
        }
        val bytes = javaClass.getResourceAsStream("/webview/$path")?.use { it.readBytes() } ?: return null
        return object : CefResourceHandlerAdapter() {
            private val content = bytes.inputStream()

            override fun processRequest(request: CefRequest, callback: CefCallback): Boolean {
                callback.Continue()
                return true
            }

            override fun getResponseHeaders(response: CefResponse, responseLength: IntRef, redirectUrl: StringRef) {
                response.status = 200
                response.mimeType = mime
                response.setHeaderMap(mapOf(
                    "Cache-Control" to "no-store",
                    "Content-Security-Policy" to "default-src 'self'; style-src 'self' 'unsafe-inline'; " +
                        "object-src 'none'; frame-src 'none'; form-action 'none'; base-uri 'none'",
                ))
                responseLength.set(bytes.size)
            }

            override fun readResponse(
                dataOut: ByteArray, bytesToRead: Int, bytesRead: IntRef, callback: CefCallback,
            ): Boolean {
                val count = content.read(dataOut, 0, bytesToRead)
                bytesRead.set(count.coerceAtLeast(0))
                return count > 0
            }
        }
    }
}
