# Реестр кодов диагностик Takt

> **Единый источник истины** по кодам диагностик (`SE-036`, `CC-014`, `SIM-010`,
> …). Заведён фичей [0077](../features/0077-diagnostic-code-registry.md)
> ([ADR 0077](../adr/0077-diagnostic-code-registry.md)). Прежде коды жили только в
> исходниках (голыми литералами `Diagnostic::with_code("…")`) и в **вручную**
> пополняемых таблицах `README.md`, которые отставали (AM/PU не описаны, ST —
> частично) и не защищали от коллизий (`CC-014` был выбран двумя фичами).

## Как этим пользоваться

- **Выдаёшь новый код?** Найди ниже секцию нужного **префикса**, возьми
  **следующий свободный номер** (гейт печатает максимальный занятый по каждому
  префиксу), добавь строку сюда **и** используй тот же код в `.with_code("…")`.
  Реестр — источник истины: код без строки здесь **завалит** сборку.
- **Формат кода:** `XX-NNN` — 2–4 заглавные латинские буквы, дефис, ровно три
  цифры с ведущими нулями. Иных форм нет.
- **Пропуски номеров** (нет `CC-009`, `SE-024`…) — норма: код мог быть выведен или
  зарезервирован. Занятый номер **не переиспользуют** под другой смысл.
- **`RESERVED`/`RETIRED`** в столбце «Источник» помечают код, которого в исходнике
  **нет** (забронирован под будущее либо выведен): гейт не требует его эмиссии, но
  и не отдаёт номер повторно. Сейчас таких нет.

## Машинная проверка

`scripts/check-diagnostic-codes.sh` (в `precheck.sh` и CI) сверяет реестр с
исходниками: каждый эмитируемый код обязан быть здесь (иначе — ошибка
«незадокументированный код»), каждая строка реестра — встречаться в исходнике
(иначе — «протухшая запись»), формат `XX-NNN` соблюдён, дублей строк нет. Печатает
по префиксам максимальный занятый номер — это и есть «следующий свободный».

## Префиксы

| Префикс | Слой | Крейт |
|---|---|---|
| `LE` | лексические ошибки | `takt-lang` (`parser/lexer.rs`, диспетчер `code()`) |
| `SY` | синтаксические ошибки | `takt-lang` (парсер) |
| `SE` | семантические ошибки | `takt-lang` (`semantic/`, `address_map/`) |
| `CC` | цель C (кодоген) | `takt-lang` (`generator/c/`) |
| `RS` | цель Rust | `takt-lang` (`generator/rust/`) |
| `SV` | цель SystemVerilog | `takt-lang` (`generator/sv/`) |
| `ST` | цель Structured Text | `takt-lang` (`generator/st/`) |
| `AM` | парсинг внешней карты адресов | `takt-lang` (`address_map/parse.rs`) |
| `DF` | флаг `--define` | `takt-lang` (`address_map/`) |
| `PU` | цель PlantUML | `takt-lang` (`generator/plantuml/`) |
| `SIM` | вычисление в симуляторе | `takt-sim` (`eval/`, диспетчер `EvalError::code()`) |

> Столбец «Источник» — **первое** место эмиссии кода (файл:строка). Информативен;
> код может эмитироваться из нескольких мест с тем же смыслом (напр. `CC-015`).
> Тексты сообщений в столбце «Значение» — краткая суть, а не дословный текст
> диагностики (тот строится `format!` в исходнике).

## Реестр

### AM — карта адресов

| Код | Значение | Источник |
|---|---|---|
| `AM-001` | ожидалось имя порта | `takt-lang/src/address_map/parse.rs:174` |
| `AM-002` | ожидался '=' после имени порта '…' | `takt-lang/src/address_map/parse.rs:202` |
| `AM-003` | ожидался адрес для порта '…' | `takt-lang/src/address_map/parse.rs:224` |
| `AM-004` | ожидался ';' после адреса порта '…' | `takt-lang/src/address_map/parse.rs:237` |
| `AM-005` | ожидался адрес для порта '…' | `takt-lang/src/address_map/parse.rs:227` |
| `AM-006` | повторная запись адреса для порта '…' во внешней карте | `takt-lang/src/address_map/parse.rs:137` |

