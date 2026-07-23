package org.lam.intellij.parser

import com.intellij.lang.ASTNode
import com.intellij.lang.PsiBuilder
import com.intellij.lang.PsiParser
import com.intellij.psi.tree.IElementType
import org.lam.intellij.navigation.LamSymbolScanner
import org.lam.intellij.psi.LamElementTypes
import org.lam.intellij.psi.LamTokenTypes

/**
 * Почти плоский разборщик Lam (фича 0023 + 0067, Option B ADR 0067).
 *
 * Все токены — листья под корнем, **кроме** одиночных токенов, несущих ссылки/имена:
 * - строка-путь `import` (предыдущий значимый токен — `import`/`from`) → композит
 *   [LamElementTypes.IMPORT_PATH] (файловая `PsiReference`, R5);
 * - идентификатор-**декларация** → [LamElementTypes.NAME_DECL] (`PsiNamedElement`, R3);
 * - идентификатор-**использование** → [LamElementTypes.NAME_REF] (`PsiReference`, R3).
 *
 * Декларации отличаются от использований **эвристикой `LamSymbolScanner`** (единый
 * источник форм `kw <Id>`/`Import`/`enum`) — множество стартовых смещений деклараций
 * считается один раз по тексту файла. Больше ничего структурного не строится:
 * выражения/условия/типы остаются плоскими, грамматика не дублируется. Разбор всегда
 * успешен и не теряет текст (композит группирует те же листья).
 *
 * «Предыдущий значимый токен» — предыдущая итерация цикла: `PsiBuilder` пропускает
 * пробелы/комментарии, поэтому `builder.tokenType` отдаёт только значимые токены.
 */
class LamParser : PsiParser {
    override fun parse(root: IElementType, builder: PsiBuilder): ASTNode {
        val declStarts: Set<Int> =
            LamSymbolScanner.scan(builder.originalText).mapTo(HashSet()) { it.range.startOffset }
        val rootMarker = builder.mark()
        var prevType: IElementType? = null
        var prevText: String? = null
        while (!builder.eof()) {
            val type = builder.tokenType
            val text = builder.tokenText
            val offset = builder.currentOffset
            when {
                type == LamTokenTypes.STRING && prevType == LamTokenTypes.KEYWORD &&
                    (prevText == "import" || prevText == "from") ->
                    wrap(builder, LamElementTypes.IMPORT_PATH)

                type == LamTokenTypes.IDENTIFIER ->
                    wrap(builder, if (offset in declStarts) LamElementTypes.NAME_DECL else LamElementTypes.NAME_REF)

                else -> builder.advanceLexer()
            }
            prevType = type
            prevText = text
        }
        rootMarker.done(root)
        return builder.treeBuilt
    }

    /** Оборачивает текущий токен в композит `elementType` (один лист внутри). */
    private fun wrap(builder: PsiBuilder, elementType: IElementType) {
        val marker = builder.mark()
        builder.advanceLexer()
        marker.done(elementType)
    }
}
