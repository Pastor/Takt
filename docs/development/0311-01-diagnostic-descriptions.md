# Разработка 0311-01: описания записей реестра и гейт

> Фича: [../features/0311-diagnostic-descriptions.md](../features/0311-diagnostic-descriptions.md) · ADR: [../adr/0311-diagnostic-descriptions.md](../adr/0311-diagnostic-descriptions.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `docs/diagnostics/README.md` | десять записей получили описания (`DF-001`…`DF-004`, `CS-001`, `FM-001`, `AM-001`, `ST-004`, `SV-005`, `SV-006`) |
| `scripts/check-diagnostic-descriptions.py` | гейт: классы D1 (служебное слово), D2 (короткое вне долга), D3 (протухший долг); самопроверка |
| `scripts/diagnostic-description-baseline.txt` | узаконенный долг — 15 кратких, но осмысленных записей |
| `scripts/test-diagnostic-descriptions.sh` | сторож гейта: 7 условий на копии дерева (`DD_ROOT`) |
| `scripts/precheck.sh` | шаг рядом с гейтом 0290 |

## Откуда взяты описания

Каждое снято **с кода эмиссии**, а не придумано: `DF-001` — из двух ветвей
`address_map/env.rs` (нет `=`; недопустимое имя символа), `DF-003` — из
комментария о симметрии с `AM-006`, `SV-006` — из текста отказа про третье
состояние.

## Проверено

- `sh scripts/test-diagnostic-descriptions.sh` — 7/7.
- `python3 scripts/check-diagnostic-descriptions.py` — 229 записей, долга 15.
- `python3 scripts/check-book-diagnostics.py` — 265 кодов сверены (гейт 0290 не
  задет).
