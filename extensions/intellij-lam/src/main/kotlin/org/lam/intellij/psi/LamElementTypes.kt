package org.lam.intellij.psi

/**
 * Композитные узлы PSI-дерева Lam (фича 0067, Option B ADR 0067).
 *
 * Дерево остаётся **почти плоским** (ADR 0023): в композиты оборачиваются
 * **только** те одиночные токены, что несут ссылки/имена — иначе `PsiReference`
 * и `PsiNamedElement` невозможны (проба 0067 доказала: контрибьютор/ссылка не
 * привязываются к листовому `LeafPsiElement`). Выражения/условия/типы/приоритеты
 * НЕ оборачиваются и грамматику не дублируют.
 */
object LamElementTypes {
    /** Строка-путь директивы `import` (носитель файловой `PsiReference`, R5). */
    @JvmField val IMPORT_PATH = LamElementType("IMPORT_PATH")

    /** Идентификатор-**декларация** имени Lam (носитель `PsiNamedElement`, R3). */
    @JvmField val NAME_DECL = LamElementType("NAME_DECL")

    /** Идентификатор-**использование** имени Lam (носитель `PsiReference`, R3). */
    @JvmField val NAME_REF = LamElementType("NAME_REF")
}
