package org.takt.intellij

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertNotNull
import org.junit.Test
import org.takt.intellij.scheme.TaktSchemeProtocol
import org.takt.intellij.scheme.TaktSchemeProtocol.Incoming
import java.io.File

class TaktSchemeProtocolTest {
    @Test
    fun layoutKeepsItsTextAndUsesModelSibling() {
        val text = "имя = \"a\\b\"\n"
        val outgoing = TaktSchemeProtocol.layoutMessage(text)
        assertEquals(Incoming.Layout(text), parse(outgoing))
        assertEquals(File("dir/lift.v2.takt-ui"), TaktSchemeProtocol.layoutFileOf(File("dir/lift.v2.takt")))
    }

    @Test
    fun incompleteMessagesCannotWriteLayoutOrMoveCaret() {
        assertNull(parse("""{"type":"layout"}"""))
        assertNull(parse("""{"type":"layout","text":"\u00"}"""))
        assertNull(parse("""{"type":"select","line":0,"character":2.5}"""))
        assertNull(parse("""{"type":"select","line":"0","character":2}"""))
        assertNull(parse("""{"type":"select","line":-1,"character":0}"""))
        assertEquals(Incoming.Select(0, 2), parse("""{"type":"select","line":0,"character":2}"""))
    }

    @Test
    fun exportRequestUsesOnlyTheActiveEditorContext() {
        assertEquals(Incoming.Export, parse("""{"type":"export","path":"/ignored.takt","args":["--format","mp4"]}"""))
    }

    @Test
    fun graphRequiresSheets() {
        assertNotNull(TaktSchemeProtocol.graphObject("""{"sheets":[]}"""))
        assertNull(TaktSchemeProtocol.graphObject("""{"sheets":null}"""))
        assertNull(TaktSchemeProtocol.graphObject("unexpected output"))
    }

    private fun parse(message: String) =
        TaktSchemeProtocol.parseObject(message)?.let(TaktSchemeProtocol::parse)
}
