//! Подкоманда `compile` утилиты `taktc` — разбор аргументов и исполнение.
//!
//! Логика вынесена из бинарника `taktc` в библиотеку по правилу размера модуля
//! (`bin/taktc.rs` пришпилен к baseline и расти не может): бинарник держит лишь
//! тонкий диспетчер `args[1] == "compile" → run_compile`. Прецедент — подкоманды
//! `address-map` (`address_map::export_cli`) и `verify`
//! (`verification::verify_cli`), фича 0043.
//!
//! Разбор аргументов — тот же ручной `while i < args.len()` + `match`, что и у
//! соседних подкоманд: `clap` в проект не заводится.

use crate::address_map::split_include_dirs;
use std::fs;
use std::process;

#[cfg(test)]
mod tests;

/// Режим применения параметров модели (`--parameters=`, фича 0185, ADR Option E).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParametersMode {
    /// Модель одна; значения аргументов присваиваются полям экземпляра после
    /// его инициализации. Умолчание — заказчик выбрал простоту (ADR 0185).
    #[default]
    Assign,
    /// Копия модели на каждый различный набор значений: свои `_init` в `c`,
    /// свои `FUNCTION_BLOCK` в `st`, вывод константности (задача 0185-05).
    Specialize,
}

/// Параметры компиляции, разобранные из аргументов командной строки.
#[derive(Debug, PartialEq)]
pub struct CompileOptions {
    /// Целевой язык генерации (по умолчанию `"c"`).
    pub target: String,
    /// Путь к входному `.takt`-файлу.
    pub input_file: String,
    /// Путь к выходному файлу или директории.
    pub output_path: String,
    /// Список директорий для поиска файлов `import`.
    ///
    /// Заполняется через флаги `-I` / `--include-dirs`.
    /// Порядок элементов соответствует порядку указания флагов —
    /// первый найденный файл используется (приоритет первого пути).
    pub include_dirs: Vec<String>,
    /// Расширенный диагностический вывод.
    ///
    /// При `true` выводятся все предупреждения и полные пути к файлам.
    /// Взаимоисключается с [`quiet`](CompileOptions::quiet).
    pub verbose: bool,
    /// Тихий режим.
    ///
    /// При `true` подавляются все сообщения, кроме ошибок компиляции.
    /// Взаимоисключается с [`verbose`](CompileOptions::verbose).
    pub quiet: bool,
    /// Флаг включения генерации проверок Guard-формул.
    pub guard_enable: bool,
    /// Символы платформы для выражений адреса: сырые аргументы `--define`
    /// (фича 0042). Разбор в среду — `takt_lang::parse_defines`.
    ///
    /// По умолчанию пусто → без флага поведение `taktc` идентично прежнему.
    pub defines: Vec<String>,
    /// Путь к внешней карте адресов (`.ld`-подобный формат, фича 0020).
    ///
    /// Заполняется флагом `--address-map`. Если задан, карта разбирается и
    /// накладывается на модель оверлеем (с предупреждениями об оверлее/висячих
    /// записях). Понижение адреса в целевой код — задача 0020-05.
    pub address_map: Option<String>,
    /// Ширина вещественного типа в порождаемом C (фича 0029).
    ///
    /// Заполняется флагом `--float-width=32|64`; умолчание — 64 (`double`),
    /// что совпадает с точностью симулятора (f64). Значение 32 (`float`) — для
    /// платформ, где 8-байтное чтение недопустимо.
    pub float_width: crate::FloatWidth,
    /// Глобальная точность `q(m, n)` для реализации `float` (фича 0096).
    ///
    /// Заполняется флагом `--float-as-q=<m>.<n>` (границы правила 1 ADR 0061:
    /// `m ≥ 1`, `n ≥ 1`, `m + n ≤ 64`). `None` без флага — прежнее поведение.
    pub float_as_q: Option<(u8, u8)>,
    /// Реализовать `float` целочисленным Q-путём в `c`/`rust`/`st` (embedded) —
    /// флаг `--float-embedded` (фича 0096). Действует только с `--float-as-q`.
    pub float_embedded: bool,
    /// Режим применения параметров модели — флаг `--parameters=` (фича 0185).
    ///
    /// Умолчание [`ParametersMode::Assign`]: модель одна, значения аргументов
    /// присваиваются полям экземпляра после его инициализации.
    /// `--parameters=specialize` — копия модели на каждый различный набор
    /// значений (задача 0185-05; до неё — честный отказ, не молчание).
    /// Неизвестное значение — ошибка CLI с перечислением допустимых, а не
    /// молчаливое умолчание (критерий A11 анализа 0185).
    pub parameters: ParametersMode,
    /// Частота такта устройства в герцах — флаг `--tick-hz` (фича 0134).
    ///
    /// Включает профиль «такты»: длительности пересчитываются в число тактов.
    /// Если модель объявила `clock`, флаг **обязан совпасть** (контракт 0134-05,
    /// `SE-069`/`SE-070`); если не объявила — флаг задаёт частоту. `None` без
    /// флага — профиль «часы» (внешний источник времени).
    pub tick_hz: Option<u64>,
}

