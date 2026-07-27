# Задача 0139-01: Удаление мёртвой конфигурации `.travis.yml`

> Фича: [../features/0139-remove-travis-config.md](../features/0139-remove-travis-config.md) · ADR: [../adr/0139-remove-travis-config.md](../adr/0139-remove-travis-config.md) · анализ: [../analyze/0139-remove-travis-config.md](../analyze/0139-remove-travis-config.md)

## Что было

В корне репозитория — `.travis.yml` (5 строк): `cargo clippy` + `cargo test
--verbose`. Файл отслеживался git, но не исполнялся: Travis CI к репозиторию не
подключён (прогонов нет, бейджа нет). Действующий CI — `.github/workflows/ci.yml`
→ единственный вызов `./scripts/precheck.sh` под `PRECHECK_STRICT=1` (фича 0090).

Итого в репозитории существовало **два** описания того, «что нужно проверить
перед коммитом», и более слабое из них было ложным.

## Что сделано

- `git rm .travis.yml` — файл удалён из дерева и из индекса.
- Проверено грепом по всему дереву, что живых упоминаний Travis не осталось:
  найденные совпадения принадлежат **артефактам самой фичи 0139** и строкам
  реестров (`docs/*/README.md`), то есть истории решения, а не контракту
  проверок.
- `scripts/precheck.sh` и `.github/workflows/ci.yml` **не трогались** (требование
  R2 анализа): набор гейтов остался ровно прежним.

## Проверки

| Проверка | Команда | Результат |
|---|---|---|
| A1 файла нет | `test ! -e .travis.yml` | ✅ |
| A2 нет в индексе git | `git ls-files .travis.yml` | ✅ пусто |
| A3 нет живых упоминаний | `grep -rni travis README.md docs/ CLAUDE.md scripts/ .github/` | ✅ только артефакты фичи и реестры |
| A4 гейты не ослаблены | `git diff --name-only HEAD -- scripts/precheck.sh .github/workflows/ci.yml` | ✅ пусто |
| A5 предкоммит | `./scripts/precheck.sh` | ✅ `EXIT=0` |
| A6 ссылки (правило 14) | `python3 scripts/check-links.py` | ✅ битых нет |
