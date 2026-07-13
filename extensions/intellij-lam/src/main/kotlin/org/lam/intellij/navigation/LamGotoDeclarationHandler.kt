package org.lam.intellij.navigation

import com.intellij.codeInsight.navigation.actions.GotoDeclarationHandler
import com.intellij.openapi.editor.Editor
import com.intellij.psi.PsiElement
import org.lam.intellij.psi.LamFile
import org.lam.intellij.psi.LamTokenTypes

/**
 * Переход к декларации имени Lam (фича 0023, задача 0023-01).
 *
 * По идентификатору под кареткой ищет одноимённое объявление
 * ([LamSymbolScanner]) в том же файле и отдаёт платформе его листовой элемент
 * как цель `Go to Declaration` (`Ctrl/⌘+B`, `Ctrl/⌘+Click`). На самой
 * декларации и на не-идентификаторах молчит, не мешая штатному поведению.
 *
 * Дополнительно обрабатывает строки-пути директив `import` ([LamImports]):
 * `Ctrl/⌘+Click` по пути открывает соответствующий файл `.lam`.
 */
class LamGotoDeclarationHandler : GotoDeclarationHandler {

    override fun getGotoDeclarationTargets(
        sourceElement: PsiElement?,
        offset: Int,
        editor: Editor?,
    ): Array<PsiElement>? {
        val element = sourceElement ?: return null

        // Путь import → сам файл.
        if (element.node?.elementType == LamTokenTypes.STRING) {
            val path = LamImports.pathOf(element) ?: return null
            val target = LamImports.resolve(element, path) ?: return null
            return arrayOf(target)
        }

        if (element.node?.elementType != LamTokenTypes.IDENTIFIER) return null
        val file = element.containingFile as? LamFile ?: return null

        val name = element.text
        val caretStart = element.textRange.startOffset
        val targets = LamSymbolScanner.scan(file.text)
            .asSequence()
            .filter { it.name == name }
            // Исключаем декларацию, на которой уже стоит каретка (переход в себя не нужен).
            .filter { !it.range.contains(caretStart) }
            .mapNotNull { file.findElementAt(it.range.startOffset) }
            .toList()

        return if (targets.isEmpty()) null else targets.toTypedArray()
    }
}
