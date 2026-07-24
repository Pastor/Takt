//! CLI-симулятор Takt-моделей.
//!
//! Запускает пошаговую симуляцию модели, переданной в аргументах командной строки.
//! Поддерживает: JSON-файл входных данных, проверку guard, запись в GIF.

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use takt_lang::parse;
use takt_lang::semantic::tree::construct_model_with_files;
use takt_sim::build_unit;
use takt_sim::graphics_config::GraphicsConfig;
use takt_sim::json_input::load_sim_steps;
use takt_sim::runner::{PortNames, RunResult, SimulationRunner};
use takt_sim::state_io;

// ── Аргументы командной строки ────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "simulation", about = "Симуляция Takt-моделей", version)]
struct Args {
    /// Путь к .takt файлу (обязательный)
    lam_file: PathBuf,

    /// Директории поиска include (можно указать несколько)
    #[arg(short = 'I', long = "include", value_name = "DIR")]
    include_paths: Vec<PathBuf>,

    /// Количество шагов (по умолчанию — до терминального состояния)
    #[arg(short = 'n', long = "steps", value_name = "N")]
    steps: Option<usize>,

    /// Директория для сохранения графики (GIF или SVG).
    /// Режим выбирается полем output_mode в --graphics-config ("gif" по умолчанию).
    #[arg(short = 'o', long = "output", value_name = "DIR")]
    output_dir: Option<PathBuf>,

    /// JSON-файл с входными данными и проверками
    #[arg(short = 's', long = "sim-file", value_name = "FILE")]
    sim_file: Option<PathBuf>,

    /// Загрузить состояние модели из JSON-файла перед симуляцией
    #[arg(long = "load-state", value_name = "FILE")]
    load_state: Option<PathBuf>,

    /// Сохранить состояние модели в JSON-файл после симуляции
    #[arg(long = "save-state", value_name = "FILE")]
    save_state: Option<PathBuf>,

    /// Путь к JSON-файлу с настройками генерации GIF
    /// (см. examples/graphics-configs/*.json)
    #[arg(long = "graphics-config", value_name = "FILE")]
    graphics_config: Option<PathBuf>,

    /// Мягкий режим инвариантов (фича 0087): нарушение записывается, и прогон
    /// продолжается, вместо останова. Для отладки — сверки с C у него нет.
    #[arg(long = "invariant-soft")]
    invariant_soft: bool,
}

