package org.lam.intellij.parser

import com.intellij.lang.ASTNode
import com.intellij.lang.PsiBuilder
import com.intellij.lang.PsiParser
import com.intellij.psi.tree.IElementType
import org.lam.intellij.psi.LamElementTypes
import org.lam.intellij.psi.LamTokenTypes

/**
 * Почти плоский разборщик Lam (фича 0023 + 0067, Option B ADR 0067).
 *
 * Все токены — листья под корнем, **кроме** строки-пути `import`: если её
 * предыдущий значимый токен — ключевое слово `import`/`from`, она оборачивается в
 * композит [LamElementTypes.IMPORT_PATH] (носитель файловой `PsiReference`, R5).
 * Больше ничего структурного не строится — выражения/условия/типы остаются
 * плоскими, грамматика не дублируется. Разбор всегда успешен: любой набор токенов
 * принимается как есть (по построению не может потерять текст).
 *
 * «Предыдущий значимый токен» — это предыдущая итерация цикла: `PsiBuilder`
 * автоматически пропускает пробелы и комментарии (`getWhitespaceTokens`/
 * `getCommentTokens`), поэтому `builder.tokenType` отдаёт только значимые токены.
 */
class LamParser : PsiParser {
    override fun parse(root: IElementType, builder: PsiBuilder): ASTNode {
        val rootMarker = builder.mark()
        var prevType: IElementType? = null
        var prevText: String? = null
        while (!builder.eof()) {
            val type = builder.tokenType
            val text = builder.tokenText
            val isImportPath = type == LamTokenTypes.STRING &&
                prevType == LamTokenTypes.KEYWORD &&
                (prevText == "import" || prevText == "from")
            if (isImportPath) {
                val marker = builder.mark()
                builder.advanceLexer()
                marker.done(LamElementTypes.IMPORT_PATH)
            } else {
                builder.advanceLexer()
            }
            prevType = type
            prevText = text
        }
        rootMarker.done(root)
        return builder.treeBuilt
    }
}