/// Разбирает аргументы подкоманды `compile` в [`CompileOptions`].
///
/// Принимает слайс без имени программы и без `"compile"` в начале
/// (т.е. аргументы, следующие сразу после `"compile"`).
///
/// # Поддерживаемые флаги
///
/// | Флаг                   | Описание                                  |
/// |------------------------|-------------------------------------------|
/// | `--target`, `-t`       | Целевой язык: `c` (по умолчанию), `c-hal`, `plantuml`, `st`, `st-at`, `rust` |
/// | `--output`, `-o`       | Путь к выходному файлу/директории         |
/// | `--include-dirs`, `-I` | Пути поиска импортов (`:` или `;`)        |
/// | `-I<путь>`             | Слитная форма без пробела                 |
/// | `--tick-hz=<n>`        | Частота такта устройства (Гц), профиль «такты» (фича 0134) |
/// | `--verbose`, `-v`      | Расширенный вывод (все предупреждения)    |
/// | `--quiet`, `-q`        | Тихий режим (только ошибки)               |
///
/// Флаги `--verbose` и `--quiet` взаимоисключающие.
/// Флаг `-I` можно повторять; все пути объединяются в один список.
///
/// # Ошибки
///
/// Возвращает строку с описанием ошибки при отсутствии входного файла, флаге без
/// обязательного аргумента, одновременном `--verbose`/`--quiet` или неизвестном
/// флаге.
pub fn parse_compile_args(args: &[String]) -> Result<CompileOptions, String> {
    let mut target = "c".to_string();
    let mut input_file: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut include_dirs: Vec<String> = Vec::new();
    let mut defines: Vec<String> = Vec::new();
    let mut verbose = false;
    let mut quiet = false;
    let mut guard_enable = true;
    let mut address_map: Option<String> = None;
    let mut float_width = crate::FloatWidth::default();
    let mut float_as_q: Option<(u8, u8)> = None;
    let mut parameters = ParametersMode::default();
    let mut float_embedded = false;
    let mut tick_hz: Option<u64> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--target" | "-t" => {
                i += 1;
                match args.get(i) {
                    Some(v) => target = v.clone(),
                    None => return Err(format!("{} требует аргумент", arg)),
                }
            }
            "-o" | "--output" => {
                i += 1;
                match args.get(i) {
                    Some(v) => output_path = Some(v.clone()),
                    None => return Err(format!("{} требует аргумент", arg)),
                }
            }
            "-D" | "--define" => {
                i += 1;
                match args.get(i) {
                    Some(v) => defines.push(v.clone()),
                    None => return Err(format!("{} требует аргумент", arg)),
                }
            }
            // Слитная форма: -DNAME=VALUE. Ветка стоит ПОСЛЕ раздельной — иначе
            // она перехватывала бы сам `-D` (образец: `-I` ниже).
            s if s.starts_with("-D") && s.len() > 2 => {
                defines.push(s[2..].to_string());
            }
            "-I" | "--include-dirs" => {
                i += 1;
                match args.get(i) {
                    Some(v) => include_dirs.extend(split_include_dirs(v)),
                    None => return Err(format!("{} требует аргумент", arg)),
                }
            }
            // Слитная форма: -I/path или -I/a:/b
            s if s.starts_with("-I") && s.len() > 2 => {
                include_dirs.extend(split_include_dirs(&s[2..]));
            }
            "--verbose" | "-v" => {
                verbose = true;
            }
            "--quiet" | "-q" => {
                quiet = true;
            }
            "--guard-enable" => {
                guard_enable = true;
            }
            "--guard-disable" => {
                guard_enable = false;
            }
            "--address-map" => {
                i += 1;
                match args.get(i) {
                    Some(v) => address_map = Some(v.clone()),
                    None => return Err(format!("{} требует аргумент", arg)),
                }
            }
            // Фича 0029: ширина вещественного типа. Принимаются обе формы —
            // `--float-width=32` (как в тест-плане) и `--float-width 32` (как у
            // прочих флагов): расходиться с соседями по синтаксису нечего.
            "--float-width" => {
                i += 1;
                match args.get(i) {
                    Some(v) => float_width = parse_float_width(v)?,
                    None => return Err(format!("{} требует аргумент: 32 или 64", arg)),
                }
            }
            a if a.starts_with("--float-width=") => {
                float_width = parse_float_width(&a["--float-width=".len()..])?;
            }
            // `--float-as-q=m.n` — глобальная точность реализации float (0096).
            "--float-as-q" => {
                i += 1;
                match args.get(i) {
                    Some(v) => float_as_q = Some(parse_float_as_q(v)?),
                    None => return Err(format!("{} требует аргумент: m.n (напр. 10.22)", arg)),
                }
            }
            a if a.starts_with("--float-as-q=") => {
                float_as_q = Some(parse_float_as_q(&a["--float-as-q=".len()..])?);
            }
            "--float-embedded" => float_embedded = true,
            // Фича 0185: режим применения параметров модели. Только слитная
            // форма со значением: флаг без значения — ошибка, не умолчание.
            a if a.starts_with("--parameters=") => {
                parameters = parse_parameters_mode(&a["--parameters=".len()..])?;
            }
            "--parameters" => {
                return Err(
                    "--parameters требует значение: --parameters=assign|specialize".to_string(),
                );
            }
            // Фича 0134: частота такта устройства. Обе формы, как у соседей.
            "--tick-hz" => {
                i += 1;
                match args.get(i) {
                    Some(v) => tick_hz = Some(parse_tick_hz(v)?),
                    None => return Err(format!("{} требует аргумент: частоту в Гц", arg)),
                }
            }
            a if a.starts_with("--tick-hz=") => {
                tick_hz = Some(parse_tick_hz(&a["--tick-hz=".len()..])?);
            }
            // Позиционный аргумент — входной файл
            a if !a.starts_with('-') => {
                input_file = Some(a.to_string());
            }
            unknown => {
                return Err(format!("неизвестный флаг '{}'", unknown));
            }
        }
        i += 1;
    }

    // Флаги --verbose и --quiet взаимоисключающие
    if verbose && quiet {
        return Err(
            "флаги --verbose и --quiet взаимоисключающие: нельзя указывать оба одновременно"
                .to_string(),
        );
    }

    let input_file = input_file.ok_or_else(|| "не указан входной файл".to_string())?;
    let output_path = output_path.unwrap_or_else(|| "output".to_string());

    Ok(CompileOptions {
        target,
        input_file,
        output_path,
        include_dirs,
        defines,
        verbose,
        quiet,
        guard_enable,
        address_map,
        float_width,
        float_as_q,
        float_embedded,
        parameters,
        tick_hz,
    })
}

