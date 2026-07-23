# Задача 0089-01: CT3 — неалфавитный символ → `BAD_CHARACTER`

> Фича: [../features/0089-intellij-residual-checks.md](../features/0089-intellij-residual-checks.md) · ADR: [../adr/0089-intellij-residual-checks.md](../adr/0089-intellij-residual-checks.md) · анализ: [../analyze/0089-intellij-residual-checks.md](../analyze/0089-intellij-residual-checks.md)

## Что было

Ветка `LamLexer.scanOperatorOrPunct` `else -> TokenType.BAD_CHARACTER` покрывала
любой неизвестный символ, но **сторожа-теста не было** (CT1 проверял лишь `==`).

## Что сделано

Добавлен юнит-тест `LamLexerTest.testArbitraryNonAlphaIsBadCharacter`: для набора
`@ $ \` \\ №` лексер даёт **ровно один** `BAD_CHARACTER`-токен на символ с текстом
= сам символ. Символы выбраны заведомо вне списка операторов/пунктуации языка
(`+ - * / % ! | & ^ ~ ? #` и скобки/пунктуация — это НЕ `BAD_CHARACTER`).

Только тест — **production-код не менялся** (поведение уже было). Стеки
`grammar`/`simulation` — н/п.

## Проверки

`cd extensions/intellij-lam && ./gradlew --offline test` — зелёный; новый тест
проходит, регресс 0022/0023 без правки ожиданий.
