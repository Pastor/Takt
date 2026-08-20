# Разработка 0315-01: сторожа четырёх гейтов

> Фича: [../features/0315-gate-guards.md](../features/0315-gate-guards.md) · ADR: [../adr/0315-gate-guards.md](../adr/0315-gate-guards.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `scripts/check-links.py` | самопроверка `--self-test` (битая ссылка ловится; целая, внешняя, в код-спане и в блоке — нет) |
| `scripts/check-exhaustive-nodes.sh` | корень переопределяется `EN_ROOT` |
| `scripts/check-language-version.sh` | корень переопределяется `LV_ROOT` |
| `scripts/check-repo-url.sh` | корень переопределяется `RU_ROOT` |
| `scripts/test-exhaustive-nodes.sh` | 4 условия |
| `scripts/test-language-version.sh` | 5 условий |
| `scripts/test-repo-url.sh` | 5 условий |
| `scripts/precheck.sh` | сторожа и самопроверка идут перед своими гейтами |

## Проверено

- Каждый сторож прогнан отдельно: все условия проходят.
- `./scripts/precheck.sh` — код 0; в логе видны все четыре прикрытия.
