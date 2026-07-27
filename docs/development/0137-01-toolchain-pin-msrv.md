# Задача 0137-01: Фиксация толчейна Rust и MSRV

> Фича: [../features/0137-toolchain-pin-msrv.md](../features/0137-toolchain-pin-msrv.md) · ADR: [../adr/0137-toolchain-pin-msrv.md](../adr/0137-toolchain-pin-msrv.md) · анализ: [../analyze/0137-toolchain-pin-msrv.md](../analyze/0137-toolchain-pin-msrv.md)

## Что было

- Файла `rust-toolchain.toml` нет; CI берёт `dtolnay/rust-toolchain@nightly`.
- `rust-version` (MSRV) не задан ни в одном манифесте.
- `Cargo.lock` — в `.gitignore`, при том что workspace даёт бинарники, а
  `lalrpop` берётся **из git** без `rev`.
- `scripts/precheck.sh` явно зовёт `cargo +nightly fmt`.
- Корневой `Cargo.toml`: `resolver = "1"` при `edition = "2024"`.

## Что сделано

1. **`rust-toolchain.toml`** — пин `channel = "1.97.1"`, компоненты `rustfmt`,
   `clippy`, `rust-std`, `profile = "minimal"`. В шапке файла — зачем пин, почему
   stable и что подъём версии делается отдельным коммитом.
2. **MSRV** — `rust-version = "1.97"` в `takt-lang/Cargo.toml` и
   `takt-sim/Cargo.toml`, с пометкой, что нижняя граница не исследовалась.
3. **`Cargo.lock` введён в репозиторий** — строка `/Cargo.lock` убрана из
   `.gitignore` (на её месте — пояснение почему), файл добавлен в индекс. Lock
   фиксирует git-зависимость `lalrpop` коммитом `f67f8741…`.
4. **`resolver = "3"`** в корневом манифесте вместо `"1"` (штатный для
   edition 2024).
5. **`scripts/precheck.sh`** — `cargo +nightly fmt` → `cargo fmt` (канал задаёт
   пин); рядом комментарий, что явный `+канал` вернул бы снятую неопределённость.
6. **`.github/workflows/ci.yml`** — шаги `dtolnay/rust-toolchain@nightly` (в обоих
   job) заменены на `rustup show active-toolchain && cargo --version`: rustup сам
   ставит версию и компоненты из файла пина, и версия остаётся описанной в одном
   месте.

## Пробы, на которых стоят решения

| Проба | Результат |
|---|---|
| `cargo +stable clippy --all-targets --all-features -- -D warnings` | **rc 0, ноль предупреждений** — nightly не нужен |
| `cargo +stable fmt --check` на дереве, отформатированном nightly | **rc 0** — стабильный rustfmt (1.9.0) принимает вывод nightly (1.10.0) байт-в-байт |
| `cargo check --locked --all-targets --all-features` | проходит — lock согласован |
| смена `resolver` `"1"` → `"3"` + `cargo check --locked` | проходит, состав зависимостей **не меняется** |
| `grep -rn "#!\[feature(" takt-lang/src takt-sim/src` | пусто — нестабильных возможностей нет |

## Замеченное по ходу (важно для следующего)

⚠️ **Толчейн может установиться неполно и молча.** После первой установки пина
`cargo build` падал с `E0463: can't find crate for std`, хотя
`rustup component list --toolchain 1.97.1` показывал `rust-std … (installed)`:
каталог `lib/rustlib/aarch64-apple-darwin/lib` был **пуст** (0 файлов против 59 у
исправного толчейна). `rustup component add rust-std` отвечал «up to date» и
ничего не чинил. Лечение — **переустановка**:

```sh
rustup toolchain uninstall 1.97.1
rustup toolchain install 1.97.1 --profile minimal -c rustfmt -c clippy
```

Диагностика в одну команду: `ls "$(rustc --print sysroot)/lib/rustlib/<target>/lib" | wc -l`
— у исправного толчейна там десятки файлов.

## Проверки

| Проверка | Результат |
|---|---|
| A1 пин активен | ✅ `1.97.1-aarch64-apple-darwin (overridden by '…/rust-toolchain.toml')` |
| A2 компоненты | ✅ `rustfmt 1.9.0-stable`, `clippy` доступен |
| A3 MSRV | ✅ `rust-version = "1.97"` в обоих манифестах |
| A4 `Cargo.lock` | ✅ в индексе git, из `.gitignore` убран |
| A5 `--locked` | ✅ проходит |
| A6 нет явного канала | ✅ `+nightly`/`@nightly` не осталось |
| A7 предкоммит | ✅ `EXIT=0` |
| A8 резолвер | ✅ граф зависимостей не изменился |