/// Разбирает значение флага `--parameters=` (фича 0185).
///
/// Неизвестное значение — ошибка с перечислением допустимых, а не молчаливое
/// умолчание: `--parameters=specialise` (опечатка) обязан сказать, что такого
/// режима нет, а не собрать код другим режимом.
fn parse_parameters_mode(value: &str) -> Result<ParametersMode, String> {
    match value {
        "assign" => Ok(ParametersMode::Assign),
        "specialize" => Ok(ParametersMode::Specialize),
        other => Err(format!(
            "--parameters={} — неизвестный режим; допустимы: assign (умолчание), specialize",
            other
        )),
    }
}

/// Разбирает значение флага `--float-width` (фича 0029).
///
/// Допустимы только 32 и 64: иных вещественных типов у цели `c` нет. Прочее —
/// ошибка разбора, а не молчаливое умолчание: `--float-width=16` должен сказать,
/// что такой ширины не бывает, а не собрать код с другой точностью.
fn parse_float_width(value: &str) -> Result<crate::FloatWidth, String> {
    match value {
        "32" => Ok(crate::FloatWidth::W32),
        "64" => Ok(crate::FloatWidth::W64),
        other => Err(format!(
            "--float-width: недопустимое значение '{}' (допустимо 32 или 64)",
            other
        )),
    }
}