### CC — цель C

| Код | Значение | Источник |
|---|---|---|
| `CC-001` | BitAccess на порт типа `float` не поддерживается | `takt-lang/src/generator/c/c_expr/condition.rs:341` |
| `CC-002` | Неразрешённая функция в условии перехода | `takt-lang/src/generator/c/c_expr/condition.rs:364` |
| `CC-003` | Ссылки на модели и состояния не поддерживаются в условиях переходов | `takt-lang/src/generator/c/c_expr/condition.rs:390` |
| `CC-004` | Модель с таким именем не найдена в карте (кодоген) | `takt-lang/src/generator/c/c_map.rs:30` |
| `CC-005` | Состояние с таким именем не найдено в карте (кодоген) | `takt-lang/src/generator/c/c_map.rs:42` |
| `CC-006` | Модель … не найдена | `takt-lang/src/generator/c/c_expr/condition.rs:100` |
| `CC-007` | Неподдерживаемый тип элемента конкатенации | `takt-lang/src/generator/c/c_model.rs:352` |
| `CC-008` | Начальное состояние модели не определено | `takt-lang/src/generator/c/c_model.rs:747` |
| `CC-010` | Ошибка записи выходного файла (`.h` или `.c`) | `takt-lang/src/generator/c/mod.rs:140` |
| `CC-011` | Состояние … не найдено в модели … | `takt-lang/src/generator/c/c_expr/condition.rs:111` |
| `CC-012` | Модель … не найдена | `takt-lang/src/generator/c/c_expr/condition.rs:92` |
| `CC-013` | Выражение … не разыменовано | `takt-lang/src/generator/c/c_expr/condition.rs:78` |
| `CC-014` | ~~Бит-вектор непредставимой ширины~~ — выведен фичей 0078 (`[bit;N]` теперь упаковывается: N≤64 скаляр, N>64 массив `uint64_t[⌈N/64⌉]`) | RETIRED |
| `CC-015` | Тип не представим в C (в т.ч. тип элемента массива) | `takt-lang/src/generator/c/c_hal.rs:141` |
| `CC-016` | Ширина доступа к регистру порта неизвестна для его типа C (`c-hal`) | `takt-lang/src/generator/c/c_hal.rs:152` |
| `CC-017` | Инициализатор массива не выразим в C (скалярный либо иной длины) | `takt-lang/src/generator/c/c_model.rs:230` |
| `CC-018` | Условие перехода не переводится в C (причина — заметкой) | `takt-lang/src/generator/c/c_model.rs:482` |
| `CC-019` | состояние модели '…' недостижимо из '…': модель не \ | `takt-lang/src/generator/c/c_expr/condition.rs:165` |
| `CC-020` | **RETIRED** (фича 0183): тип `duration` цель `c` эмитит целым в миллисекундах — отказывать не за что. Номер не переиспользовать под другой смысл | — |

### DF — флаг --define

| Код | Значение | Источник |
|---|---|---|
| `DF-001` | ошибка | `takt-lang/src/address_map/env.rs:124` |
| `DF-002` | ошибка | `takt-lang/src/address_map/env.rs:147` |
| `DF-003` | ошибка | `takt-lang/src/address_map/env.rs:160` |
| `DF-004` | предупреждение | `takt-lang/src/address_map/resolve.rs:156` |

### LE — лексер

| Код | Значение | Источник |
|---|---|---|
| `LE-001` | Незакрытый блочный комментарий | `takt-lang/src/parser/lexer.rs:443` |
| `LE-002` | Незакрытая строка (EOF внутри строкового литерала) | `takt-lang/src/parser/lexer.rs:444` |
| `LE-003` | Незакрытый шестнадцатеричный литерал | `takt-lang/src/parser/lexer.rs:445` |
| `LE-004` | Отсутствует цифра после знака числа | `takt-lang/src/parser/lexer.rs:446` |
| `LE-005` | Недопустимый символ в шестнадцатеричном литерале | `takt-lang/src/parser/lexer.rs:447` |
| `LE-006` | Нераспознанный токен | `takt-lang/src/parser/lexer.rs:448` |
| `LE-007` | Отсутствует показатель степени в числе с плавающей точкой | `takt-lang/src/parser/lexer.rs:449` |
| `LE-008` | Ожидался токен `from` | `takt-lang/src/parser/lexer.rs:450` |
| `LE-009` | Числовой литерал вне диапазона `i64` | `takt-lang/src/parser/lexer.rs:162` |
| `LE-010` | Литерал времени вне представимого диапазона (нс/Гц) | `takt-lang/src/parser/lexer.rs:183` |
| `LE-011` | Единица времени у формы, которая её не допускает (`1.5s`, `1e3ms`, `0xFFms`) | `takt-lang/src/parser/lexer.rs:184` |

