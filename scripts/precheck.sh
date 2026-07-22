#!/usr/bin/env bash
# Предкоммит-проверка: ссылки в Markdown + fmt + check + clippy + test +
# формат примеров (lamc fmt --check) +
# генерация C/PlantUML/ST/Rust/SV из примеров Lam + гейт воспроизводимости
# (фича 0048) + сборка сгенерированного кода (C — cmake/ninja, Rust — cargo +
# прогон проверок по моделям, SV — verilator + yosys).
# Запускать из любого каталога.
set -euo pipefail

if command -v rtk &>/dev/null; then
  CARGO_CMD="rtk cargo"
else
  CARGO_CMD="cargo"
fi

# СТРОГИЙ РЕЖИМ (фича 0090). В CI отсутствие любого инструмента гейта — ОШИБКА,
# а не мягкий пропуск: пропущенный гейт зелёный, то есть неотличим от пройденного
# (урок 0045). Локально (умолчание PRECHECK_STRICT=0) поведение прежнее: нет
# инструмента → [пропуск], сборка не падает — машина разработчика вправе быть без
# verilator/iec2c. SV_GATE_REQUIRED (фича 0045) теперь НАСЛЕДУЕТ общий флаг;
# явно заданное значение по-прежнему уважается (обратная совместимость).
PRECHECK_STRICT="${PRECHECK_STRICT:-0}"
export SV_GATE_REQUIRED="${SV_GATE_REQUIRED:-$PRECHECK_STRICT}"

# require_tool <бинарник> <назначение/как поставить>: единая точка мягкого
# пропуска. Под PRECHECK_STRICT=1 отсутствие инструмента валит скрипт; иначе —
# прежний [пропуск]. Код возврата (для `if require_tool …; then <гейт>; fi`):
# 0 — инструмент есть, гейт выполнять; 1 — мягкий пропуск, гейт не выполнять.
require_tool() {
  command -v "$1" >/dev/null 2>&1 && return 0
  if [ "$PRECHECK_STRICT" = "1" ]; then
    echo "  ОШИБКА: $1 не найден, а PRECHECK_STRICT=1 — гейт обязателен ($2)." >&2
    exit 1
  fi
  echo "  [пропуск] $1 не найден — гейт пропущен ($2)"
  return 1
}

if require_tool python3 "проверка ссылок, правило 14; apt install python3"; then
  echo "Проверка ссылок в Markdown (правило 14)..."
  "$(dirname "$0")/check-links.py"
fi

# Формат: под строгим режимом ПРОВЕРЯЕТСЯ (падение на неканоне), а не применяется
# молча — иначе CI переформатировал бы неканоничный коммит и прошёл бы мимо него.
# Локально — прежнее применение (удобство разработчика, фича 0024).
if [ "$PRECHECK_STRICT" = "1" ]; then
  $CARGO_CMD +nightly fmt --check
else
  $CARGO_CMD +nightly fmt
fi
$CARGO_CMD check
# Ноль-долг предупреждений закреплён фичей 0046: `-D warnings` на clippy валит
# сборку на ЛЮБОМ предупреждении (clippy И rustc — clippy гоняет оба набора).
# Это CLI-уровень (не запрещённый `#![deny(warnings)]` в коде, docs/CODE.md):
# обновление компилятора может добавить предупреждение — тогда его надо
# устранить, а не копить. Точечные исключения — через `#[allow(...)]` с
# обоснованием у места (как ~38 существующих).
$CARGO_CMD clippy --all-targets --all-features -- -D warnings

# Размер модулей (фича 0027): быстрая проверка идёт ДО долгих тестов — падать
# надо раньше, а не после трёх минут прогона.
"$(dirname "$0")/check-module-size.sh"

# Реестр кодов диагностик (фича 0077): каждый код диагностики согласован с
# docs/diagnostics/README.md. Тоже быстрая — до тестов.
"$(dirname "$0")/check-diagnostic-codes.sh"

$CARGO_CMD test -- --test-threads=1

# Корень репозитория = каталог этого скрипта /..
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

$CARGO_CMD build --bin lamc 2>/dev/null
$CARGO_CMD build --features lsp --bin lam-lsp

LAMC="./target/debug/lamc"

# Канон форматирования примеров (фича 0024). Проверка НЕразрушающая: только код
# возврата. Область — `examples/`: они являются документацией по языку
# (правила 15, 16) и обязаны быть в каноне. Фикстуры `tests/data/` намеренно НЕ
# проверяются: часть тестов завязана на их раскладку и позиции.
echo "Проверка формата примеров (lamc fmt --check)..."
$LAMC fmt --check examples/ || {
  echo "  Примеры не в каноне. Исправить: $LAMC fmt examples/"
  exit 1
}

C_OUTPUT="examples/generated/c"
PLANTUML_OUTPUT="examples/generated/plantuml"
ST_OUTPUT="examples/generated/st"
RUST_OUTPUT="examples/generated/rust"
SV_OUTPUT="examples/generated/sv"
SV_MMIO_OUTPUT="examples/generated/sv-mmio"