/// Разбирает значение флага `--float-as-q=<m>.<n>` в точность `q(m, n)` (0096).
///
/// Границы — те же, что у типа `q` (правило 1 ADR 0061): `m ≥ 1`, `n ≥ 1`,
/// `m + n ≤ 64`. Нарушение или неверный формат — **ошибка CLI**, а не молчаливое
/// умолчание: угадывать точность нельзя (ровно довод, по которому 0045/0061
/// отвергли автоугадывание формата).
fn parse_float_as_q(value: &str) -> Result<(u8, u8), String> {
    let (m_str, n_str) = value.split_once('.').ok_or_else(|| {
        format!("--float-as-q: ожидался формат m.n (напр. 10.22), получено '{value}'")
    })?;
    let parse = |s: &str, what: &str| -> Result<u8, String> {
        s.parse::<u8>()
            .map_err(|_| format!("--float-as-q: {what} '{s}' — не целое 0..255"))
    };
    let (m, n) = (parse(m_str, "m")?, parse(n_str, "n")?);
    if m < 1 || n < 1 || (m as u16) + (n as u16) > 64 {
        return Err(format!(
            "--float-as-q: q({m}, {n}) вне границ (m ≥ 1, n ≥ 1, m + n ≤ 64)"
        ));
    }
    Ok((m, n))
}

/// Разбирает значение флага `--tick-hz` — частоту такта в герцах (фича 0134).
///
/// Значение — целое число Гц (`--tick-hz=1000`), совпадающее по форме с текстом
/// диагностики контракта `SE-069`. Ноль и нечисло — **ошибка CLI**: частота
/// такта не бывает нулевой, а угадывать нельзя (тот же довод, что у `--float-as-q`).
fn parse_tick_hz(value: &str) -> Result<u64, String> {
    let hz = value
        .parse::<u64>()
        .map_err(|_| format!("--tick-hz: '{value}' — не целое число герц"))?;
    if hz == 0 {
        return Err("--tick-hz: частота такта не может быть нулевой".to_string());
    }
    Ok(hz)
}

/// Собирает опции генерации из разобранных аргументов CLI.
///
/// Одна точка сборки на все цели: иначе новая опция доезжает до одних целей и
/// не доезжает до других, а расхождение обнаруживается на выходе генератора.
/// Цели `st`/`st-at` `float_width` не потребляют — у ST своё отображение типов
/// (`generator/st/st_type.rs`).
fn generate_options(options: &CompileOptions) -> crate::GenerateOptions {
    let mut generate = crate::GenerateOptions::new(options.guard_enable);
    generate.float_width = options.float_width;
    generate.float_as_q = options.float_as_q;
    generate.float_embedded = options.float_embedded;
    generate.tick_hz = options.tick_hz;
    // Режим параметров (фича 0185): `specialize` включает копирование моделей
    // по наборам аргументов между стадиями 1 и 2 семантики.
    generate.specialize = options.parameters == ParametersMode::Specialize;
    generate
}

