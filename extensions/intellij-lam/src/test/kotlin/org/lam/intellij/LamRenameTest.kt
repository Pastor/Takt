package org.lam.intellij

import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.lam.intellij.refactoring.LamNamesValidator

/**
 * Проверки нативного rename имён Lam (фича 0067, R3).
 *
 * Rename идёт штатным рефакторингом IDEA: декларация — `PsiNamedElement`
 * (`NAME_DECL`), использования — `PsiReference` (`NAME_REF`) к ней в том же файле.
 * Комментарии и строковые литералы не задеваются (в них нет ссылок).
 */
class LamRenameTest : BasePlatformTestCase() {

    private fun renameAtCaret(code: String, newName: String): String {
        myFixture.configureByText("test.lam", code)
        myFixture.renameElementAtCaret(newName)
        return myFixture.file.text
    }

    fun testRenameModelFromDeclaration() {
        val r = renameAtCaret(
            "model Produ<caret>cer { }\nstart Main = Producer { }\n", "Maker",
        )
        assertTrue("декларация: $r", r.contains("model Maker"))
        assertTrue("использование: $r", r.contains("= Maker"))
        assertFalse("старое имя не осталось: $r", r.contains("Producer"))
    }

    fun testRenameModelFromUsage() {
        val r = renameAtCaret(
            "model Producer { }\nstart Main = Produ<caret>cer { }\n", "Maker",
        )
        assertTrue(r.contains("model Maker"))
        assertTrue(r.contains("= Maker"))
        assertFalse(r.contains("Producer"))
    }

    fun testRenameType() {
        val r = renameAtCaret("type Sp<caret>eed = bit;\nvar v: Speed := 0;\n", "Rate")
        assertTrue(r.contains("type Rate"))
        assertTrue(r.contains(": Rate"))
    }

    fun testRenameVarFromUsage() {
        val r = renameAtCaret("var flag: bit := 0;\ncond Ready = fl<caret>ag;\n", "active")
        assertTrue(r.contains("var active"))
        assertTrue(r.contains("= active"))
    }

    fun testRenameEnumVariant() {
        val r = renameAtCaret(
            "enum Action { Idle = 670, Clo<caret>sing }\nvar a: Action := Closing;\n", "Shut",
        )
        assertTrue(r.contains("Shut }") || r.contains(", Shut"))
        assertTrue(r.contains(":= Shut"))
        assertFalse(r.contains("Closing"))
    }

    fun testRenamePort() {
        val r = renameAtCaret("in sen<caret>sors: u8 := 0x10;\ncond Occupied = sensors.0;\n", "sens")
        assertTrue(r.contains("in sens"))
        assertTrue(r.contains("= sens.0"))
    }

    fun testRenameFunction() {
        val r = renameAtCaret(
            "fn help<caret>er() -> bit { return 0; }\nfn caller() -> bit { return helper(); }\n", "util",
        )
        assertTrue(r.contains("fn util("))
        assertTrue(r.contains("return util("))
    }

    fun testRenameImportAlias() {
        myFixture.addFileToProject("shared.lam", "model SharedModel { }\n")
        val r = renameAtCaret(
            "import { SharedModel as <caret>M } from \"shared.lam\";\nstart Entry = M { }\n", "Sh",
        )
        assertTrue(r.contains("as Sh"))
        assertTrue(r.contains("= Sh"))
        // Путь import не задет.
        assertTrue(r.contains("\"shared.lam\""))
    }

    /** R3.2: одноимённые подстроки в комментарии и строке НЕ задеты. */
    fun testCommentAndStringNotTouched() {
        val r = renameAtCaret(
            "// Producer stays\nmodel Produ<caret>cer { always { formula \"Producer\"; } }\n", "Maker",
        )
        assertTrue("комментарий цел: $r", r.contains("// Producer stays"))
        assertTrue("строка цела: $r", r.contains("\"Producer\""))
        assertTrue("декларация переименована: $r", r.contains("model Maker"))
    }

    /** R3.3: валидатор отвергает ключевые слова Lam, принимает идентификаторы. */
    fun testNamesValidatorRejectsKeywords() {
        val v = LamNamesValidator()
        assertTrue(v.isKeyword("model", project))
        assertTrue(v.isKeyword("address", project))
        assertFalse(v.isKeyword("Producer", project))
        assertFalse("ключевое слово — не идентификатор", v.isIdentifier("state", project))
        assertTrue(v.isIdentifier("Producer", project))
        assertTrue(v.isIdentifier("_x1", project))
        assertFalse(v.isIdentifier("1abc", project))
        assertFalse(v.isIdentifier("", project))
    }
}
