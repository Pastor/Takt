package org.takt.intellij.psi

import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiReferenceBase
import org.takt.intellij.navigation.TaktSymbolScanner

/**
 * Ссылка использования имени Takt на его декларацию в том же файле (фича 0067, R3).
 *
 * Резолв — эвристикой `TaktSymbolScanner` (первая одноимённая декларация файла; без
 * областей видимости — осознанное ограничение 0023). **Мягкая** (`soft = true`):
 * неразрешённое имя (кросс-файловое, имя состояния, встроенное) НЕ подсвечивается
 * ошибкой. Даёт нативный find usages и rename использований; кросс-файловость —
 * за LSP (0038).
 */
class TaktNameReference(element: TaktNameRef) :
    PsiReferenceBase<TaktNameRef>(element, TextRange(0, element.textLength), /* soft = */ true) {

    override fun resolve(): PsiElement? {
        val file = element.containingFile ?: return null
        val name = element.text
        val selfStart = element.textRange.startOffset
        val declRange = TaktSymbolScanner.scan(file.text)
            .firstOrNull { it.name == name && it.range.startOffset != selfStart }
            ?: return null
        val leaf = file.findElementAt(declRange.range.startOffset) ?: return null
        return leaf.parent as? TaktNameDecl ?: leaf
    }

    /** Rename использования — замена текста листа напрямую (без манипулятора). */
    override fun handleElementRename(newElementName: String): PsiElement {
        element.setIdentifierText(newElementName)
        return element
    }
}
