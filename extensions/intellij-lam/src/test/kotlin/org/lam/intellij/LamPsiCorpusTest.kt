package org.lam.intellij

import com.intellij.psi.PsiFileFactory
import com.intellij.psi.util.PsiTreeUtil
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import com.intellij.psi.PsiFile
import java.io.File

/**
 * Антидивергентная сверка PSI с корпусом `.lam` (фича 0067, Option B ADR 0067).
 *
 * По образцу `grammar/tests/format_tests.rs` (форматтер по всему корпусу): парсер
 * прогоняется по **всем** `.lam` из `examples/` и `grammar/tests/data/` и обязан
 * (а) **round-trip байт-в-байт** — конкатенация текстов листьев PSI равна
 * исходнику (оборачивание одиночных токенов в композиты `IMPORT_PATH`/`NAME_DECL`/
 * `NAME_REF` не теряет и не переставляет текст); (б) **не давать `PsiErrorElement`**.
 *
 * ⚠️ Сверка **вердикта** с оракулом `lamc` (как планировалось в 0040) для Option B
 * **вырождена**: парсер тотальный (ADR 0023 — «разбор всегда успешен», принимает
 * любой поток токенов) и `PsiErrorElement` не порождает в принципе. Синтаксическую
 * валидность плагин не заявляет — это работа `lamc`/LSP (0038). Поэтому реальный
 * сторож здесь — round-trip: он ловит потерю/перестановку текста при будущем
 * углублении PSI (новый узел без покрытия завалит этот тест).
 */
class LamPsiCorpusTest : BasePlatformTestCase() {

    fun testRoundTripAndNoErrorsOverCorpus() {
        val root = repoRoot() ?: run {
            fail("корень репозитория не найден (нет examples/ и grammar/tests/data/)")
            return
        }
        val files = corpusFiles(root)
        assertTrue("корпус .lam неожиданно мал: ${files.size}", files.size >= 150)

        val factory = PsiFileFactory.getInstance(project)
        val roundTripFailures = ArrayList<String>()
        val errorFailures = ArrayList<String>()

        for (file in files) {
            val text = file.readText()
            val rel = file.relativeTo(root).path
            val psi: PsiFile = factory.createFileFromText(file.name, LamFileType, text)

            // Текст корневого AST-узла = конкатенация всех листьев в порядке дерева
            // (авторитетный round-trip: потеря/перестановка токена изменила бы его).
            if (psi.node.text != text) roundTripFailures.add(rel)
            if (PsiTreeUtil.hasErrorElements(psi)) errorFailures.add(rel)
        }

        assertTrue("round-trip PSI разошёлся с исходником: $roundTripFailures", roundTripFailures.isEmpty())
        assertTrue("неожиданные PsiErrorElement (парсер тотальный): $errorFailures", errorFailures.isEmpty())
    }

    /** Корень репозитория: подъём от рабочего каталога до `examples/` + `grammar/`. */
    private fun repoRoot(): File? {
        var dir: File? = File("").absoluteFile
        repeat(8) {
            if (dir != null && dir!!.resolve("examples").isDirectory &&
                dir!!.resolve("grammar/tests/data").isDirectory
            ) {
                return dir
            }
            dir = dir?.parentFile
        }
        return null
    }

    /** Все `.lam` корпуса: подкаталоги `examples/` и `grammar/tests/data/`. */
    private fun corpusFiles(root: File): List<File> {
        val roots = listOf(root.resolve("examples"), root.resolve("grammar/tests/data"))
        return roots.flatMap { base ->
            base.walkTopDown().filter { it.isFile && it.extension == "lam" }.toList()
        }.sorted()
    }
}
