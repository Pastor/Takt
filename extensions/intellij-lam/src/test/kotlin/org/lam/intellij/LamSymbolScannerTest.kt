package org.lam.intellij

import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.lam.intellij.navigation.LamSymbolScanner

/**
 * Проверки сканера деклараций (фича 0023, критерий A1).
 *
 * Формы деклараций сверяются с грамматикой `grammar/src/grammar.lalrpop`
 * (`model`/`state`/`start`/`type`/`enum`/`cond`/`var`/`const`/`fn` и `import`).
 */
class LamSymbolScannerTest : BasePlatformTestCase() {

    private fun names(src: String): List<String> = LamSymbolScanner.scan(src).map { it.name }

    fun testModelDeclaration() {
        assertEquals(listOf("Foo"), names("model Foo { }"))
    }

    fun testStateAndStart() {
        val n = names("start Main = A { } state Idle = B { }")
        assertTrue(n.contains("Main"))
        assertTrue(n.contains("Idle"))
    }

    fun testTypeEnumCond() {
        assertTrue(names("type Bar = bit;").contains("Bar"))
        assertTrue(names("enum Color { Red, Green }").contains("Color"))
        // Варианты enum не индексируются как декларации.
        assertFalse(names("enum Color { Red, Green }").contains("Red"))
        assertTrue(names("cond Ready = x = 1;").contains("Ready"))
    }

    fun testVarConstFn() {
        assertTrue(names("var x: bit := 0;").contains("x"))
        assertTrue(names("const K := 5;").contains("K"))
        assertTrue(names("fn helper() -> bit { return 0; }").contains("helper"))
        assertTrue(names("extern fn ext();").contains("ext"))
    }

    fun testImportRename() {
        val n = names("""import { SharedModel as M, SharedType as ST } from "shared.lam";""")
        assertTrue(n.contains("M"))
        assertTrue(n.contains("ST"))
        // Источники переименования (имена в импортируемом файле) не вводятся локально.
        assertFalse(n.contains("SharedModel"))
    }

    fun testImportBareList() {
        val n = names("""import { A, B } from "f.lam";""")
        assertTrue(n.contains("A"))
        assertTrue(n.contains("B"))
    }

    fun testImportAsAliases() {
        assertTrue(names("""import "p.lam" as P;""").contains("P"))
        assertTrue(names("""import * as Q from "p.lam";""").contains("Q"))
    }

    fun testPlainImportsIntroduceNoLocalNames() {
        assertTrue(names("""import "p.lam";""").isEmpty())
        assertTrue(names("import foo.bar;").isEmpty())
    }

    fun testDeclarationRangePointsToName() {
        val decl = LamSymbolScanner.scan("model Widget { }").single()
        assertEquals("Widget", decl.name)
        assertEquals("Widget", "model Widget { }".substring(decl.range.startOffset, decl.range.endOffset))
    }
}