# Примеры, которые цель `sv` ОБЯЗАНА транслировать (фича 0045, задача 0045-02).
# Список ведётся ЯВНО, а не выводится как «все, кроме падающих»: иначе пример,
# переставший транслироваться, молча выпал бы из гейта — и гейт замолчал бы
# регресс.
#
# Остальные три отвергаются ЗАКОНОМЕРНО — это границы цели, а не недоделка
# (README, раздел «Генерация SystemVerilog»). Причины проверены прогоном
# 2026-07-16 и оказались НЕ теми, что предполагали ADR и план задачи («все три
# отсекаются по extern fn / SV-005»): до `extern fn` дело у двух из них просто
# не доходит.
#
#   elevator.lam        SV-005  extern fn (8 шт.) — как и предполагалось
#   comprehensive.lam   SV-002  цикл `for`: в синтезируемом RTL цикл обязан
#                               разворачиваться в схему, то есть иметь границы,
#                               известные на этапе синтеза
#   extend_complex.lam  SV-005  extern fn `has_flag` (плюс struct/битовый доступ
#                               `x.2`). Композиция `A + B + (C|D) + E` целью sv
#                               теперь ПОДДЕРЖАНА (фича 0057) — пример остаётся вне
#                               гейта по НЕ связанным с композицией причинам.
SV_TRANSLATABLE="stacker elevator_mini regulator pid_regulator"

# Примеры для цели `sv-mmio` (фича 0062): порты с адресом → регистровый файл на
# шине. Демонстрируется на `stacker` — его 17 адресов (`0x100`…`0x601`) суть
# готовая карта регистров. Прочие sv-транслируемые примеры адресов не несут: под
# `sv-mmio` они дали бы обычный модуль без регистрового файла, то есть дубль
# гейта `sv`. Единственный содержательный пример — `stacker`.
SV_MMIO_TRANSLATABLE="stacker"

# Примеры на `float` (фича 0096, «прозрачный float»), которым цель `sv` требует
# понижения float→q(m.n) флагом `--float-as-q`: нативного float в синтезируемом
# RTL нет, без флага — `SV-003`. float→q(8,8) байт-в-байт равно явному q
# (0096-02), поэтому committed `.sv` таких примеров НЕ меняется от перевода
# исходника с q на float. Значение — точность q(m.n). Цели `c`/`rust`/`st` берут
# float нативно (double/f64/LREAL); встраиваемые `c-hal`/`st-at` — q через
# `--float-embedded` (отдельный гейт ниже).
FLOAT_AS_Q_EXAMPLES="pid_regulator"
FLOAT_AS_Q_PREC="8.8"

# Доп. флаги генерации `sv` для float-примера (пусто для остальных).
sv_float_flags() {  # $1 = имя примера
  case " $FLOAT_AS_Q_EXAMPLES " in
    *" $1 "*) printf -- '--float-as-q=%s' "$FLOAT_AS_Q_PREC" ;;
    *) : ;;
  esac
}