/// Печатает ошибку компиляции. Формат — в библиотеке (фича 0130), потому что
/// вид диагностики есть её свойство, а не свойство бинарника; копия формата
/// в бинарнике уже расходилась по целям (задача 0028-01).
fn print_compile_error(diag: &crate::diagnostics::Diagnostic) {
    eprintln!("{}", crate::diagnostics::format_compile_error(diag));
}

/// Печатает предупреждения общим форматом, **разрешив номер файла в путь**
/// (фича 0228).
///
/// ⚠️ Путь ставится **реестром файлов**, а не именем входа: у диагностики
/// импортированного файла смещения принадлежат чужому тексту, и штамп «путь
/// входа» дал бы верный код с **неверными** координатами. `path_of` отвечает
/// `None` на незарегистрированный номер, поэтому такая диагностика останется без
/// префикса — как прежде, но не соврёт.
///
/// Прежде предупреждения печатались **без позиции вовсе**: `format_warning`
/// зовёт `position_prefix`, а тот без пути возвращает пустую строку. Координата в
/// `loc` при этом была — терялась она на печати.
fn print_warnings(
    warnings: &[crate::diagnostics::Diagnostic],
    files: &crate::diagnostics::FileTable,
    quiet: bool,
) {
    if quiet {
        return;
    }
    for w in warnings {
        let stamped = w.clone().with_file_if_unset(files.path_of(&w.loc));
        eprintln!("{}", crate::diagnostics::format_warning(&stamped));
    }
}

/// Печатает результат простой цели (`Result<(), Diagnostic>`: `plantuml`/`st`/
/// `rust`/`sv`) и завершает процесс при ошибке. verbose даёт полный путь входа.
/// Цель `c` не пользуется этим: её сообщение перечисляет пути поиска.
fn report_simple_result(
    result: Result<(), crate::diagnostics::Diagnostic>,
    target: &str,
    options: &CompileOptions,
) {
    if let Err(diag) = result {
        print_compile_error(&diag);
        process::exit(1);
    }
    if options.quiet {
        return;
    }
    if options.verbose {
        eprintln!(
            "Скомпилировано: {} → {} ({})",
            fs::canonicalize(&options.input_file)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| options.input_file.clone()),
            options.output_path,
            target,
        );
    } else {
        eprintln!(
            "Скомпилировано: {} → {}/ ({})",
            options.input_file, options.output_path, target
        );
    }
}

/// Печатает результат адрес-потребляющей цели (`c-hal`/`st-at`/`sv-mmio`) и
/// завершает процесс при ошибке. Тело идентично (возвращают `Ok(warnings)` либо
/// `Err`), поэтому вынесено сюда.
fn report_hal_result(
    result: Result<Vec<crate::diagnostics::Diagnostic>, crate::diagnostics::Diagnostic>,
    target: &str,
    options: &CompileOptions,
) {
    match result {
        Ok(warnings) => {
            // Реестр здесь — только корневой файл: цель строит своё дерево внутри
            // и таблицу наружу не отдаёт. Предупреждение своего файла получит
            // позицию, импортированного — останется без префикса (как прежде).
            let files = crate::diagnostics::FileTable::new(&options.input_file);
            print_warnings(&warnings, &files, options.quiet);
            if !options.quiet {
                eprintln!(
                    "Скомпилировано: {} → {}/ ({})",
                    options.input_file, options.output_path, target
                );
            }
        }
        Err(diag) => {
            print_compile_error(&diag);
            process::exit(1);
        }
    }
}