// ── Точка входа ───────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    env_logger::init();
    let args = Args::parse();

    match run(args) {
        Ok(result) => {
            print_result(&result);
            match &result {
                // Мягкий режим (0087) завершает прогон, но нарушения — находки:
                // не молчим кодом возврата.
                RunResult::GuardFailed { .. }
                | RunResult::EvalFailed { .. }
                | RunResult::CompletedWithInvariantViolations { .. } => ExitCode::FAILURE,
                _ => ExitCode::SUCCESS,
            }
        }
        Err(e) => {
            eprintln!("Ошибка: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<RunResult, String> {
    // 1. Читаем исходный файл LAM
    let source = std::fs::read_to_string(&args.lam_file)
        .map_err(|e| format!("Не удалось прочитать {}: {e}", args.lam_file.display()))?;

    // Реестр файлов: корневой — номер 0, импортируемые получит проход 0.
    // Нужен, чтобы назвать пользователю файл ошибки (фичи 0053, 0054).
    let mut files = takt_lang::diagnostics::FileTable::new(&args.lam_file.to_string_lossy());

    // 2. Парсинг
    let (ast, _comments) =
        parse(&source, 0).map_err(|diags| format_diagnostics("Ошибки парсинга", &diags, &files))?;

    // 3. Семантический анализ
    let search_paths: Vec<String> = args
        .include_paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    // Позиция — в начале строки, как у `taktc` и `rustc`: так её видит редактор.
    // Слова «Семантическая ошибка» не дублируются — об этом говорит код (`SE-…`).
    let model_rc = construct_model_with_files(&ast, None, &search_paths, &mut files)
        .map_err(|d| format_diagnostic(&d, &files))?;

    // 4. Извлекаем имена портов и имя модели
    let port_names = extract_port_names(&model_rc.borrow());
    let model_name = model_rc.borrow().name.clone();

    // 5. Строим Unit
    let mut unit = build_unit(model_rc).map_err(|d| format!("Ошибка построения: {}", d.message))?;

    // 5а. Загружаем сохранённое состояние (если указано)
    if let Some(path) = &args.load_state {
        state_io::load_from_file(&mut unit, path)?;
        println!("Состояние загружено из {}", path.display());
    }

    // 6. Загружаем шаги симуляции (если указан файл)
    let sim_steps = if let Some(path) = &args.sim_file {
        load_sim_steps(path)?
    } else {
        vec![]
    };

    // 6а. Загружаем конфигурацию GIF (если указан --gif-config)
    let gif_config = match &args.graphics_config {
        Some(path) => GraphicsConfig::from_file(path)?,
        None => GraphicsConfig::default(),
    };

    // 7. Создаём и запускаем runner
    // Имя выходного файла берётся из файла симуляции; если он не задан — из LAM-файла.
    let input_stem = args
        .sim_file
        .as_ref()
        .or(Some(&args.lam_file))
        .and_then(|p| p.file_stem())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_string());
    let output_mode = gif_config.output_mode.clone();
    let mut runner = SimulationRunner::new(
        unit,
        sim_steps,
        args.steps,
        args.output_dir.as_ref(),
        &input_stem,
        output_mode,
        port_names,
        model_name,
        gif_config,
    )?;
    runner.set_invariant_soft(args.invariant_soft);

    let result = runner.run()?;

    // 9. Сохраняем состояние модели до потребления runner (если указано)
    if let Some(path) = &args.save_state {
        state_io::save_to_file(runner.unit(), path)?;
        println!("Состояние сохранено в {}", path.display());
    }

    // 8. Сохраняем вывод графики (потребляет runner)
    runner.save_output()?;

    Ok(result)
}

// ── Вспомогательные функции ───────────────────────────────────────────────────

fn extract_port_names(model: &takt_lang::semantic::ModelNode) -> PortNames {
    // 0079: сбор рекурсивен (включая под-модели композиции) — вынесен в
    // библиотеку `PortNames::from_model` ради тестируемости.
    PortNames::from_model(model)
}

/// Печатает диагностики с позицией и кодом (фича 0054).
///
/// Прежде бралось только `d.message` — терялись и позиция, и код (`SE-002`),
/// из-за чего ошибка в своём файле была неотличима от ошибки внутри
/// импортированной библиотеки.
///
/// Печатаются **все** диагностики, а не первая: у разбора их обычно несколько, и
/// каждая — своя подсказка. (`taktc` показывает первую; здесь поведение полезнее и
/// сохранено осознанно.)
fn format_diagnostics(
    prefix: &str,
    diags: &[takt_lang::diagnostics::Diagnostic],
    files: &takt_lang::diagnostics::FileTable,
) -> String {
    let messages: Vec<String> = diags.iter().map(|d| format_diagnostic(d, files)).collect();
    format!("{prefix}:\n{}", messages.join("\n"))
}

/// Одна диагностика: `путь:строка:колонка: [КОД] сообщение`.
///
/// Позиция печатается общей для всех бинарников функцией
/// (`takt_lang::diagnostics::position_prefix`) — формат позиции един у `taktc` и
/// симулятора физически, а не по договорённости.
fn format_diagnostic(
    diag: &takt_lang::diagnostics::Diagnostic,
    files: &takt_lang::diagnostics::FileTable,
) -> String {
    let stamped = stamp_file(diag.clone(), files);
    let code = stamped
        .code
        .as_deref()
        .map(|c| format!("[{c}] "))
        .unwrap_or_default();
    format!(
        "{}{code}{}",
        takt_lang::diagnostics::position_prefix(&stamped),
        stamped.message
    )
}

/// Разрешает номер файла диагностики в путь (приём фичи 0053).
///
/// Реестр — деталь загрузки модели: он жив только здесь, а наружу выходит уже
/// разрешённый путь в `Diagnostic::file`.
fn stamp_file(
    diag: takt_lang::diagnostics::Diagnostic,
    files: &takt_lang::diagnostics::FileTable,
) -> takt_lang::diagnostics::Diagnostic {
    let path = files.path_of(&diag.loc).map(str::to_string);
    diag.with_file_if_unset(path.as_deref())
}

fn print_result(result: &RunResult) {
    match result {
        RunResult::Terminated { steps } => {
            println!("Завершено: модель достигла терминального состояния за {steps} шагов.");
        }
        RunResult::StepsReached { steps } => {
            println!("Выполнено {steps} шагов (лимит достигнут).");
        }
        RunResult::StepsExhausted {
            completed,
            requested,
        } => {
            println!(
                "Предупреждение: выполнено {completed} из {requested} запрошенных шагов (JSON исчерпан)."
            );
        }
        RunResult::GuardFailed { step, details } => {
            eprintln!("ОШИБКА guard на шаге {step}: {details}");
        }
        RunResult::EvalFailed { step, details } => {
            eprintln!("ОШИБКА вычисления на шаге {step}: {details}");
            eprintln!("Симуляция остановлена: результат недостоверен.");
        }
        RunResult::CompletedWithInvariantViolations {
            steps,
            terminated,
            violations,
        } => {
            let how = if *terminated {
                "модель достигла терминального состояния"
            } else {
                "лимит шагов достигнут"
            };
            println!("Прогон завершён ({how}) за {steps} шагов; мягкий режим инвариантов.");
            eprintln!(
                "Нарушений инвариантов: {} (режим --invariant-soft — прогон продолжен):",
                violations.len()
            );
            for (step, details) in violations {
                eprintln!("  шаг {step}: {details}");
            }
        }
    }
}
