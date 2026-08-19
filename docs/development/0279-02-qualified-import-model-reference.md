# Задача 0279-02: Печать примечаний в `takt-sim`

> Фича: [../features/0279-qualified-import-model-reference.md](../features/0279-qualified-import-model-reference.md) · ADR: [../adr/0279-qualified-import-model-reference.md](../adr/0279-qualified-import-model-reference.md) · анализ: [../analyze/0279-qualified-import-model-reference.md](../analyze/0279-qualified-import-model-reference.md)

## Что было

Цикл по заметкам жил **внутри** `format_compile_error`, а `takt-sim` строил
текст диагностики своей функцией `format_diagnostic` — позицию он брал у общего
носителя (`position_prefix`, фича 0053), а заметок не печатал вовсе.

Замер 2026-08-19, один вход (`import "lib.takt"; start Main = Lib;`):

| Потребитель | Что печатал |
|---|---|
| `taktc` | `SE-106` **и** сноску «состояния есть у вложенной модели 'Helper'» |
| `takt-sim` | только `SE-106` |

Сноска — единственный указатель выхода из этой ситуации, и её не видел
пользователь эталона.

## Что сделано

Печать заметок вынесена в `diagnostics::format_notes` — **общий носитель**; её
зовут `format_compile_error` и `takt-sim`. Формат заметки (позиция по правилу
0243, префикс `примечание:`) теперь физически один, а не по договорённости.

⚠️ Это ровно тот класс, о котором предупреждает док-строка
`format_compile_error`: «копия формата в `taktc` уже расходилась однажды
(задача 0028-01)». Вторая копия жила в другом крейте и разошлась молча.

## Проверки

```sh
cargo test --test sim diagnostic_notes   # сквозной: гоняет бинарник takt-sim
```

Мутация «не печатать заметки» валит сторож.
