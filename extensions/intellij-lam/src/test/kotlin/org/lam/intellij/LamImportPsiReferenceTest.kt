package org.lam.intellij

import com.intellij.psi.PsiFile
import com.intellij.testFramework.fixtures.BasePlatformTestCase

/**
 * Проверки настоящей `PsiReference` на строке-пути `import` (фича 0067, R5).
 *
 * В отличие от навигации через `GotoDeclarationHandler` (0023, тест
 * [LamImportReferenceTest]), здесь проверяется именно **ссылка**: `resolve()` во
 * всех формах `import` и **rename-on-move** (переименование файла обновляет путь).
 */
class LamImportPsiReferenceTest : BasePlatformTestCase() {

    /** Ссылка под кареткой (маркер `<caret>`), либо `null`. */
    private fun refResolvesTo(mainCode: String, addShared: Boolean = true): PsiFile? {
        if (addShared) myFixture.addFileToProject("shared.lam", "model SharedModel { }\n")
        myFixture.configureByText("main.lam", mainCode)
        val ref = myFixture.getReferenceAtCaretPosition() ?: return null
        return ref.resolve() as? PsiFile
    }

    fun testPlainImportResolves() {
        val f = refResolvesTo("""import "sha<caret>red.lam";""")
        assertNotNull("ссылка должна резолвиться в файл", f)
        assertEquals("shared.lam", f!!.name)
    }

    fun testImportAsResolves() {
        val f = refResolvesTo("""import "sha<caret>red.lam" as S;""")
        assertNotNull(f); assertEquals("shared.lam", f!!.name)
    }

    fun testStarFromImportResolves() {
        val f = refResolvesTo("""import * as S from "sha<caret>red.lam";""")
        assertNotNull(f); assertEquals("shared.lam", f!!.name)
    }

    fun testNamedFromImportResolves() {
        val f = refResolvesTo("""import { SharedModel as M } from "sha<caret>red.lam";""")
        assertNotNull(f); assertEquals("shared.lam", f!!.name)
    }

    /** R5.2: переименование целевого файла обновляет строку-путь в тексте import. */
    fun testRenameTargetFileUpdatesImportPath() {
        val target = myFixture.addFileToProject("shared.lam", "model SharedModel { }\n")
        myFixture.configureByText("main.lam", """import "sha<caret>red.lam";""")
        assertNotNull("до rename ссылка должна быть", myFixture.getReferenceAtCaretPosition())

        myFixture.renameElement(target, "renamed.lam")

        val text = myFixture.file.text
        assertTrue("путь должен стать renamed.lam, получили: $text",
            text.contains("""import "renamed.lam";"""))
    }

    /** R5.3: битый путь — resolve() == null, без исключений. */
    fun testMissingFileResolvesToNull() {
        val f = refResolvesTo("""import "no_su<caret>ch.lam";""", addShared = false)
        assertNull(f)
    }

    /** Контрпример: строка вне import (в formula) ссылки НЕ несёт (регресс 0023). */
    fun testNonImportStringHasNoReference() {
        myFixture.addFileToProject("shared.lam", "model SharedModel { }\n")
        myFixture.configureByText(
            "main.lam",
            """model M { always { formula "sha<caret>red.lam"; } }""",
        )
        assertNull(myFixture.getReferenceAtCaretPosition())
    }
}
