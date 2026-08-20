# Тест-план фичи 0294: `SE-102` называет импортёра

> Фича: [../features/0294-se102-suggest-importer.md](../features/0294-se102-suggest-importer.md) · анализ: [../analyze/0294-se102-suggest-importer.md](../analyze/0294-se102-suggest-importer.md) · отчёт: [../reports/0294-se102-suggest-importer.md](../reports/0294-se102-suggest-importer.md)

## Условия проверок

| # | Условие | Как проверяется | Ожидаемый результат |
|---|---|---|---|
| П1 | Импортёр найден и назван | `importer_is_found_and_named` | один путь, оканчивается на `app.takt` |
| П2 | **Контрпример:** посторонний сосед | `unrelated_neighbour_is_not_named` | список пуст, заметки нет |
| П3 | Нет импортёров — нет заметки | `no_importers_means_no_note` | `None` |
| П4 | Неразобранный сосед | `unparsable_neighbour_is_skipped` | найден только настоящий импортёр |
| П5 | Три формы `import` | `every_import_form_is_recognised` | каждая даёт находку |
| П6 | Импортёр из `-I` | `importer_from_search_path_is_found` | найден |
| П7 | Оба инструмента отвечают одинаково | прогон `taktc` и `takt-sim` | одинаковое примечание |
| П8 | У заметки нет координаты | тот же прогон | `примечание: эту библиотеку подключает: …` без `1:1` |
| П9 | Регрессия | `cargo test --all-features` | провалов нет |
| П10 | Предкоммит | `./scripts/precheck.sh` | код 0 |

## Примеры и контрпримеры (правило 16)

**Пример** (библиотека и её импортёр в одном каталоге):

```takt
// helper.takt — библиотека
struct Gains { kp: u8, ki: u8 }
fn scale(a: u8) -> u8 { return a + 1; }
```

```takt
// app.takt — импортёр
import "helper.takt";
start Run { ref Run: false; }
```

`taktc compile -t c helper.takt` → `SE-102` **и** примечание про `app.takt`.

**Контрпример** (сосед без директивы импорта):

```takt
// other.takt
var m: u8 := 0;
start Idle { ref Idle: false; }
```

Он в подсказку не попадает: иначе «назвали файл» значило бы «назвали любой
файл рядом».
