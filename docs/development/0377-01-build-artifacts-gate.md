# Задача 0377-01: Артефакты сборки не попадают в репозиторий

> Фича: [../features/0377-build-artifacts-gate.md](../features/0377-build-artifacts-gate.md) · ADR: [../adr/0377-build-artifacts-gate.md](../adr/0377-build-artifacts-gate.md) · анализ: [../analyze/0377-build-artifacts-gate.md](../analyze/0377-build-artifacts-gate.md)

## Что было

Три файла `*.rlib` в корне, отслеживаемые гитом с коммита `3c6bec19`.
`.gitignore` их не закрывал, гейта на состав репозитория не было.

## Что сделано

- `git rm` трёх файлов.
- `.gitignore`: закрыты `/*.rlib`, `/*.rmeta`, `/*.d`, `/*.so`, `/*.dylib` —
  рядом с уже существовавшим `/*.o`, с объяснением повода.
- `scripts/check-build-artifacts.py`: гейт по **отслеживаемым** файлам
  (`git ls-files`), падает списком; список расширений — в заголовке гейта,
  исключений нет.
- `scripts/test-build-artifacts.sh`: сторож, строящий **временный
  репозиторий** (копии дерева мало — нужен свой индекс); четыре условия.
- Оба подключены в `precheck.sh` рядом с гейтом устаревших имён.

**Статус по функциональности (правило 11).** Вывод инструментов не затронут —
фича процессная; целям и эталону **н/п**.

## Проверки

```sh
python3 scripts/check-build-artifacts.py   # 3703 файла, артефактов нет
./scripts/test-build-artifacts.sh          # 4 условия
./scripts/precheck.sh
```