### PU — цель PlantUML

| Код | Значение | Источник |
|---|---|---|
| `PU-001` | Ошибка записи файла диаграммы `.puml` (ввод-вывод) | `takt-lang/src/generator/plantuml/mod.rs:47` |

### RS — цель Rust

| Код | Значение | Источник |
|---|---|---|
| `RS-001` | Ошибка записи файла `.rs` (ввод-вывод) | `takt-lang/src/generator/rust/mod.rs:99` |
| `RS-004` | имя даёт `Self`/`self` — непредставимо ни как идентификатор, ни как `r#Self` (отдельное правило языка). Прочие ключевые слова спасает регистр (`type` → `Type`) или `r#type` | `takt-lang/src/generator/rust/rust_name.rs:60` |
| `RS-005` | два имени слипаются после приведения регистра (`floor_sensor` и `FloorSensor`) | `takt-lang/src/generator/rust/rust_name.rs:73` |
| `RS-010` | LTL-формула в теле блока не транслируется в Rust (проверять через `taktc verify`) | `takt-lang/src/generator/rust/rust_stmt.rs:661` |
| `RS-011` | конструкция не транслируется (срез массива, `**`, строка вне `debug`) | `takt-lang/src/generator/rust/rust_expr.rs:38` |
| `RS-012` | Корневой элемент карты не является моделью | `takt-lang/src/generator/rust/mod.rs:112` |
| `RS-013` | Состояние '…' не найдено | `takt-lang/src/generator/rust/rust_map.rs:125` |
| `RS-014` | тип не представим; `RS-015` — `--float-width=32` (цель всегда даёт `f64`) | `takt-lang/src/generator/rust/rust_decl.rs:372` |
| `RS-015` | `--float-width=32` несовместим с целью rust (float → f64, ADR 0050) | `takt-lang/src/generator/rust/rust_type.rs:148` |
| `RS-016` | тип порта не ложится на метод HAL (составной) | `takt-lang/src/generator/rust/rust_port.rs:116` |
| `RS-017` | Обращение к переменной модели из тела функции не транслируется в Rust — передать параметром | `takt-lang/src/generator/rust/rust_expr.rs:155` |
| `RS-018` | чтение выходного порта либо запись во входной | `takt-lang/src/generator/rust/rust_expr.rs:178` |
| `RS-019` | Присваивание в константу '…' недопустимо | `takt-lang/src/generator/rust/rust_expr.rs:542` |
| `RS-020` | условный переход в состояние '…' не переводится в Rust: … | `takt-lang/src/generator/rust/rust_model.rs:1132` |
| `RS-021` | последовательная композиция (`+`) вложена в шаг другой `+` | `takt-lang/src/generator/rust/rust_model.rs:263` |
| `RS-022` | нужен HAL, но он в этой области недоступен | `takt-lang/src/generator/rust/rust_expr.rs:125` |
| `RS-023` | **RETIRED** (фича 0183): тип `duration` цель `rust` эмитит `u32` в миллисекундах — отказывать не за что. Номер не переиспользовать под другой смысл | — |

### SE — семантика

