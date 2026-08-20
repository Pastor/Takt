# Задача 0254-02: Гейт ловит служебные имена

> Фича: [../features/0254-legacy-names-internal-identifiers.md](../features/0254-legacy-names-internal-identifiers.md) · ADR: [../adr/0254-legacy-names-internal-identifiers.md](../adr/0254-legacy-names-internal-identifiers.md) · анализ: [../analyze/0254-legacy-names-internal-identifiers.md](../analyze/0254-legacy-names-internal-identifiers.md)

## Что сделано

**Шаблон `scripts/check-legacy-names.sh`** дополнен служебными префиксами:
`lam_…`/`LAM_…` в любом регистре (прежний частный случай `lam_q_` в него
вошёл), `LAMC`, `BUT_…`, имя крейта `lam-generated`.

**Исключение `scripts/precheck.sh` снято.** Оно ставилось как «файл описывает
сам переезд и обязан называть старое имя», а держало **16 живых** вхождений
`LAMC` и устаревший путь к сверке. Комментарии файла переписаны: перечень
запрещённых имён живёт **только** в заголовке гейта (второй список одного
набора разошёлся бы с первым — класс 0084/0193/0195), а описание ссылается на
фичу.

**Сторож `scripts/test-legacy-names.sh`** получил две проверки:

- **7** — каждая из шести форм служебного имени (переменная скрипта, цикл по
  файлам, временный каталог теста, константа LSP, импорт крейта примеров, ключ
  цвета плагина) обязана ловиться;
- **8** — контроль: действующие имена (`TAKTC`, `takt_file`, `takt_…`,
  `TAKT_KEYWORDS`, `takt_generated`) гейт не роняют. Без контроля «ловится»
  означало бы лишь, что гейт ругается на любое имя с подчёркиванием.

## Проверка

```sh
sh scripts/test-legacy-names.sh   # 8 групп проверок
sh scripts/check-legacy-names.sh  # 1162 файла, чисто
```
