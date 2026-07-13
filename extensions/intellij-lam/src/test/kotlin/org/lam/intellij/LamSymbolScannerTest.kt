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
        assertTrue(names("cond Ready = x = 1;").contains("Ready"))
    }

    fun testEnumVariantsAreDeclarations() {
        // Константы enum индексируются — переход к декларации от их использования.
        val n = names("enum Color { Red, Green = 5, Blue }")
        assertTrue(n.contains("Red"))
        assertTrue(n.contains("Green"))
        assertTrue(n.contains("Blue"))
        // Значение `= 5` — число, не имя варианта.
        assertEquals(4, n.count { it in setOf("Color", "Red", "Green", "Blue") })
    }

    fun testEnumVariantRangePointsToName() {
        val src = "enum Color { Red, Green }"
        val green = LamSymbolScanner.scan(src).single { it.name == "Green" }
        assertEquals("Green", src.substring(green.range.startOffset, green.range.endOffset))
    }

    fun testPortDeclarations() {
        // Порты in/out/inout объявляют имя — используются как `port.N`.
        assertTrue(names("in sensors_cab: u8 := 0x10000009;").contains("sensors_cab"))
        assertTrue(names("out relay: bit := 0;").contains("relay"))
        assertTrue(names("inout bus: u8;").contains("bus"))
    }

    fun testPortRangePointsToName() {
        val src = "in sensors_cab: u8 := 0x10000009;"
        val decl = LamSymbolScanner.scan(src).single { it.name == "sensors_cab" }
        assertEquals("sensors_cab", src.substring(decl.range.startOffset, decl.range.endOffset))
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