| Код | Значение | Источник |
|---|---|---|
| `SE-001` | Модель не найдена | `takt-lang/src/diagnostics.rs:370` |
| `SE-002` | Ссылка на состояние не найдена | `takt-lang/src/semantic/tree.rs:1565` |
| `SE-003` | Переменная не найдена в области видимости | `takt-lang/src/semantic/expression.rs:98` |
| `SE-004` | Неизвестная встроенная функция | `takt-lang/src/semantic/builtin.rs:50` |
| `SE-005` | Переменная с таким именем уже объявлена | `takt-lang/src/semantic/tree.rs:414` |
| `SE-006` | Модель с таким именем уже объявлена | `takt-lang/src/semantic/tree.rs:251` |
| `SE-007` | Тип '…' уже объявлен | `takt-lang/src/semantic/tree.rs:405` |
| `SE-008` | Условие '…' уже объявлено | `takt-lang/src/semantic/tree.rs:423` |
| `SE-009` | Функция с таким именем уже определена | `takt-lang/src/semantic/tree.rs:658` |
| `SE-010` | Нет терминальных состояний | `takt-lang/src/semantic/validate/states.rs:150` |
| `SE-011` | Нет начального состояния в модели | `takt-lang/src/semantic/minimap.rs:211` |
| `SE-012` | Состояние '…' уже содержит оператор next | `takt-lang/src/semantic/tree.rs:1418` |
| `SE-013` | Файл импорта не найден | `takt-lang/src/semantic/import.rs:90` |
| `SE-014` | Недопустимое расширение файла импорта | `takt-lang/src/semantic/import.rs:129` |
| `SE-015` | Ошибка чтения файла импорта | `takt-lang/src/semantic/import.rs:134` |
| `SE-016` | Не удалось канонизировать путь «…»: … | `takt-lang/src/semantic/import.rs:104` |
| `SE-017` | Условие '…' уже объявлено | `takt-lang/src/semantic/tree.rs:431` |
| `SE-018` | Именованный блок кода при определении должен иметь имя | `takt-lang/src/semantic/tree.rs:606` |
| `SE-019` | Условие при определении должно иметь имя | `takt-lang/src/semantic/tree.rs:538` |
| `SE-020` | Имя состояния не задано | `takt-lang/src/semantic/tree.rs:1391` |
| `SE-021` | Идентификатор не задан | `takt-lang/src/semantic/tree.rs:50` |
| `SE-022` | При определении функция должна иметь имя | `takt-lang/src/semantic/function.rs:39` |
| `SE-023` | Порт должен иметь конкретный тип | `takt-lang/src/semantic/tree.rs:473` |
| `SE-025` | Неразрешённое условие перехода | `takt-lang/src/semantic/validate/common.rs:51` |
| `SE-026` | Запись в входной порт '…' запрещена | `takt-lang/src/semantic/validate/common.rs:207` |
| `SE-027` | Чтение из выходного порта '…' запрещено | `takt-lang/src/semantic/validate/common.rs:129` |
| `SE-028` | Индекс массива вне границ | `takt-lang/src/semantic/expression.rs:129` |
| `SE-029` | Начало среза … выходит за границы массива '…' (размер …) | `takt-lang/src/semantic/expression.rs:414` |
| `SE-030` | Переменная '…' не является массивом | `takt-lang/src/semantic/expression.rs:137` |
| `SE-033` | <анонимная> | `takt-lang/src/semantic/validate/common.rs:40` |
| `SE-034` | Локальный тип '…' не найден | `takt-lang/src/semantic/type_node.rs:115` |
| `SE-035` | Переменная '…' имеет тип bit, но инициализирована значением … \ | `takt-lang/src/semantic/validate/enums.rs:36` |
| `SE-036` | переменная '…' объявлена, но нигде не используется | `takt-lang/src/semantic/unused.rs:418` |
| `SE-037` | Неявное приведение числа к булевому / недетерминированный переход | `takt-lang/src/semantic/validate/implicit_bool.rs:164` |
| `SE-038` | массив размером MAX_ARRAY_SIZE+1 должен давать ошибку Ce15 | `takt-lang/src/semantic/validate/tests_ce15_array_size.rs:39` |
| `SE-039` | псевдоним типа '…' образует циклическую зависимость | `takt-lang/src/semantic/validate/types.rs:149` |
| `SE-040` | структура '…' содержит дублирующееся поле '…' | `takt-lang/src/semantic/validate/structs.rs:43` |
| `SE-041` | поле '…' структуры '…' ссылается на неизвестный тип '…' | `takt-lang/src/semantic/validate/structs.rs:100` |
| `SE-042` | Перекрывающиеся условия переходов | `takt-lang/src/semantic/validate/nondeterminism.rs:218` |
| `SE-043` | Инициализатор переменной — не вариант её перечисления | `takt-lang/src/semantic/validate/enums.rs:171` |
| `SE-044` | лишняя точка с запятой | `takt-lang/src/lib.rs:1136` |
| `SE-045` | неизвестный именованный блок '…'; допустимые имена: enter, exit, always | `takt-lang/src/lib.rs:1125` |
| `SE-046` | состояние '…' недостижимо из начального состояния | `takt-lang/src/semantic/validate/states.rs:314` |
| `SE-047` | условие перехода всегда истинно — переход безусловный | `takt-lang/src/semantic/validate/constant_conditions.rs:61` |
| `SE-048` | оператор `address` ссылается на несуществующий порт '…' | `takt-lang/src/semantic/validate/ports.rs:117` |
| `SE-049` | адрес порта '…' задан оператором `address` более одного раза | `takt-lang/src/semantic/validate/ports.rs:128` |
| `SE-050` | внешняя карта переопределяет адрес порта '…', заданный в модели | `takt-lang/src/address_map/parse.rs:304` |
| `SE-051` | внешняя карта задаёт адрес для несуществующего порта '…' | `takt-lang/src/address_map/parse.rs:317` |
| `SE-052` | Порт используется в кодогенерации, но не имеет адреса ни из одного источника | `takt-lang/src/address_map/resolve.rs:322` |
| `SE-053` | Рекурсия функций запрещена (цикл в графе вызовов) | `takt-lang/src/address_map/resolve.rs:141` |
| `SE-054` | Имя инварианта конфликтует с существующим условием/переменной | `takt-lang/src/address_map/eval.rs:27` |
| `SE-055` | LTL-формула разобрана, но верификация LTL не реализована (предупр.) | `takt-lang/src/address_map/eval.rs:36` |
| `SE-056` | Неизвестный атом LTL-формулы (предупреждение) | `takt-lang/src/semantic/ltl_check.rs:80` |
| `SE-057` | Неизвестный конструктор параметрического типа (единственный — `q(m, n)`) | `takt-lang/src/semantic/type_node.rs:153` |
| `SE-058` | Литерал float вне диапазона точности `q(m, n)` — непредставим | `takt-lang/src/semantic/type_node.rs:303` |
| `SE-059` | неявное смешение типов '…' и '…' в арифметике fixed-point запрещено; \ | `takt-lang/src/semantic/validate/fixed.rs:254` |
| `SE-060` | Бит адреса порта вне диапазона [0, 63] | `takt-lang/src/address_map/resolve.rs:295` |
| `SE-061` | структура '…' не содержит поля '…' | `takt-lang/src/semantic/validate/member_access.rs:105` |
| `SE-062` | Превышен предел вложенности выражений/условий | `takt-lang/src/semantic/validate/depth.rs:77` |
| `SE-063` | Длительность непредставима в выбранном профиле времени | `takt-lang/src/semantic/duration.rs:150` |
| `SE-064` | Длительность не помещается в счётчик времени | `takt-lang/src/semantic/duration.rs:170` |
| `SE-065` | Смешение `duration` с числом в арифметике | `takt-lang/src/semantic/validate/fixed.rs:262` |
| `SE-067` | Частота тактирования объявлена дважды и по-разному | `takt-lang/src/semantic/time_ast.rs:96` |
| `SE-068` | Выдержка `after` вне условия перехода `ref` | `takt-lang/src/semantic/time_ast.rs:63` |
| `SE-069` | Модель объявила `clock`, но `--tick-hz` не передан (контракт частоты) | `takt-lang/src/semantic/duration.rs:251` |
| `SE-070` | `--tick-hz` не совпадает с объявленной `clock` частотой | `takt-lang/src/semantic/duration.rs:259` |
| `SE-071` | Во что пересчиталась длительность (информационное предупреждение) | `takt-lang/src/semantic/duration.rs:314` |
| `SE-072` | Выдержка `after` не сводится к константной длительности | `takt-lang/src/semantic/condition/after_const.rs` |

