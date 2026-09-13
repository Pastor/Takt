#!/usr/bin/env bash
# Контроль проверки литералов сообщений `check-message-literals.py`.
#
# Мутациями доказывается, что проверка ловит то, ради чего заведена:
#
# L0 - согласованное дерево принимается: комментарии, `#[cfg(test)]`, `assert!`,
#      `expect` и символьный литерал с кириллицей проверке не мешают;
# L1 - сообщение литералом в коде;
# L1m - многострочный литерал с продолжением `\`: кириллица только во второй
#      строке, и однострочный поиск такой литерал не видит;
# L1r - сырой литерал `r#"..."#`;
# L1c - слово `panic!` в комментарии рядом с сообщением его не прячет;
# L2 - в разрешённом файле литералов стало больше реестра;
# L2d - литералов стало меньше: запись обязана быть опущена;
# L3 - запись реестра без файла;
# L4 - вырожденный вход (исходников нет) даёт отказ, а не молчание.
#
# Мутации ставятся на копии-макете дерева (`ML_ROOT`), рабочие каталоги не трогаются.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GATE="$ROOT/scripts/check-message-literals.py"
FAILED=0

# Макет: полсотни файлов (проверка требует нижней границы выборки) и один
# разрешённый файл с порождённым текстом.
prepare() {
  local dst="$1"
  mkdir -p "$dst/scripts" "$dst/takt-lang/src/generator" "$dst/takt-sim/src"
  cp "$ROOT/scripts/gatelib.py" "$dst/scripts/"
  for i in $(seq 1 50); do
    printf 'pub fn f%s() -> u8 { %s }\n' "$i" "$i" > "$dst/takt-lang/src/m$i.rs"
  done
  cat > "$dst/takt-lang/src/clean.rs" <<'RS'
//! Модуль без сообщений: кириллица только там, где ей место.

/// Комментарий по-русски законен.
pub fn clean(x: u8) -> char {
    /* блочный /* вложенный */ комментарий */
    assert!(x < 10, "нарушен инвариант кода");
    let _ = Some(x).expect("значение есть по построению");
    'я'
}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        assert_eq!("проба", "проба");
    }
}
RS
  cat > "$dst/takt-lang/src/generator/emit.rs" <<'RS'
pub fn emit() -> &'static str {
    "(* первый скан *)"
}
RS
  echo "takt-lang/src/generator/emit.rs 1 # текст порождённого кода" > "$dst/scripts/message-literals.txt"
}

expect() {
  local name="$1" want="$2" dst="$3"
  ML_ROOT="$dst" python3 "$GATE" >/dev/null 2>&1
  local got=$?
  if [ "$want" = "fail" ] && [ "$got" -eq 0 ]; then
    echo "  ✗ $name: проверка ПРОПУСТИЛА мутацию"
    FAILED=1
  elif [ "$want" = "pass" ] && [ "$got" -ne 0 ]; then
    echo "  ✗ $name: проверка отвергла корректное дерево"
    FAILED=1
  else
    echo "  ✓ $name"
  fi
  rm -rf "$dst"
}

echo "Контроль проверки литералов сообщений (0532)..."

# L0 идёт первым: краснеющая на корректном дереве проверка обесценивает прочие пробы.
D=$(mktemp -d); prepare "$D"
expect "L0 согласованное дерево принимается" pass "$D"

D=$(mktemp -d); prepare "$D"
printf 'pub fn m() -> String { format!("порт {} не найден", 1) }\n' > "$D/takt-lang/src/bad.rs"
expect "L1 сообщение литералом в коде" fail "$D"

D=$(mktemp -d); prepare "$D"
cat > "$D/takt-lang/src/multi.rs" <<'RS'
pub fn m(name: &str) -> String {
    format!(
        "name '{name}': value \
         не вычисляется при компиляции"
    )
}
RS
expect "L1m многострочный литерал с продолжением" fail "$D"

D=$(mktemp -d); prepare "$D"
printf 'pub fn m() -> &%sstatic str { r#"сырой текст"# }\n' "'" > "$D/takt-sim/src/raw.rs"
expect "L1r сырой литерал" fail "$D"

D=$(mktemp -d); prepare "$D"
cat > "$D/takt-lang/src/hidden.rs" <<'RS'
pub fn m() -> String {
    // здесь не panic!, а обычное сообщение
    "текст мимо каталога".to_string()
}
RS
expect "L1c слово panic! в комментарии не прячет сообщение" fail "$D"

D=$(mktemp -d); prepare "$D"
cat >> "$D/takt-lang/src/generator/emit.rs" <<'RS'
pub fn more() -> &'static str {
    "ещё текст"
}
RS
expect "L2 рост счёта в разрешённом файле" fail "$D"

D=$(mktemp -d); prepare "$D"
printf 'pub fn emit() -> u8 { 0 }\n' > "$D/takt-lang/src/generator/emit.rs"
expect "L2d убыль счёта без правки реестра" fail "$D"

D=$(mktemp -d); prepare "$D"
echo "takt-lang/src/gone.rs 1 # файла нет" >> "$D/scripts/message-literals.txt"
expect "L3 запись реестра без файла" fail "$D"

D=$(mktemp -d); prepare "$D"
rm -rf "$D/takt-lang/src" "$D/takt-sim/src"
expect "L4 пустая выборка — отказ" fail "$D"

if [ "$FAILED" -ne 0 ]; then
  echo "Контроль проверки литералов сообщений: ПРОВАЛ"
  exit 1
fi
echo "Контроль проверки литералов сообщений: все пробы пройдены"
