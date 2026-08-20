# Задача 0289-01: Метки инвариантов и класс 5 гейта

> Фича: [../features/0289-claude-md-invariant-checklist-gate.md](../features/0289-claude-md-invariant-checklist-gate.md) · ADR: [../adr/0289-claude-md-invariant-checklist-gate.md](../adr/0289-claude-md-invariant-checklist-gate.md) · анализ: [../analyze/0289-claude-md-invariant-checklist-gate.md](../analyze/0289-claude-md-invariant-checklist-gate.md)

## Что сделано

**`CLAUDE.md`:** шесть подробных пунктов получили метку
`` `[критический инвариант N]` ``; во врезке чек-листа сказано, что связь
двусторонняя и сторожится машиной.

**`scripts/check-claude-md.py`** — **класс 5**:

| Находка | Когда |
|---|---|
| «инвариант N есть в чек-листе, но метки нет» | забыт подробный пункт |
| «пункт помечен N, но строки в чек-листе нет» | забыта строка указателя |
| «метка N стоит K раз» | дубль номера |
| «раздел не найден или пуст» | проверка вырождена |

**Самопроверка** дополнена двумя случаями класса 5. ⚠️ Заодно пришлось
дополнить «законный» образец согласованной парой: без чек-листа он давал
находку вырожденности — то есть контроль сам ломался. Поймано первым прогоном.

## Проверка

```sh
python3 scripts/check-claude-md.py --self-test   # 5 классов
python3 scripts/check-claude-md.py
```
