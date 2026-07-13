package org.lam.intellij

import com.intellij.psi.PsiFile
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.lam.intellij.navigation.LamGotoDeclarationHandler

/**
 * Проверки навигации по директивам `import` к файлу (фича 0023, критерии A4/A5).
 *
 * Навигация ведётся `GotoDeclarationHandler` (см. [org.lam.intellij.navigation.LamImports]).
 */
class LamImportReferenceTest : BasePlatformTestCase() {

    private val handler = LamGotoDeclarationHandler()

    /** Цель перехода из строки-пути под кареткой (маркер `<caret>`). */
    private fun importTargetAtCaret(mainCode: String, addShared: Boolean = true): PsiFile? {
        if (addShared) myFixture.addFileToProject("shared.lam", "model SharedModel { }\n")
        myFixture.configureByText("main.lam", mainCode)
        val element = myFixture.file.findElementAt(myFixture.caretOffset)
        val targets = handler.getGotoDeclarationTargets(element, myFixture.caretOffset, myFixture.editor)
        return targets?.singleOrNull() as? PsiFile
    }

    fun testPlainImportResolvesToFile() {
        val file = importTargetAtCaret("""import "sha<caret>red.lam";""")
        assertNotNull(file)
        assertEquals("shared.lam", file!!.name)
    }

    fun testFromImportResolvesToFile() {
        val file = importTargetAtCaret("""import { SharedModel as M } from "sha<caret>red.lam";""")
        assertNotNull(file)
        assertEquals("shared.lam", file!!.name)
    }

    fun testImportAsResolvesToFile() {
        val file = importTargetAtCaret("""import "sha<caret>red.lam" as S;""")
        assertNotNull(file)
        assertEquals("shared.lam", file!!.name)
    }

    fun testMissingFileHasNoTarget() {
        // Файла нет — цели нет, без исключений.
        val file = importTargetAtCaret("""import "no_su<caret>ch.lam";""", addShared = false)
        assertNull(file)
    }

    fun testNonImportStringHasNoTarget() {
        // Строка вне import (в formula) не должна давать файловую навигацию.
        myFixture.addFileToProject("shared.lam", "model SharedModel { }\n")
        myFixture.configureByText("main.lam", """model M { always { formula "sha<caret>red.lam"; } }""")
        val element = myFixture.file.findElementAt(myFixture.caretOffset)
        val targets = handler.getGotoDeclarationTargets(element, myFixture.caretOffset, myFixture.editor)
        assertNull(targets)
    }
}