### SIM — симулятор

| Код | Значение | Источник |
|---|---|---|
| `SIM-001` | Деление или остаток на ноль | `takt-sim/src/eval/error.rs:121` |
| `SIM-002` | Сдвиг на отрицательное число или ≥ 64 бит | `takt-sim/src/eval/error.rs:122` |
| `SIM-003` | Значение не помещается в знаковый тип назначения | `takt-sim/src/eval/error.rs:123` |
| `SIM-004` | Переполнение внутреннего 64-битного представления | `takt-sim/src/eval/error.rs:124` |
| `SIM-005` | Операция не определена для операндов таких типов | `takt-sim/src/eval/error.rs:125` |
| `SIM-006` | Значение нельзя привести к типу назначения | `takt-sim/src/eval/error.rs:126` |
| `SIM-007` | Тип не поддерживается симулятором (например, структуры) | `takt-sim/src/eval/error.rs:127` |
| `SIM-008` | Не удалось разобрать вещественный литерал | `takt-sim/src/expression.rs:251` |
| `SIM-009` | Переменная не найдена | `takt-sim/src/expression.rs:54` |
| `SIM-010` | Ошибка доступа к массиву (не массив, нецелый индекс, выход за границы) | `takt-sim/src/eval/error.rs:138` |
| `SIM-011` | Ошибка доступа к биту (не целое, номер бита вне 0..64) | `takt-sim/src/eval/error.rs:134` |
| `SIM-012` | Доступ к полю у значения, не являющегося структурой | `takt-sim/src/eval/access.rs:132` |
| `SIM-013` | Сравнение с состоянием или моделью пока не поддерживается | `takt-sim/src/predicate.rs:245` |
| `SIM-014` | Конструкция не поддерживается симулятором (строки и др.) | `takt-sim/src/expression.rs:242` |
| `SIM-015` | Пустое выражение/условие не может быть вычислено | `takt-sim/src/expression.rs:232` |
| `SIM-016` | Неразрешённый узел не может быть вычислен | `takt-sim/src/expression.rs:237` |
| `SIM-017` | Оператор пока не поддерживается симулятором | `takt-sim/src/unit/statement.rs:362` |
| `SIM-018` | Превышен предел итераций цикла (100 000) | `takt-sim/src/unit/statement.rs:421` |
| `SIM-019` | Внешняя функция без тела: симуляция значения невозможна | `takt-sim/src/unit/statement.rs:530` |
| `SIM-020` | Встроенная функция пока не поддерживается симулятором | `takt-sim/src/unit/statement.rs:538` |
| `SIM-021` | Неверное число аргументов при вызове функции | `takt-sim/src/unit/statement.rs:559` |
| `SIM-022` | Превышена глубина рекурсии (256) | `takt-sim/src/unit/statement.rs:590` |
| `SIM-023` | Функция не вернула значение | `takt-sim/src/unit/statement.rs:619` |
| `SIM-024` | `break`/`continue` вне цикла | `takt-sim/src/unit/statement.rs:627` |
| `SIM-025` | Нарушен инвариант / Guard-формула (assert языка Takt) | `takt-sim/src/unit/statement.rs:269` |
| `SIM-026` | Инициализатор структуры содержит не столько полей, сколько объявлено | `takt-sim/src/eval/error.rs:129` |
| `SIM-027` | Обращение к несуществующему полю структуры | `takt-sim/src/eval/access.rs:103` |
| `SIM-028` | Несоответствие типа структуры при присваивании | `takt-sim/src/eval/error.rs:131` |
| `SIM-029` | Обращение к структуре по номеру бита | `takt-sim/src/eval/access.rs:109` |

