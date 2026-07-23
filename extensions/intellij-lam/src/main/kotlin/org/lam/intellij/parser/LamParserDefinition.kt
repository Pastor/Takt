package org.lam.intellij.parser

import com.intellij.extapi.psi.ASTWrapperPsiElement
import com.intellij.lang.ASTNode
import com.intellij.lang.ParserDefinition
import com.intellij.lang.PsiParser
import com.intellij.lexer.Lexer
import com.intellij.openapi.project.Project
import com.intellij.psi.FileViewProvider
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.psi.tree.IFileElementType
import com.intellij.psi.tree.TokenSet
import org.lam.intellij.LamLanguage
import org.lam.intellij.lexer.LamLexer
import org.lam.intellij.psi.LamElementTypes
import org.lam.intellij.psi.LamFile
import org.lam.intellij.psi.LamImportPath
import org.lam.intellij.psi.LamTokenSets

/**
 * Определение разбора файлов Lam (фича 0023, задача 0023-01).
 *
 * Разбор **плоский** ([LamParser]): токены [LamLexer] складываются листьями под
 * единственным корневым узлом [FILE]. Полноценного PSI-дерева нет (ADR 0023,
 * Option A) — цель лишь дать платформе реальные `PsiElement` под кареткой, чтобы
 * заработали `GotoDeclarationHandler` и `PsiReferenceContributor`. Подсветка
 * (0022) от этого не зависит и продолжает работать через `SyntaxHighlighter`.
 */
class LamParserDefinition : ParserDefinition {
    override fun createLexer(project: Project?): Lexer = LamLexer()

    override fun createParser(project: Project?): PsiParser = LamParser()

    override fun getFileNodeType(): IFileElementType = FILE

    override fun getCommentTokens(): TokenSet = LamTokenSets.COMMENTS

    override fun getStringLiteralElements(): TokenSet = LamTokenSets.STRINGS

    override fun getWhitespaceTokens(): TokenSet = LamTokenSets.WHITESPACES

    // Композит IMPORT_PATH (0067) → узел-носитель файловой ссылки; прочие
    // (если появятся) — безопасная обёртка. FILE обрабатывается createFile.
    override fun createElement(node: ASTNode): PsiElement = when (node.elementType) {
        LamElementTypes.IMPORT_PATH -> LamImportPath(node)
        else -> ASTWrapperPsiElement(node)
    }

    override fun createFile(viewProvider: FileViewProvider): PsiFile = LamFile(viewProvider)

    companion object {
        val FILE: IFileElementType = IFileElementType(LamLanguage)
    }
}
