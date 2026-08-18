# Задача 0161-01: Ренейм старых имён и гейт запрета

> Фича: [../features/0161-fixture-comments-rename.md](../features/0161-fixture-comments-rename.md) · ADR: [../adr/0161-fixture-comments-rename.md](../adr/0161-fixture-comments-rename.md) · анализ: [../analyze/0161-fixture-comments-rename.md](../analyze/0161-fixture-comments-rename.md)

## Что было

Ренейм [0100](../features/0100-language-rename-takt.md) оставил в рабочих файлах
имена `BuT`, `Lam`, `butc`, `lamc`, `lam-lsp`, расширения `.but`/`.lam` и пути
упразднённых крейтов `grammar/…`, `simulation/…`. Замер ADR насчитал 36 мест;
гейт, написанный в этой же задаче, нашёл ещё **8**, которых замер не увидел, —
все они ссылались на пути вида `simulation/tests/conformance_sv_tests.rs`, то
есть на файлы, которых нет в дереве **дважды** (крейт переименован 0100, тесты
перенесены [0244](../features/0244-test-target-build-cost.md)).

## Что сделано

**1. Ренейм в данных и комментариях (36 файлов).** Замены применены разом:
`BuT`/`Lam` → `Takt`, `butc`/`lamc` → `taktc`, `lam-lsp` → `takt-lsp`,
`.but`/`.lam` → `.takt`, `grammar/tests` → `takt-lang/tests`. Затронуто:
фикстуры `takt-lang/tests/data/**` (23) и `takt-sim/tests/data/**` (2), харнессы
`examples/generated/**` (8), корпус `examples/include/std.takt`, doc-комментарий
`takt-lang/src/semantic/unused.rs`, README пресетов графики.

**2. Устаревшие пути упразднённых крейтов (8 мест).** `simulation/src/…` →
`takt-sim/src/…`, `simulation/tests/X.rs` → фактическое место после 0244
(`takt-sim/tests/{sim,conformance}/X.rs`); каждый путь проверен на существование.
Историческая цитата в `takt-lang/tests/syntax/format_tests.rs` переписана без
старых имён (правило: истории место в `docs/`, ADR 0161).

**3. Команда `--graphics-config` в `examples/graphics-configs/README.md`.**
Переписана на действующий CLI. Правка имён её **не чинила**: флагов `--gif` и
`--gif-config` не существует вовсе — каталог кадров задаётся `-o`/`--output`,
файл настроек `--graphics-config`, вид вывода — полем `output_mode` внутри
файла. Команда исполнена (см. «Проверки»).

**4. Старое имя в выводе самого симулятора.** `takt-sim/src/bin/takt_sim.rs`:
`#[command(name = "simulation", …)]` → `name = "takt-sim"`, позиционный аргумент
`lam_file` → `model_file` (в `--help` было `<LAM_FILE>`, стало `<MODEL_FILE>`),
два комментария «файл LAM» → «файл модели». Ломающего изменения нет:
позиционный аргумент по имени в командной строке не называется, а `name` влияет
только на печать; потребителей строки `<LAM_FILE>` в дереве нет.

**5. Гейт `scripts/check-legacy-names.sh`.** POSIX `sh`, без зависимостей
(образец — `check-repo-url.sh`). Область — индекс `git ls-files`; исключения
заданы **префиксами путей** (`docs/`, `CHANGES.md`, `CLAUDE.md`, `AGENTS.md`,
`FEATURES.md`, `.claude/`, три гейта, описывающие сам переезд). Падает
**списком** мест с номерами строк и печатает таблицу замен.

⚠️ Границы слова записаны отрицаемыми классами `(^|[^A-Za-z0-9_])…`, а **не**
`\b` и не `[[:<:]]`: первый не входит в POSIX ERE, второй есть у BSD `grep` и
отсутствует у GNU — гейт обязан вести себя одинаково на машине разработчика и в
CI.

**6. Сторож гейта `scripts/test-legacy-names.sh`.** Мутацией проверяет, что
ловится **каждый** из десяти запрещённых шаблонов поимённо (шаблон, выпавший из
ERE, иначе молчит), что падение идёт списком, что исключение `docs/` действует
**на настоящем файле** (ADR самой фичи) и что граница гейта написана в его
заголовке. Оба скрипта включены в `precheck.sh` рядом с `check-repo-url.sh`.

## Проверки

```sh
scripts/check-legacy-names.sh     # OK: старых имён нет (проверено файлов: 1096)
scripts/test-legacy-names.sh      # 14 проверок, все пройдены
cargo test --all-features         # 3167 тестов, 0 провалов
cargo run --bin takt-sim -- examples/stacker.takt -n 12 \
    --output out/ --graphics-config examples/graphics-configs/dark.json
cargo run --bin takt-sim -- --help | grep -iE 'simulation|LAM_FILE'   # пусто
./scripts/precheck.sh
```

Соответствие условиям анализа: R1 — A1 (гейт), R2 — A2 (тесты зелены **без**
правки ожидаемых позиций, что и опровергает обоснование отказа 0100), R3 — A3
(GIF 398 КиБ создан), R3а — A3а (`--help` чист), R4 — A4 (сторож), R5 — A5
(граница в заголовке гейта + кандидат в `FEATURES.md`).