/// Исполняет подкоманду `compile`; возвращает код возврата процесса.
///
/// Принимает аргументы без имени программы и без `"compile"` в начале. Ошибки
/// печатаются здесь же, код `1` — при любой из них; `0` — при успехе.
pub fn run_compile(args: &[String]) -> i32 {
    let options = match parse_compile_args(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Ошибка разбора аргументов: {e}");
            eprintln!("Подсказка: см. `taktc --help`.");
            return 1;
        }
    };

    // Читаем исходный файл
    let source = match fs::read_to_string(&options.input_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Ошибка чтения файла '{}': {}", options.input_file, e);
            return 1;
        }
    };

    // Внешняя карта адресов (фича 0020): разбор один раз. В режиме `c-hal` карта
    // участвует в разрешении адресов (compile_to_c_hal); для остальных целей —
    // только информационные предупреждения об оверлее/висячих записях (0020-03).
    let external_entries: Vec<crate::AddressMapEntry> = if let Some(map_path) = &options.address_map
    {
        let map_src = match fs::read_to_string(map_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Ошибка чтения карты адресов '{}': {}", map_path, e);
                return 1;
            }
        };
        match crate::parse_address_map(&map_src, 0) {
            Ok(entries) => entries,
            Err(diags) => {
                for d in diags {
                    eprintln!(
                        "Ошибка карты адресов [{}]: {}",
                        d.code.as_deref().unwrap_or("?"),
                        d.message
                    );
                }
                return 1;
            }
        }
    } else {
        Vec::new()
    };

    // Символы платформы для выражений адреса (фича 0042). Разбор один раз, до
    // компиляции: битый аргумент — ошибка CLI, а не повод собрать не тот адрес.
    let address_env = match crate::parse_defines(&options.defines) {
        Ok(env) => env,
        Err(diags) => {
            for d in diags {
                eprintln!(
                    "Ошибка --define [{}]: {}",
                    d.code.as_deref().unwrap_or("?"),
                    d.message
                );
            }
            return 1;
        }
    };

    // Все ошибки за один прогон (фича 0130): проверка идёт ДО компиляции цели,
    // потому что `compile_to_*` отдают одну ошибку — первую. Прежде пользователь
    // получал по одной ошибке за прогон даже там, где парсер нашёл все сразу.
    // Режим параметров передаётся и сюда: `after PARAM` законен в `specialize` и
    // отвергается в `assign` (`SE-088`), поэтому проверка обязана идти в том же
    // режиме, в каком пойдёт генерация (фича 0185, R12).
    let errors = crate::collect_compile_diagnostics(
        &options.input_file,
        &source,
        &options.include_dirs,
        options.parameters == ParametersMode::Specialize,
    );
    if !errors.is_empty() {
        for diag in &errors {
            print_compile_error(diag);
        }
        return 1;
    }

    // Предупреждения компилятора (фича 0081): единая точка `collect_model_warnings`
    // печатается для **всех** целей (до 0081 CLI не звал ни `SE-036`, ни `SE-037`).
    // Адрес-специфичные (оверлей карты, сломанное выражение адреса) — только у
    // целей, адрес НЕ потребляющих: у `c-hal`/`st-at` это ошибки при генерации.
    // Пересчёт длительностей (фича 0134) печатается тем же каналом.
    let consumes_addresses = matches!(options.target.as_str(), "c-hal" | "st-at" | "sv-mmio");
    // Реестр файлов, а не однодневка `construct_model` (фича 0228): без него
    // предупреждение не знает своего пути, а `position_prefix` без пути отдаёт
    // пустую строку — координата в `loc` есть, но до пользователя не доезжает.
    let mut files = crate::diagnostics::FileTable::new(&options.input_file);
    if let Some((ast, model)) = crate::parse(&source, 0).ok().and_then(|(ast, _)| {
        crate::semantic::tree::construct_model_with_files(
            &ast,
            None,
            &options.include_dirs,
            &mut files,
            false,
        )
        .ok()
        .map(|m| (ast, m))
    }) {
        let mut warnings = crate::semantic::warnings::collect_model_warnings(&ast, &model);
        if !consumes_addresses {
            if !external_entries.is_empty() {
                warnings.extend(crate::address_map_overlay_warnings(
                    std::rc::Rc::clone(&model),
                    &external_entries,
                ));
            }
            warnings.extend(crate::address_expr_warnings(
                std::rc::Rc::clone(&model),
                &address_env,
            ));
        }
        // Пересчёт длительностей во что превратилась каждая (R10/A10, фича 0134):
        // best-effort по разрешённому профилю. При Err профиля (контракт частоты
        // не выполнен) печатать нечего — ошибку `SE-069`/`SE-070` даст кодоген.
        if target_reports_time(options.target.as_str())
            && let Ok(profile) =
                crate::semantic::duration::resolve_profile(model.borrow().clock_hz, options.tick_hz)
        {
            warnings.extend(crate::semantic::duration::describe_durations(
                &model.borrow(),
                profile,
            ));
        }
        // Формат общий с ошибкой (`print_compile_error`): позиция + код + текст.
        print_warnings(&warnings, &files, options.quiet);
    }

    match options.target.as_str() {
        "c-hal" => {
            report_hal_result(
                crate::compile_to_c_hal(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &external_entries,
                    &address_env,
                    &generate_options(&options),
                ),
                "c-hal",
                &options,
            );
        }
        "c" => {
            if let Err(diag) = crate::compile_to_c(
                &options.input_file,
                &source,
                &options.output_path,
                &options.include_dirs,
                &generate_options(&options),
            ) {
                print_compile_error(&diag);
                return 1;
            }
            // В тихом режиме не выводим информационные сообщения
            if !options.quiet {
                if options.verbose {
                    // Расширенный вывод: полный путь к файлу и список директорий поиска
                    eprintln!(
                        "Скомпилировано: {} → {} (путей поиска: {}: {:?})",
                        fs::canonicalize(&options.input_file)
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| options.input_file.clone()),
                        options.output_path,
                        options.include_dirs.len(),
                        options.include_dirs,
                    );
                } else {
                    eprintln!(
                        "Скомпилировано: {} → {}/ (путей поиска: {})",
                        options.input_file,
                        options.output_path,
                        options.include_dirs.len()
                    );
                }
            }
        }
        "plantuml" => {
            report_simple_result(
                crate::compile_to_plantuml(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                ),
                "plantuml",
                &options,
            );
        }
        "st" => {
            report_simple_result(
                crate::compile_to_st(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &generate_options(&options),
                ),
                "st",
                &options,
            );
        }
        "st-at" => {
            report_hal_result(
                crate::compile_to_st_at(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &external_entries,
                    &address_env,
                    &generate_options(&options),
                ),
                "st-at",
                &options,
            );
        }
        "rust" => {
            report_simple_result(
                crate::compile_to_rust(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &generate_options(&options),
                ),
                "rust",
                &options,
            );
        }
        "sv" => {
            report_simple_result(
                crate::compile_to_sv(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &generate_options(&options),
                ),
                "sv",
                &options,
            );
        }
        "sv-mmio" => {
            report_hal_result(
                crate::compile_to_sv_mmio(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &external_entries,
                    &address_env,
                    &generate_options(&options),
                ),
                "sv-mmio",
                &options,
            );
        }
        t => {
            eprintln!(
                "Ошибка: неизвестная цель '{}'. Поддерживается: c, c-hal, plantuml, st, st-at, rust, sv, sv-mmio",
                t
            );
            return 1;
        }
    }

    0
}

/// Печатает ли цель пересчёт длительностей (фича 0134).
///
/// Пересчёт зависит от профиля времени, а печать «во что превратилась
/// длительность» сегодня подтверждена только для целей на базе C (`c`/`c-hal`).
///
/// ⚠️ Прежняя причина («прочие цели время не порождают») **устарела**: фича 0183
/// дала тип `duration` всем целям, а выдержку `after` они эмитили и раньше.
/// Осталась причина поскромнее: для `rust`/`st`/`sv` формулировка предупреждения
/// не проверена прогоном, и включать её здесь значило бы обещать сверенное
/// поведение без сверки. Расширение — отдельная работа.
fn target_reports_time(target: &str) -> bool {
    matches!(target, "c" | "c-hal")
}