### ST — цель Structured Text

| Код | Значение | Источник |
|---|---|---|
| `ST-001` | Ошибка записи файла `.st` (ввод-вывод) | `takt-lang/src/generator/st/mod.rs:95` |
| `ST-002` | переменная обязана считаться используемой — иначе тест проверял бы фильтр | `takt-lang/src/generator/st/st_decl.rs:662` |
| `ST-004` | оператор address | `takt-lang/src/generator/st/st_at.rs:159` |
| `ST-005` | Порт BOOL без указанного бита — принят бит 0 (предупреждение) | `takt-lang/src/generator/st/st_at.rs:94` |
| `ST-006` | Порт '…' не булев, а в адресе задан бит: у локации %…… \ | `takt-lang/src/generator/st/st_at.rs:113` |
| `ST-007` | Массив нулевого размера ('…') невыразим в IEC 61131-3: \ | `takt-lang/src/generator/st/st_type.rs:177` |
| `ST-008` | … '…' не найдена в модели: объявление типа для ST построить нельзя | `takt-lang/src/generator/st/st_type.rs:233` |
| `ST-009` | Тело внешней функции неизвестно, а IEC 61131-3 требует тела | `takt-lang/src/generator/st/st_func.rs:443` |
| `ST-010` | LTL-формул (…) в блоке кода: в Structured Text они не \ | `takt-lang/src/generator/st/st_stmt.rs:211` |
| `ST-011` | Не транслируется в Structured Text: … | `takt-lang/src/generator/st/st_expr.rs:312` |
| `ST-012` | Корневой элемент карты не является моделью | `takt-lang/src/generator/st/mod.rs:113` |
| `ST-013` | q({m}, {n}): W = … > 32 — точное произведение шириной 2W не влезает в LINT | `takt-lang/src/generator/st/st_fixed.rs:242` |
| `ST-014` | Приведение float → q в цели st (LREAL_TO_INT округляет к ближайшему) | `takt-lang/src/generator/st/st_fixed.rs:177` |
| `ST-015` | Выдержка `after` попала в печатник условий вместо `st_model` (сторож пути: фича 0183 сняла прежний смысл «`duration` не поддерживается») | `takt-lang/src/generator/st/st_expr.rs` |

