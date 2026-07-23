# Задача 0094-01: Доработка scripts/new-feature.sh (идемпотентность, статус ADR, поздние стадии)

> Фича: [../features/0094-new-feature-script-fixes.md](../features/0094-new-feature-script-fixes.md) · ADR: [../adr/0094-new-feature-script-fixes.md](../adr/0094-new-feature-script-fixes.md) · анализ: [../analyze/0094-new-feature-script-fixes.md](../analyze/0094-new-feature-script-fixes.md)

## Что было

Генератор `scripts/new-feature.sh` (0015): `insert_row` безусловно дописывал строку
→ повторный `--register` **дублировал** (правилось вручную ×3 за сессию); ADR-строка
реестра и шаблон — `Accepted` до написания ADR; нет добора одной стадии, `report`
недоступен вовсе; одна dev-подзадача.

## Что сделано

Реализована **Option A** [ADR 0094](../adr/0094-new-feature-script-fixes.md).

- **Идемпотентный `insert_row <README> <key> <row>`:** если строка с ключом-номером
  (`| [NNNN]` / `| NNNN |` / `| NNNN-YY |`) в реестре уже есть — вставка
  пропускается. Убирает дубли конструктивно.
- **Статус ADR — `Draft`:** ADR-строка реестра и шаблон `docs/templates/adr.md`
  (`Status: Accepted → Draft`). `Accepted` проставляется по факту (стадия 2).
- **`--stage NAME`** (`feature|adr|analyze|dev|tests|report`) — рендер + идемпотентная
  регистрация **одной** стадии; `report` поддержан впервые. **`--subtask NN`** —
  номер dev-подзадачи (`XXXX-NN`). Общий помощник `do_stage`, дефолт переиспользует
  его.
- **`NF_ROOT`** — переопределение корня (для тестируемости в temp-дереве).
- ⚠️ **Регистрация — `if … then … fi`, а не `[ … ] && insert_row`:** под `set -e`
  последняя форма при `REGISTER=0` возвращает 1 и обрывает `do_stage` (ломало
  дефолтный путь без `--register`). **Поймано регресс-тестом** при разработке.

| Функциональность | Статус |
|---|---|
| идемпотентность `--register` | ✅ по ключу-номеру |
| статус ADR `Draft` | ✅ строка + шаблон |
| `--stage`/`--subtask` (+`report`) | ✅ |
| `NF_ROOT` + регресс-тест | ✅ `scripts/test-new-feature.sh` в precheck |
| крейты / язык | н/п — инфраструктура |

## Проверки

- `scripts/test-new-feature.sh` (A1–A5): 14 проверок зелёные — идемпотентность
  (одна строка после двух прогонов), `Draft`/не `Accepted`, `--stage report`
  создаёт+регистрирует, `--subtask 03` идемпотентно, дефолтный путь.
- **Dogfooding:** стадии `dev`/`tests`/`report` фичи 0094 добраны **исправленным**
  скриптом на реальных реестрах — дублей нет (все счётчики 0094 = 1).
- Подключён в `precheck.sh` (быстрый блок, до тестов).
- Полный `./scripts/precheck.sh` → зелёный (см.
  [отчёт](../reports/0094-new-feature-script-fixes.md)).
