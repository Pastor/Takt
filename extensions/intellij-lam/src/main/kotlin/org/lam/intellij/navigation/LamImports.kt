package org.lam.intellij.navigation

import com.intellij.openapi.roots.ProjectRootManager
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFileSystemItem
import com.intellij.psi.PsiManager
import com.intellij.psi.TokenType
import org.lam.intellij.psi.LamTokenTypes

/**
 * Разрешение путей в директивах `import` (фича 0023, задача 0023-01).
 *
 * Навигация по `import` реализована через `GotoDeclarationHandler`
 * ([LamGotoDeclarationHandler]), а не через `PsiReferenceContributor`: листовые
 * токены (`LeafPsiElement`) не являются `ContributedReferenceHost`, поэтому
 * ссылки из контрибьютора к ним не привязываются. Ctrl/⌘+Click/`B` при этом
 * работает одинаково.
 */
object LamImports {

    /** Строковый токен — путь `import`, если ближайший слева значимый токен — `import`/`from`. */
    fun isImportPathElement(element: PsiElement): Boolean {
        if (element.node?.elementType != LamTokenTypes.STRING) return false
        var prev: PsiElement? = element.prevSibling
        while (prev != null) {
            val type = prev.node?.elementType
            val isTrivia = type == TokenType.WHITE_SPACE ||
                type == LamTokenTypes.LINE_COMMENT ||
                type == LamTokenTypes.DOC_COMMENT ||
                type == LamTokenTypes.BLOCK_COMMENT
            if (!isTrivia) {
                return type == LamTokenTypes.KEYWORD && (prev.text == "import" || prev.text == "from")
            }
            prev = prev.prevSibling
        }
        return false
    }

    /** Содержимое строки-пути (между кавычками) или `null`, если это не путь/пусто. */
    fun pathOf(element: PsiElement): String? {
        if (!isImportPathElement(element)) return null
        val text = element.text
        if (text.length < 2 || text[0] != '"') return null
        val contentEnd = if (text.last() == '"') text.length - 1 else text.length
        if (contentEnd <= 1) return null
        return text.substring(1, contentEnd)
    }

    /**
     * Резолвит путь: сначала относительно каталога текущего файла, затем от
     * корней контента проекта. Директории игнорируются (нужен файл).
     */
    fun resolve(element: PsiElement, path: String): PsiFileSystemItem? {
        if (path.isBlank()) return null
        val target = resolveVirtualFile(element, path) ?: return null
        return PsiManager.getInstance(element.project).findFile(target)
    }

    private fun resolveVirtualFile(element: PsiElement, path: String): VirtualFile? {
        val currentFile = element.containingFile?.originalFile?.virtualFile
            ?: element.containingFile?.virtualFile
        currentFile?.parent?.findFileByRelativePath(path)?.let { if (!it.isDirectory) return it }

        for (root in ProjectRootManager.getInstance(element.project).contentRoots) {
            root.findFileByRelativePath(path)?.let { if (!it.isDirectory) return it }
        }
        return null
    }
}