### SV — цель SystemVerilog

| Код | Значение | Источник |
|---|---|---|
| `SV-001` | Ошибка записи файла `.sv` (ввод-вывод) | `takt-lang/src/generator/sv/mod.rs:95` |
| `SV-002` | цикл в теле, досрочный возврат из функции, `match`, срез массива | `takt-lang/src/generator/sv/mod.rs:539` |
| `SV-003` | `float` в типе **без** `--float-as-q` | `takt-lang/src/generator/sv/sv_fixed.rs:173` |
| `SV-004` | Вещественный тип (float) не существует в синтезируемом RTL | `takt-lang/src/generator/sv/sv_type.rs:88` |
| `SV-005` | `extern fn` | `takt-lang/src/generator/sv/sv_expr.rs:84` |
| `SV-006` | `inout` | `takt-lang/src/generator/sv/sv_mmio.rs:208` |
| `SV-007` | порт с именем `clk`, `rst_n`, `is_done`, `state`, `state_next` | `takt-lang/src/generator/sv/sv_module.rs:371` |
| `SV-008` | неконстантный `enter` стартового состояния | `takt-lang/src/generator/sv/sv_fsm.rs:81` |
| `SV-009` | Деление/остаток по переменному делителю — в RTL нет аппаратного делителя (предупреждение) | `takt-lang/src/generator/sv/sv_expr.rs:108` |
| `SV-010` | Корневой элемент карты не является моделью | `takt-lang/src/generator/sv/mod.rs:119` |
| `SV-011` | Состояние '…' не найдено | `takt-lang/src/generator/sv/sv_map.rs:151` |
| `SV-012` | имя = ключевое слово SystemVerilog (`fork`, `wire`, `time`, …) | `takt-lang/src/generator/sv/sv_module.rs:387` |
| `SV-013` | Порт занимает биты адреса шире регистра цели sv-mmio | `takt-lang/src/generator/sv/sv_mmio.rs:85` |
| `SV-014` | Имя зарезервировано регистровым интерфейсом цели sv-mmio | `takt-lang/src/generator/sv/sv_mmio.rs:101` |
| `SV-015` | Выдержка `after` попала в печатник условий вместо `sv_time` (сторож пути: фича 0183 сняла прежний смысл «`duration` не поддерживается») | `takt-lang/src/generator/sv/sv_expr.rs` |

### SY — парсер

| Код | Значение | Источник |
|---|---|---|
| `SY-001` | Недопустимый токен | `takt-lang/src/lib.rs:151` |
| `SY-002` | Нераспознанный токен | `takt-lang/src/lib.rs:163` |
| `SY-003` | Лишний токен после завершения конструкции | `takt-lang/src/lib.rs:171` |
| `SY-004` | Неожиданный конец файла | `takt-lang/src/lib.rs:176` |

