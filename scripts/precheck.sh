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

if command -v python3 &>/dev/null; then
  echo "Проверка ссылок в Markdown (правило 14)..."
  "$(dirname "$0")/check-links.py"
else
  echo "  [пропуск] python3 не найден — проверка ссылок пропущена"
fi

$CARGO_CMD +nightly fmt
$CARGO_CMD check
$CARGO_CMD clippy --all-targets --all-features
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
#   extend_complex.lam  SV-002  досрочный возврат из функции `is_collected`:
#                               возврат печатается присваиванием имени функции и
#                               исполнение не прерывает
SV_TRANSLATABLE="stacker elevator_mini"

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
  $LAMC compile "$lam_file" -t sv -o "$SV_OUTPUT" \
    || echo "    [предупреждение] цель sv: $lam_file не транслируется"
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
  for spec in "c:" "c-hal:-t c-hal" "plantuml:-t plantuml" "st:-t st" "st-at:-t st-at" "rust:-t rust" "sv:-t sv"; do
    tgt="${spec%%:*}"
    flag="${spec#*:}"
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
SV_GATE_REQUIRED="${SV_GATE_REQUIRED:-0}"

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

if [ "$sv_failed" -ne 0 ]; then
  echo "  Порождённый SystemVerilog не проходит гейт — предкоммит провален (фича 0045)."
  exit 1
fi

cmake -DCMAKE_BUILD_TYPE=Debug -G Ninja -S $C_OUTPUT -B $C_OUTPUT/cmake-build-debug/
cd $C_OUTPUT/cmake-build-debug/ && ninja
cd -

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
# в ~/.local/bin/iec2c. Скрипт никогда не валит предкоммит — `iec2c` внешний и
# для сборки/тестов `lamc` не нужен.
#
# Сам ГЕЙТ (прогон iec2c по порождённым .st) пока не включён: у порождённых
# `FUNCTION_BLOCK` нет тела — это задача 0041-03, — и iec2c их закономерно
# отвергает. Включение гейта — задача 0041-05 (см. docs/development/0041-06).
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
else
  echo "  [пропуск] iec2c недоступен — гейт ST пропущен"
fi
