# Разработка 0316-01: гейт пересказа и правка четырёх шагов

> Фича: [../features/0316-precheck-comment-duplication.md](../features/0316-precheck-comment-duplication.md) · ADR: [../adr/0316-precheck-comment-duplication.md](../adr/0316-precheck-comment-duplication.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `scripts/check-precheck-comments.py` | гейт с самопроверкой: общий дословный кусок ≥60 символов между комментарием шага и заголовком его скрипта |
| `scripts/precheck.sh` | четыре комментария переписаны; добавлен шаг гейта |

## Переписанные шаги

| Шаг | Что пересказывалось |
|---|---|
| `test-language-version.sh` | урок 0255 («гейт, который никогда не падал…») |
| `test-probe.sh` | повод фичи 0251 и обещание сторожа |
| `check-book-generated.sh` | находка трёх отставших снимков |
| `check-readme-commands.sh` | замер команд README |

## Проверено

- `python3 scripts/check-precheck-comments.py --self-test` — ловушка взведена.
- `python3 scripts/check-precheck-comments.py` — 35 шагов, пересказов нет.
- `./scripts/precheck.sh` — код 0.
