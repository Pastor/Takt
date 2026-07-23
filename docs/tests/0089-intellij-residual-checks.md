# Тест-план фичи 0089: Остаточные проверки плагина IntelliJ (0022/0023)

> Фича: [../features/0089-intellij-residual-checks.md](../features/0089-intellij-residual-checks.md) · ADR: [../adr/0089-intellij-residual-checks.md](../adr/0089-intellij-residual-checks.md) · анализ: [../analyze/0089-intellij-residual-checks.md](../analyze/0089-intellij-residual-checks.md)

## Область и цель

Проверить CT3 (неалфавитный символ → `BAD_CHARACTER`) автотестом и зафиксировать
визуальный остаток `runIde`.

## Проверки (условие → ожидаемый результат)

| # | Проверка | Предусловие | Ожидаемый результат | R/A |
|---|---|---|---|---|
| T1 | `@` в лексер | — | ровно 1 `BAD_CHARACTER`, текст `@` | R1/A1 |
| T2 | `$`, `` ` ``, `\`, `№` | — | по 1 `BAD_CHARACTER` на символ | R1/A1 |
| T3 | контрпример: `#`/`?`/`!` | — | это `OPERATOR`, **не** `BAD_CHARACTER` (не в наборе теста; поведение по коду) | R1 |
| T4 | регресс лексера/подсветки | прежние тесты | зелёные без правки ожиданий | R3/A2 |
| T5 | визуальный показ цветом | `runIde` (GUI) | подсветка корректна — **визуальный остаток** | R2/A3 |

## Разбивка проверок по функциональности

| Функциональность | Статус |
|---|---|
| Плагин IntelliJ — лексер (CT3) | ✅ (T1–T4) |
| Визуальное (`runIde`) | ⬜ визуальный остаток (T5) |
| `grammar` / `simulation` | — (не затрагивается) |

<!-- Легенда: ✅ пройдено · ⬜ не проверялось · — не применимо -->

## Тестовые данные и окружение

- `cd extensions/intellij-lam && ./gradlew --offline test`; JDK 21, платформа IC
  2024.2.5. Тест-класс — `LamLexerTest` (`testArbitraryNonAlphaIsBadCharacter`).
- Контрпример (правило 16): символы-операторы (`# ? !`) — **не** `BAD_CHARACTER`
  (ветка `when (c)` в `scanOperatorOrPunct`), поэтому в набор CT3 не входят.
