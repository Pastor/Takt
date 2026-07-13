package org.lam.intellij

import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.junit.Assert.assertNotEquals

/**
 * Проверки каркаса 0022-01 (требование R1, критерий приёмки A1):
 * тип файла `.lam` заведён и корректно связан с языком Lam.
 */
class LamFileTypeTest : BasePlatformTestCase() {

    fun testDefaultExtensionIsLam() {
        assertEquals("lam", LamFileType.getDefaultExtension())
    }

    fun testFileTypeName() {
        assertEquals("Lam", LamFileType.name)
    }

    fun testFileTypeBoundToLamLanguage() {
        assertSame(LamLanguage, LamFileType.language)
        assertEquals("Lam", LamLanguage.id)
    }

    fun testLamFileIsRecognizedAsLamType() {
        val file = myFixture.configureByText("sample.lam", "start Start { next End; }")
        assertEquals(LamFileType, file.virtualFile.fileType)
        assertNotEquals("PLAIN_TEXT", file.virtualFile.fileType.name)
    }

    fun testIconLoads() {
        // Иконка типа файла должна загружаться из ресурсов (icons/lam.svg).
        assertNotNull(LamFileType.icon)
    }
}
