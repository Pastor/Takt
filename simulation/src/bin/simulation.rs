//! CLI-симулятор Lam-моделей.
//!
//! Запускает пошаговую симуляцию модели, переданной в аргументах командной строки.
//! Поддерживает: JSON-файл входных данных, проверку guard, запись в GIF.

use clap::Parser;
use grammar::parse;
use grammar::parser::ast::PortDirection;
use grammar::semantic::VariableNode;
use grammar::semantic::tree::construct_model;
use simulation::build_unit;
use simulation::gif_config::GifConfig;
use simulation::json_input::load_sim_steps;
use simulation::runner::{PortNames, RunResult, SimulationRunner};
use simulation::state_io;
use std::path::PathBuf;
use std::process::ExitCode;

// ── Аргументы командной строки ────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "simulation", about = "Симуляция Lam-моделей", version)]
struct Args {
    /// Путь к .lam файлу (обязательный)
    lam_file: PathBuf,

    /// Директории поиска include (можно указать несколько)
    #[arg(short = 'I', long = "include", value_name = "DIR")]
    include_paths: Vec<PathBuf>,

    /// Количество шагов (по умолчанию — до терминального состояния)
    #[arg(short = 'n', long = "steps", value_name = "N")]
    steps: Option<usize>,

    /// Путь для записи GIF-анимации (если указан — запись включена)
    #[arg(short = 'g', long = "gif", value_name = "FILE")]
    gif_output: Option<PathBuf>,

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
    /// (см. examples/gif-configs/*.json)
    #[arg(long = "gif-config", value_name = "FILE")]
    gif_config: Option<PathBuf>,
}

// ── Точка входа ───────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    env_logger::init();
    let args = Args::parse();

    match run(args) {
        Ok(result) => {
            print_result(&result);
            match &result {
                RunResult::GuardFailed { .. } => ExitCode::FAILURE,
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

    // 2. Парсинг
    let (ast, _comments) =
        parse(&source, 0).map_err(|diags| format_diagnostics("Ошибки парсинга", &diags))?;

    // 3. Семантический анализ
    let search_paths: Vec<String> = args
        .include_paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let model_rc = construct_model(&ast, None, &search_paths)
        .map_err(|d| format!("Семантическая ошибка: {}", d.message))?;

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
    let gif_config = match &args.gif_config {
        Some(path) => GifConfig::from_file(path)?,
        None => GifConfig::default(),
    };

    // 7. Создаём и запускаем runner
    let mut runner = SimulationRunner::new(
        unit,
        sim_steps,
        args.steps,
        args.gif_output.as_ref(),
        port_names,
        model_name,
        gif_config,
    );

    let result = runner.run()?;

    // 9. Сохраняем состояние модели до потребления runner (если указано)
    if let Some(path) = &args.save_state {
        state_io::save_to_file(runner.unit(), path)?;
        println!("Состояние сохранено в {}", path.display());
    }

    // 8. Сохраняем GIF (потребляет runner)
    runner.save_gif()?;

    Ok(result)
}

// ── Вспомогательные функции ───────────────────────────────────────────────────

fn extract_port_names(model: &grammar::semantic::ModelNode) -> PortNames {
    let mut in_ports = Vec::new();
    let mut out_ports = Vec::new();
    let mut inout_ports = Vec::new();
    let mut vars = Vec::new();

    for (name, var) in &model.variables {
        match var {
            VariableNode::Port { direction, .. } => match direction {
                PortDirection::In => in_ports.push(name.clone()),
                PortDirection::Out => out_ports.push(name.clone()),
                PortDirection::InOut => inout_ports.push(name.clone()),
            },
            VariableNode::Simple { .. } => vars.push(name.clone()),
            _ => {}
        }
    }

    // Сортируем для стабильного порядка
    in_ports.sort();
    out_ports.sort();
    inout_ports.sort();
    vars.sort();

    PortNames {
        in_ports,
        out_ports,
        inout_ports,
        vars,
    }
}

fn format_diagnostics(prefix: &str, diags: &[grammar::diagnostics::Diagnostic]) -> String {
    let messages: Vec<String> = diags.iter().map(|d| d.message.clone()).collect();
    format!("{prefix}:\n{}", messages.join("\n"))
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
    }
}
