package org.lam.intellij.lsp

import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.psi.PsiFile
import com.redhat.devtools.lsp4ij.features.semanticTokens.SemanticTokensColorsProvider
import org.lam.intellij.highlight.LamHighlighterColors

/**
 * Маппинг типов семантических токенов `lam-lsp` в цвета редактора (фича 0038,
 * задача 0038-02).
 *
 * Сервер отдаёт токены с типами из легенды `grammar::lsp::SEMANTIC_TOKEN_TYPES`
 * (10 типов). Провайдер сопоставляет **каждый** тип ключу [LamHighlighterColors],
 * чтобы семантический слой уважал палитру и настройки цветов пользователя (0022) и
 * не спорил с лексическим по цвету (R4/R5). Наслаивается **поверх** лексики: цвет
 * меняется только у идентификаторов, которые лексер красит одинаково.
 *
 * ⚠️ Набор ключей обязан покрывать легенду **целиком**: тип без маппинга молча
 * потеряет цвет. Сторож — `LamSemanticTokensColorsProviderTest`, читающий
 * `SEMANTIC_TOKEN_TYPES` из Rust-исходника (приём `LamKeywordSyncTest`).
 */
class LamSemanticTokensColorsProvider : SemanticTokensColorsProvider {

    override fun getTextAttributesKey(
        tokenType: String,
        tokenModifiers: List<String>,
        file: PsiFile,
    ): TextAttributesKey? = keyFor(tokenType)

    companion object {
        /**
         * Ключ цвета для типа токена легенды LSP (имена — как в
         * `lsp_types::SemanticTokenType`). `null` — тип вне легенды (цвет не
         * навязывается).
         */
        fun keyFor(tokenType: String): TextAttributesKey? = when (tokenType) {
            "keyword" -> LamHighlighterColors.KEYWORD
            "variable" -> LamHighlighterColors.IDENTIFIER
            "function" -> LamHighlighterColors.FUNCTION
            "type" -> LamHighlighterColors.TYPE
            "enumMember" -> LamHighlighterColors.ENUM_MEMBER
            "string" -> LamHighlighterColors.STRING
            "number" -> LamHighlighterColors.NUMBER
            "comment" -> LamHighlighterColors.LINE_COMMENT
            "operator" -> LamHighlighterColors.OPERATOR
            "class" -> LamHighlighterColors.CLASS
            else -> null
        }
    }
}
