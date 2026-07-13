package org.lam.intellij

import com.intellij.psi.PsiElement
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.lam.intellij.navigation.LamGotoDeclarationHandler

/**
 * Проверки перехода к декларации (фича 0023, критерии A2/A3).
 */
class LamGotoDeclarationTest : BasePlatformTestCase() {

    private val handler = LamGotoDeclarationHandler()

    /** Цели перехода из позиции каретки (маркер `<caret>` в тексте). */
    private fun targetsAtCaret(code: String): Array<PsiElement>? {
        myFixture.configureByText("test.lam", code)
        val element = myFixture.file.findElementAt(myFixture.caretOffset)
        return handler.getGotoDeclarationTargets(element, myFixture.caretOffset, myFixture.editor)
    }

    fun testJumpFromUsageToModelDeclaration() {
        val targets = targetsAtCaret(
            """
            model Producer { }
            start Main = Produ<caret>cer { }
            """.trimIndent(),
        )
        assertNotNull(targets)
        val target = targets!!.single()
        assertEquals("Producer", target.text)
        // Цель — имя в объявлении `model Producer`, т.е. первое вхождение.
        assertTrue(target.textRange.startOffset < myFixture.caretOffset)
    }

    fun testJumpFromTypeUsageToTypeDeclaration() {
        val targets = targetsAtCaret(
            """
            type Speed = bit;
            var v: Spe<caret>ed := 0;
            """.trimIndent(),
        )
        assertNotNull(targets)
        assertEquals("Speed", targets!!.single().text)
    }

    fun testJumpToImportedAlias() {
        val targets = targetsAtCaret(
            """
            import { SharedModel as M } from "shared.lam";
            start Entry = <caret>M { }
            """.trimIndent(),
        )
        assertNotNull(targets)
        assertEquals("M", targets!!.single().text)
    }

    fun testNoTargetOnKeyword() {
        assertNull(targetsAtCaret("mod<caret>el Foo { }"))
    }

    fun testNoTargetOnDeclarationItself() {
        // На самом объявлении (нет другого одноимённого) переход отсутствует.
        assertNull(targetsAtCaret("model Fo<caret>o { }"))
    }

    fun testNoTargetForUnknownIdentifier() {
        assertNull(targetsAtCaret("start Main = Unkno<caret>wn { }"))
    }
}