echo "Генерация C-кода из примеров Lam..."
for lam_file in examples/*.lam; do
  name="$(basename "$lam_file" .lam)"
  echo "  $lam_file → $C_OUTPUT/${name}.c / ${name}.h"
  $LAMC compile "$lam_file" -o "$C_OUTPUT" || echo "    [предупреждение] ошибка генерации $lam_file"
  $LAMC compile "$lam_file" -t plantuml -o "$PLANTUML_OUTPUT" || echo "    [предупреждение] ошибка генерации $lam_file"
  # Цель st (фича 0041). Отказ не валит предкоммит: бэкенд дописывается
  # (задачи 0041-03, 0041-04 часть 3), и на непокрытом узле он ЗАКОНОМЕРНО
  # отвечает ST-011 — это замысел («никакого тихого пропуска»), а не поломка.
  $LAMC compile "$lam_file" -t st -o "$ST_OUTPUT" \
    || echo "    [предупреждение] цель st: $lam_file не транслируется (бэкенд не закончен)"
  # Цель rust (фича 0050). Отказ не валит предкоммит по той же причине, что и у
  # st: на непокрытом узле бэкенд ЗАКОНОМЕРНО отвечает RS-0xx — это замысел
  # («никакого тихого пропуска»), а не поломка. Что именно не покрыто —
  # перечислено в README, раздел «Генерация Rust».
  $LAMC compile "$lam_file" -t rust -o "$RUST_OUTPUT" \
    || echo "    [предупреждение] цель rust: $lam_file не транслируется (бэкенд не закончен)"
  # Цель sv (фича 0045). Отказ здесь не валит предкоммит, но и не остаётся
  # безнаказанным: список обязательных примеров проверяется ниже отдельно
  # ($SV_TRANSLATABLE) — иначе выпадение примера из гейта прошло бы молча.
  # float-примерам (0096) добавляется --float-as-q (иначе SV-003).
  # shellcheck disable=SC2046,SC2086
  $LAMC compile "$lam_file" -t sv $(sv_float_flags "$name") -o "$SV_OUTPUT" \
    || echo "    [предупреждение] цель sv: $lam_file не транслируется"
done

# Цель sv-mmio (фича 0062): регистровый файл из адресов портов — только stacker
# (см. $SV_MMIO_TRANSLATABLE). Отказ не валит предкоммит здесь: обязательность
# проверяется гейтом ниже (как у sv).
for name in $SV_MMIO_TRANSLATABLE; do
  $LAMC compile "examples/${name}.lam" -t sv-mmio -o "$SV_MMIO_OUTPUT" \
    || echo "    [предупреждение] цель sv-mmio: examples/${name}.lam не транслируется"
done
echo "Готово. Файлы в $C_OUTPUT/"

# ГЕЙТ ВОСПРОИЗВОДИМОСТИ (фича 0048): два прогона компилятора на одном входе
# обязаны давать байт-в-байт одинаковый вывод. Порядок эмиссии — свойство общего
# слоя (BTreeMap), а не привычка бэкенда; забытый источник недетерминизма не
# ломает сборку и не ловится юнит-тестом — он проявляется случайно, раз в
# несколько прогонов. Гейт ЖЁСТКИЙ: внешних инструментов не требует (сравнение —
# diff -r), поэтому мягкого пропуска нет (в отличие от гейта ST).
#
# Сравниваются каталоги двух прогонов, а не сам факт генерации: если цель на
# каком-то примере не транслируется, оба прогона одинаково пусты → это не
# недетерминизм. Цель `st` детерминирована уже сегодня — служит контрольной
# точкой правки общего слоя.
echo "Гейт воспроизводимости: два прогона на каждый пример × цель..."
repro_failed=0
for lam_file in examples/*.lam; do
  name="$(basename "$lam_file" .lam)"
  for spec in "c:" "c-hal:-t c-hal" "plantuml:-t plantuml" "st:-t st" "st-at:-t st-at" "rust:-t rust" "sv:-t sv" "sv-mmio:-t sv-mmio"; do
    tgt="${spec%%:*}"
    flag="${spec#*:}"
    # float-примерам (0096) цели sv/sv-mmio требуют --float-as-q — иначе оба
    # прогона пусты (SV-003) и детерминизм вывода не проверился бы вовсе.
    if [ "$tgt" = "sv" ] || [ "$tgt" = "sv-mmio" ]; then
      flag="$flag $(sv_float_flags "$name")"
    fi
    d1="$(mktemp -d)"
    d2="$(mktemp -d)"
    # shellcheck disable=SC2086
    $LAMC compile "$lam_file" $flag -o "$d1" >/dev/null 2>&1 || true
    # shellcheck disable=SC2086
    $LAMC compile "$lam_file" $flag -o "$d2" >/dev/null 2>&1 || true
    if diff -r "$d1" "$d2" >/dev/null 2>&1; then
      echo "  $name [$tgt] → воспроизводим"
    else
      echo "  $name [$tgt] → НЕДЕТЕРМИНИЗМ (два прогона разошлись):"
      diff -r "$d1" "$d2" 2>&1 | sed 's/^/    /' | head -8
      repro_failed=1
    fi
    rm -rf "$d1" "$d2"
  done
done
if [ "$repro_failed" -ne 0 ]; then
  echo "  Генерация недетерминирована — предкоммит провален (фича 0048)."
  exit 1
fi

# ГЕЙТ ПРЕДСТАВЛЕНИЯ FLOAT (фича 0096, применение к примерам): пример на `float`
# на встраиваемом профиле без FPU (`c-hal`/`st-at`) обязан давать q через
# `--float-embedded --float-as-q` — «q там, где аппаратного float нет». Гейт
# проверяет, что понижение float→q СОБИРАЕТСЯ (не тихо теряется): q-путь у
# c-hal/st-at — то же ядро, что и явный q, и потактово сверен с симулятором в
# conformance_float_*_tests. Жёсткий: флаги и ядро 0096 — уже в репозитории.
echo "Гейт представления float: встраиваемые профили (c-hal/st-at) → q..."
float_embed_failed=0
for name in $FLOAT_AS_Q_EXAMPLES; do
  for etgt in c-hal st-at; do
    d="$(mktemp -d)"
    if $LAMC compile "examples/${name}.lam" -t "$etgt" \
         --float-embedded --float-as-q="$FLOAT_AS_Q_PREC" -o "$d" >/dev/null 2>&1; then
      echo "  $name [$etgt] → q сформирован (--float-embedded --float-as-q=$FLOAT_AS_Q_PREC)"
    else
      echo "  $name [$etgt] → float→q НЕ собрался под --float-embedded"
      float_embed_failed=1
    fi
    rm -rf "$d"
  done
done
if [ "$float_embed_failed" -ne 0 ]; then
  echo "  float→q для встраиваемых профилей не собрался — предкоммит провален (фича 0096)."
  exit 1
fi

# ГЕЙТ ЦЕЛИ RUST (фича 0050, задача 0050-02): порождённый .rs обязан приниматься
# rustc И clippy. Гейт ЖЁСТКИЙ и мягкого пропуска не имеет: инструменты — уже
# зависимости проекта (это Rust-репозиторий), ставить нечего. Тем он и дешевле
# соседей: арбитр ST (`iec2c`) собирается из исходников, арбитр SV требует
# `brew install verilator yosys`.
#
# ПОЧЕМУ ОБЁРТКА, А НЕ ФАЙЛ НАПРЯМУЮ. Порождённый модуль НЕ содержит `#![no_std]`:
# этот атрибут допустим только в корне крейта, и в файле, подключённом через
# `mod`, он даёт предупреждение «can only be used at the crate root» — то есть
# ломал бы сборку пользователя под -D warnings. Поэтому no_std-совместимость
# проверяется так, как модуль и будет использоваться: он кладётся в корень с
# `#![no_std]`. Это строже проверки файла в одиночку — обращение к std всплывёт
# именно здесь.
#
# ПОЛИТИКА ЛИНТОВ (R9, решение варианта (а) задачи 0050-02): `-D warnings`.
# Пробы 2026-07-16: калька с C гейт НЕ проходит (`dead_code` на вариантах,
# которые C эмитит всегда, а модель не конструирует). Выбрано «не эмитить
# недостижимое», а не `#[allow]`: сторож должен остаться живым. Приватность
# перечислений состояний — часть этого решения (у `pub enum` dead_code не
# срабатывает вовсе, то есть публичность = глушение линта видимостью).
echo "Гейт цели rust: rustc + clippy по порождённым .rs..."
rust_failed=0
rust_gate_dir="$(mktemp -d)"
for rs_file in "$RUST_OUTPUT"/*.rs; do
  [ -e "$rs_file" ] || continue
  name="$(basename "$rs_file" .rs)"
  # Пропусков нет: единственный, что здесь был (`comprehensive` —
  # clippy::absurd_extreme_comparisons на `temperature <= 0` при `u8`), снят
  # фичей 0030 вместе с причиной. Пример больше не сравнивает беззнаковое с
  # нулём: порог выхода из охлаждения выражен именованным условием
  # `cond Cooled = temperature = 0`.
  wrapper="$rust_gate_dir/gate_${name}.rs"
  {
    echo "#![no_std]"
    echo "#[path = \"$(cd "$(dirname "$rs_file")" && pwd)/${name}.rs\"]"
    echo "pub mod generated;"
  } > "$wrapper"
  if rustc --edition 2021 --crate-type=lib -D warnings "$wrapper" \
      --out-dir "$rust_gate_dir/out" 2>"$rust_gate_dir/${name}.rustc"; then
    echo "  $name → rustc принял"
  else
    echo "  $name → rustc ОТВЕРГ:"
    sed 's/^/    /' "$rust_gate_dir/${name}.rustc" | head -12
    rust_failed=1
  fi
  if clippy-driver --edition 2021 --crate-type=lib -D warnings "$wrapper" \
      --out-dir "$rust_gate_dir/out" 2>"$rust_gate_dir/${name}.clippy"; then
    echo "  $name → clippy принял"
  else
    echo "  $name → clippy ОТВЕРГ:"
    sed 's/^/    /' "$rust_gate_dir/${name}.clippy" | head -12
    rust_failed=1
  fi
done
# A12/R10: unsafe в порождаемом коде отсутствует. Проверка дублирует
# `#![forbid(unsafe_code)]` в шапке модуля НАМЕРЕННО: атрибут ловит `unsafe` в
# коде, а grep — ещё и попытку убрать сам атрибут.
if grep -rn "unsafe" "$RUST_OUTPUT"/*.rs 2>/dev/null | grep -v "forbid(unsafe_code)" ; then
  echo "  В порождённом Rust найден unsafe — предкоммит провален (A12, фича 0050)."
  rust_failed=1
fi
rm -rf "$rust_gate_dir"
if [ "$rust_failed" -ne 0 ]; then
  echo "  Порождённый Rust не проходит гейт — предкоммит провален (фича 0050)."
  exit 1
fi

# ГЕЙТ ИСПОЛНЕНИЯ ЦЕЛИ RUST: порождённый автомат обязан не только компилироваться,
# но и РАБОТАТЬ. Аналог пары «cmake + ninja + запуск stacker» цели `c`.
#
# Гейт выше доказывает, что вывод принимают rustc и clippy, — то есть что он
# СИНТАКСИЧЕСКИ и типово верен. Молча неверную трансляцию (перепутанный
# приоритет, потерянный переход, не тот операнд) он не ловит: такой код
# компилируется. Поэтому каждый `main` в `examples/generated/rust/src/bin/`
# прогоняет свою модель на подставном железе и проверяет наблюдаемое поведение
# через `assert!` — падение `assert!` валит предкоммит.
#
# Крейт НАМЕРЕННО вне workspace репозитория (своя таблица `[workspace]`), потому
# и вызывается отдельным `--manifest-path`: его содержимое порождается `lamc`, и
# под `cargo check`/`clippy` корня попадать не должно. По той же причине здесь
# `build`, а не `clippy`: на `comprehensive.rs` clippy закономерно ругается
# (известный дефект примера, фича 0030 — см. пропуск в гейте выше), а `rustc`
# этот пример принимает.
echo "Гейт исполнения цели rust: сборка Cargo и прогон проверок по моделям..."
RUST_MANIFEST="$RUST_OUTPUT/Cargo.toml"
$CARGO_CMD build --quiet --manifest-path "$RUST_MANIFEST" --bins
for bin_src in "$RUST_OUTPUT"/src/bin/*.rs; do
  [ -e "$bin_src" ] || continue
  bin="$(basename "$bin_src" .rs)"
  $CARGO_CMD run --quiet --manifest-path "$RUST_MANIFEST" --bin "$bin" | sed 's/^/  /'
done

# ГЕЙТ ЦЕЛИ SV (фича 0045, задача 0045-02): порождённый .sv обязан приниматься
# Verilator (линт) И yosys (синтез). ДВА инструмента — не осторожность, а вывод
# из проб 2026-07-15: они ловят НЕПЕРЕСЕКАЮЩИЕСЯ классы дефектов, причём ровно
# те два, что критичны для решений ADR.
#
#   | проба                | verilator --lint-only -Wall | yosys synth          |
#   |----------------------|-----------------------------|----------------------|
#   | целевой модуль       | чисто, код 0                | 22 ячейки, код 0     |
#   | `real` в always_ff   | ПРИНЯЛ МОЛЧА, код 0         | ERROR TOK_REAL, код 1|
#   | комбинационная петля | UNOPTFLAT, код 1            | СИНТЕЗИРОВАЛ, код 0  |
#
# То есть расхожее «--lint-only даёт проверку синтеза» НЕВЕРНО: Verilator — линтер
# и симулятор, для него `real` легален, и с одним лишь Verilator решение
# «Rational → SV-003» осталось бы недоказанным. Обратно: центральный риск
# отображения `M1 | M2` — комбинационная петля — синтезатором НЕ ловится вовсе.
# Убрать любой из двух = ослепить гейт на целый класс дефектов.
#
# Мягкая деградация (образец — `cc_available()` в conformance_c_tests.rs):
# инструмента нет → шаг пропускается с явным сообщением. Verilator и yosys для
# сборки и тестов `lamc` не нужны (ставятся `brew install verilator yosys`), и
# машина разработчика вправе быть без них.
#
# В CI мягкость недопустима: пропущенный гейт зелёный, то есть неотличим от
# пройденного, — и фича осталась бы без арбитра, сама себя объявив проверенной.
# Поэтому CI выставляет SV_GATE_REQUIRED=1, и тогда отсутствие инструмента —
# ОШИБКА, а не пропуск.
#
# `lint_off` в порождённом коде НЕ применяется: если вывод требует глушения
# предупреждения — это дефект генератора, а не повод глушить.
echo "Гейт цели sv: verilator (линт) + yosys (синтез) по порождённым .sv..."
sv_failed=0
# SV_GATE_REQUIRED уже установлен вверху скрипта (наследует PRECHECK_STRICT,
# фича 0090); здесь лишь используется.

# Отсутствие инструмента: пропуск локально, ошибка в CI (SV_GATE_REQUIRED=1).
sv_tool_missing() {
  if [ "$SV_GATE_REQUIRED" = "1" ]; then
    echo "  $1 не найден, а SV_GATE_REQUIRED=1 — гейт sv обязателен. Провал."
    sv_failed=1
  else
    echo "  [пропуск] $1 не найден — гейт пропущен (brew install $1)"
  fi
}

# Сперва — обязательный список. Пример, переставший транслироваться, обязан
# ронять предкоммит ГРОМКО, а не тихо исчезать из прогона гейта ниже.
for name in $SV_TRANSLATABLE; do
  if [ ! -e "$SV_OUTPUT/${name}.sv" ]; then
    echo "  $name → НЕ ОТТРАНСЛИРОВАН, хотя обязан (цель sv, фича 0045)."
    echo "    Причина — выше, в строке '[предупреждение] цель sv: examples/${name}.lam'."
    sv_failed=1
  fi
done

# И зеркально — НИ ОДНОГО лишнего .sv (доделка 0045-02). Пример, переставший
# транслироваться (SV-002/SV-005 после 0045-06), оставляет в каталоге стаб от
# каркаса: `-o` пишет только при УСПЕХЕ, отказ старый файл не трогает. Стаб
# валиден — verilator/yosys его молча примут, — то есть каталог хранит вывод,
# которого компилятор уже НЕ производит. Гейт «определяет правду» (0045-02),
# значит хранить такую ложь не вправе. Симметрично ловится и обратный дрейф:
# пример, внезапно заработавший, но не внесённый в SV_TRANSLATABLE.
for sv_file in "$SV_OUTPUT"/*.sv; do
  [ -e "$sv_file" ] || continue
  name="$(basename "$sv_file" .sv)"
  case " $SV_TRANSLATABLE " in
    *" $name "*) ;;  # ожидаемый — в списке обязательных
    *)
      echo "  $name.sv → ЛИШНИЙ .sv: примера нет в SV_TRANSLATABLE, но файл есть."
      echo "    Либо это устаревший стаб (git rm $SV_OUTPUT/${name}.sv),"
      echo "    либо пример начал транслироваться — тогда внести его в SV_TRANSLATABLE."
      sv_failed=1
      ;;
  esac
done

if command -v verilator &>/dev/null; then
  for sv_file in "$SV_OUTPUT"/*.sv; do
    [ -e "$sv_file" ] || continue
    name="$(basename "$sv_file" .sv)"
    sv_err="$(mktemp)"
    if verilator --lint-only -Wall "$sv_file" >/dev/null 2>"$sv_err"; then
      echo "  $name → verilator принял"
    else
      echo "  $name → verilator ОТВЕРГ:"
      sed 's/^/    /' "$sv_err" | head -12
      sv_failed=1
    fi
    rm -f "$sv_err"
  done
else
  sv_tool_missing verilator
fi

if command -v yosys &>/dev/null; then
  for sv_file in "$SV_OUTPUT"/*.sv; do
    [ -e "$sv_file" ] || continue
    name="$(basename "$sv_file" .sv)"
    sv_err="$(mktemp)"
    # -top обязателен: имя модуля выводится из имени корневой модели и совпадает
    # с именем файла. Без него yosys выбирает верхний модуль сам и на ошибке
    # иерархии промолчит.
    if yosys -q -p "read_verilog -sv $sv_file; synth -top $name" >/dev/null 2>"$sv_err"; then
      echo "  $name → yosys синтезировал"
    else
      echo "  $name → yosys НЕ СИНТЕЗИРОВАЛ:"
      sed 's/^/    /' "$sv_err" | head -12
      sv_failed=1
    fi
    rm -f "$sv_err"
  done
else
  sv_tool_missing yosys
fi

# ГЕЙТ ЦЕЛИ SV-MMIO (фича 0062): регистровый файл из адресов портов обязан
# приниматься verilator И yosys, как и цель `sv` (те же два инструмента ловят
# непересекающиеся классы). Наблюдение поведения — потактовая сверка ЧЕРЕЗ
# РЕГИСТРЫ (simulation/tests/conformance_sv_mmio_tests.rs), а не этот гейт: линт и
# синтез верности не доказывают (урок 0045). Тот же $sv_failed и мягкая
# деградация, что у гейта `sv`.
echo "Гейт цели sv-mmio: verilator (линт) + yosys (синтез) по регистровым файлам..."
# Обязательные примеры на месте?
for name in $SV_MMIO_TRANSLATABLE; do
  if [ ! -e "$SV_MMIO_OUTPUT/${name}.sv" ]; then
    echo "  $name → НЕ ОТТРАНСЛИРОВАН целью sv-mmio, хотя обязан (фича 0062)."
    sv_failed=1
  fi
done
# Ни одного лишнего .sv (тот же принцип, что у гейта sv).
for sv_file in "$SV_MMIO_OUTPUT"/*.sv; do
  [ -e "$sv_file" ] || continue
  name="$(basename "$sv_file" .sv)"
  case " $SV_MMIO_TRANSLATABLE " in
    *" $name "*) ;;
    *)
      echo "  $name.sv → ЛИШНИЙ .sv sv-mmio: примера нет в SV_MMIO_TRANSLATABLE."
      sv_failed=1
      ;;
  esac
done
if command -v verilator &>/dev/null; then
  for sv_file in "$SV_MMIO_OUTPUT"/*.sv; do
    [ -e "$sv_file" ] || continue
    name="$(basename "$sv_file" .sv)"
    sv_err="$(mktemp)"
    if verilator --lint-only -Wall "$sv_file" >/dev/null 2>"$sv_err"; then
      echo "  $name → verilator принял (sv-mmio)"
    else
      echo "  $name → verilator ОТВЕРГ (sv-mmio):"
      sed 's/^/    /' "$sv_err" | head -12
      sv_failed=1
    fi
    rm -f "$sv_err"
  done
else
  sv_tool_missing verilator
fi
if command -v yosys &>/dev/null; then
  for sv_file in "$SV_MMIO_OUTPUT"/*.sv; do
    [ -e "$sv_file" ] || continue
    name="$(basename "$sv_file" .sv)"
    sv_err="$(mktemp)"
    if yosys -q -p "read_verilog -sv $sv_file; synth -top $name" >/dev/null 2>"$sv_err"; then
      echo "  $name → yosys синтезировал (sv-mmio)"
    else
      echo "  $name → yosys НЕ СИНТЕЗИРОВАЛ (sv-mmio):"
      sed 's/^/    /' "$sv_err" | head -12
      sv_failed=1
    fi
    rm -f "$sv_err"
  done
else
  sv_tool_missing yosys
fi

# ГЕЙТ ТЕСТБЕНЧЕЙ ЦЕЛИ SV: линт и синтез доказывают, что RTL валиден и
# синтезируем, но НЕ что он ведёт себя как модель (см. предупреждение README —
# молча неверная трансляция синтезируется тоже). Тестбенчи в
# examples/generated/sv/tb/ прогоняют модуль замкнутой средой на осмысленном
# сценарии, проверяют наблюдаемое поведение assert-ами (провал → $fatal →
# ненулевой код выхода) и снимают осциллограмму <name>.vcd для gtkwave.
#
# Тестбенчи написаны РУКАМИ (не порождаются lamc): тестбенч — принадлежность
# проверки, а не продукта (решение 0045-07). Список модулей — тот же
# $SV_TRANSLATABLE: у каждого обязан быть парный tb/<name>_tb.sv.
#
# Verilator собирает симулятор (--binary --timing) с трассировкой (--trace).
# Отдельного мягкого/жёсткого разбора отсутствия verilator здесь нет: гейт линта
# выше уже отметил его нехватку (пропуск локально, ошибка в CI). Если verilator
# есть — тестбенчи ОБЯЗАНЫ собраться и пройти.
SV_TB_DIR="$SV_OUTPUT/tb"
if command -v verilator &>/dev/null; then
  echo "Гейт тестбенчей sv: verilator (--binary) прогоняет tb/<name>_tb.sv..."
  for name in $SV_TRANSLATABLE; do
    tb_src="$SV_TB_DIR/${name}_tb.sv"
    if [ ! -e "$tb_src" ]; then
      echo "  $name → тестбенч $tb_src отсутствует (ожидался парный tb для \$SV_TRANSLATABLE)."
      sv_failed=1
      continue
    fi
    tb_obj="$(mktemp -d)"
    tb_log="$(mktemp)"
    if verilator --binary --timing --trace -Wno-fatal --top-module tb \
        -Mdir "$tb_obj" -o simtb \
        "$tb_src" "$SV_OUTPUT/${name}.sv" >"$tb_log" 2>&1; then
      # Прогон в каталоге tb/: <name>.vcd ложится рядом с исходником тестбенча
      # (путь в $dumpfile относительный). Код выхода ≠ 0 (провал assert/$fatal)
      # валит гейт.
      if ( cd "$SV_TB_DIR" && "$tb_obj/simtb" ) >"$tb_log" 2>&1; then
        sed 's/^/    /' "$tb_log" | grep -E 'OK|TICK' | head -4 || true
        echo "  $name → тестбенч ПРОШЁЛ; осциллограмма $SV_TB_DIR/${name}.vcd"
      else
        echo "  $name → тестбенч ПРОВАЛИЛСЯ:"
        sed 's/^/    /' "$tb_log" | head -12
        sv_failed=1
      fi
    else
      echo "  $name → verilator НЕ СОБРАЛ тестбенч:"
      sed 's/^/    /' "$tb_log" | head -12
      sv_failed=1
    fi
    rm -rf "$tb_obj" "$tb_log"
  done
fi

if [ "$sv_failed" -ne 0 ]; then
  echo "  Порождённый SystemVerilog не проходит гейт — предкоммит провален (фича 0045)."
  exit 1
fi

cmake -DCMAKE_BUILD_TYPE=Debug -G Ninja -S $C_OUTPUT -B $C_OUTPUT/cmake-build-debug/
cd $C_OUTPUT/cmake-build-debug/ && ninja
cd -

# Гейт компиляции цели c-hal (фикс 0020-01 / фича 0098). Прежде c-hal НИГДЕ не
# компилировалась — только диффилась на детерминизм (её вывод сверяли сам с
# собой). Ровно эта дыра покрытия дала дожить UB в дефолтном HAL (сдвиг бита
# ≥ ширины типа). Теперь каждый пример генерируется в c-hal и компилируется в
# объект (`cc -c`); линковка не нужна — extern fn и main здесь ни при чём.
# Примеры без адресов портов (elevator_mini → SE-052) в c-hal не транслируются —
# это не отказ гейта, а отсутствие адресов; такие пропускаются.
if require_tool cc "гейт c-hal, фикс 0020-01; обычно есть на всех платформах"; then
  echo "Гейт c-hal: компиляция порождённого дефолтного HAL (фикс 0020-01)..."
  chal_failed=0
  for lam_file in examples/*.lam; do
    name="$(basename "$lam_file" .lam)"
    chal_dir="$(mktemp -d)"
    if ! "$LAMC" compile "$lam_file" -t c-hal -o "$chal_dir" >"$chal_dir/gen.log" 2>&1; then
      echo "  $name [c-hal] → пропуск (не транслируется: $(head -1 "$chal_dir/gen.log" | cut -c1-40))"
      rm -rf "$chal_dir"
      continue
    fi
    ok=1
    for c in "$chal_dir"/*.c; do
      [ -e "$c" ] || continue
      if ! cc -std=c11 -I "$chal_dir" -c "$c" -o /dev/null 2>"$chal_dir/cc.log"; then
        echo "  $name [c-hal] → НЕ КОМПИЛИРУЕТСЯ:"
        sed 's/^/    /' "$chal_dir/cc.log" | head -8
        ok=0
        chal_failed=1
      fi
    done
    [ "$ok" -eq 1 ] && echo "  $name [c-hal] → компилируется"
    rm -rf "$chal_dir"
  done
  if [ "$chal_failed" -ne 0 ]; then
    echo "  Порождённый c-hal не компилируется — предкоммит провален (фикс 0020-01)."
    exit 1
  fi
fi

BUILD_DIR="$ROOT/$C_OUTPUT/cmake-build-debug"
if [ -x "$BUILD_DIR/stacker" ]; then
  echo "Запуск симуляции stacker..."
  "$BUILD_DIR/stacker" > /tmp/stacker_sim.log
  echo "  лог: /tmp/stacker_sim.log ($(wc -l < /tmp/stacker_sim.log) строк)"
else
  echo "  [пропуск] stacker не собран — симуляция пропущена"
fi

# Арбитр валидности порождённого Structured Text (фича 0041). Здесь только
# ОБЕСПЕЧЕНИЕ инструмента: при отсутствии он собирается из исходников и ставится
# в ~/.local/bin/iec2c. Сам скрипт ensure-iec2c.sh предкоммит не валит — `iec2c`
# внешний; строгость обеспечивает ГЕЙТ ниже под PRECHECK_STRICT=1 (фича 0090).
#
# ГЕЙТ включён и ЗЕЛЁН: бэкенд ST дозрел (тела FUNCTION_BLOCK эмитятся), и iec2c
# принимает весь корпус examples/ (проверено фичей 0090 — 8/8 валидны). Прежняя
# заметка «гейт пока не включён» устарела.
echo "Проверка ST-арбитра (MatIEC iec2c)..."
"$ROOT/scripts/ensure-iec2c.sh" || true

# ГЕЙТ ST: порождённый Structured Text обязан компилироваться (фича 0041).
# Единственный автоматизируемый способ доказать, что вывод валиден по стандарту:
# юнит-тесты проверяют лишь, что генератор напечатал задуманное.
#
# Гейт НЕ валит предкоммит, если инструмента нет: `iec2c` внешний и для сборки
# `lamc` не нужен. Но если он есть — невалидный ST это ОШИБКА.
IEC2C_BIN="${IEC2C_PREFIX:-$HOME/.local}/bin/iec2c"
IEC2C_LIB="${IEC2C_PREFIX:-$HOME/.local}/share/matiec/lib"
if [ -x "$IEC2C_BIN" ] && [ -f "$IEC2C_LIB/ieclib.txt" ]; then
  echo "Гейт ST: проверка порождённого кода транспилятором iec2c..."
  st_failed=0
  for st_file in "$ST_OUTPUT"/*.st; do
    [ -e "$st_file" ] || continue
    name="$(basename "$st_file" .st)"
    out_dir="$(mktemp -d)"
    if "$IEC2C_BIN" -I "$IEC2C_LIB" -T "$out_dir" "$st_file" >/dev/null 2>"$out_dir/err"; then
      echo "  $name → валиден"
    else
      echo "  $name → НЕВАЛИДЕН:"
      sed 's/^/    /' "$out_dir/err" | head -5
      st_failed=1
    fi
    rm -rf "$out_dir"
  done
  if [ "$st_failed" -ne 0 ]; then
    echo "  Порождённый ST не принимается iec2c — предкоммит провален."
    exit 1
  fi
elif [ "$PRECHECK_STRICT" = "1" ]; then
  # iec2c — проверка файла, а не команды на PATH (собирается в ~/.local), поэтому
  # require_tool здесь не применить; строгость дублируется явной веткой.
  echo "  ОШИБКА: iec2c недоступен ($IEC2C_BIN), а PRECHECK_STRICT=1 — ST-гейт обязателен." >&2
  exit 1
else
  echo "  [пропуск] iec2c недоступен — гейт ST пропущен"
fi
