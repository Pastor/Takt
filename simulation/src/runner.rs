use crate::context::Context;
use crate::eval::value::Value;
use crate::gif::GifRecorder;
use crate::graphics_config::{GraphicsConfig, OutputMode};
use crate::json_input::{Guard, SimStep, json_to_value};
use crate::svg::SvgRecorder;
use crate::unit::viewport::{CachedLayout, LegendData, compute_layout, render_from_layout};
use crate::unit::{TickResult, Unit};
use std::path::PathBuf;

// ── Диспетчер режимов записи графики ─────────────────────────────────────────

pub(crate) enum GraphicsRecorder {
    Gif(GifRecorder),
    Svg(SvgRecorder),
}

impl GraphicsRecorder {
    fn add_frame(
        &mut self,
        viewport: &crate::unit::viewport::Viewport,
        w: u32,
        h: u32,
        delay: Option<u16>,
    ) -> Result<crate::gif::FrameTiming, String> {
        match self {
            Self::Gif(r) => r.add_frame(viewport, w, h, delay),
            Self::Svg(r) => r.add_frame(viewport, w, h, delay),
        }
    }

    fn save(self) -> Result<(), String> {
        match self {
            Self::Gif(r) => r.save(),
            Self::Svg(r) => r.save(),
        }
    }

    fn is_svg(&self) -> bool {
        matches!(self, Self::Svg(_))
    }
}

// ── Имена портов из модели ───────────────────────────────────────────────────

/// Упорядоченные имена портов модели (по направлению).
pub struct PortNames {
    pub in_ports: Vec<String>,
    pub out_ports: Vec<String>,
    pub inout_ports: Vec<String>,
    pub vars: Vec<String>,
}

// ── Результат симуляции ──────────────────────────────────────────────────────

#[derive(Debug)]
pub enum RunResult {
    /// Модель достигла терминального состояния.
    Terminated { steps: usize },
    /// Выполнено заданное количество шагов.
    StepsReached { steps: usize },
    /// Шагов в JSON меньше, чем запрошено — симуляция прервана.
    StepsExhausted { completed: usize, requested: usize },
    /// Guard не выполнен на шаге `step` (нумерация с 1).
    GuardFailed { step: usize, details: String },
    /// Ошибка вычисления на шаге `step` (нумерация с 1): симуляция недостоверна.
    ///
    /// Отличает сломанную модель от честно неактивного перехода (R5 фичи 0025):
    /// раньше ошибка вычисления сводилась к `false` и была неотличима.
    EvalFailed { step: usize, details: String },
}

// ── Бегун симуляции ──────────────────────────────────────────────────────────

pub struct SimulationRunner {
    unit: Unit,
    sim_steps: Vec<SimStep>,
    max_steps: Option<usize>,
    graphics_recorder: Option<GraphicsRecorder>,
    gif_frame_size: Option<(u32, u32)>,
    port_names: PortNames,
    model_name: Option<String>,
    gif_config: GraphicsConfig,
    // Раскладка графа вычисляется один раз перед первым кадром.
    cached_layout: Option<CachedLayout>,
}

impl SimulationRunner {
    pub fn new(
        unit: Unit,
        sim_steps: Vec<SimStep>,
        max_steps: Option<usize>,
        output_dir: Option<&PathBuf>,
        input_stem: &str,
        output_mode: OutputMode,
        port_names: PortNames,
        model_name: Option<String>,
        gif_config: GraphicsConfig,
    ) -> Result<Self, String> {
        let (graphics_recorder, gif_frame_size) = if let Some(dir) = output_dir {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Не удалось создать директорию {}: {e}", dir.display()))?;
            let size = (
                (gif_config.canvas.width + gif_config.legend.width) as u32,
                gif_config.canvas.height as u32,
            );
            let recorder = match output_mode {
                OutputMode::Gif => {
                    let gif_path = dir.join(format!("{input_stem}.gif"));
                    GraphicsRecorder::Gif(GifRecorder::new(
                        &gif_path,
                        gif_config.canvas.frame_delay_cs,
                    ))
                }
                OutputMode::Svg => GraphicsRecorder::Svg(SvgRecorder::new(
                    dir.clone(),
                    input_stem.to_string(),
                    &gif_config,
                )),
            };
            (Some(recorder), Some(size))
        } else {
            (None, None)
        };
        Ok(Self {
            unit,
            sim_steps,
            max_steps,
            graphics_recorder,
            gif_frame_size,
            port_names,
            model_name,
            gif_config,
            cached_layout: None,
        })
    }

    /// Запускает главный цикл симуляции.
    pub fn run(&mut self) -> Result<RunResult, String> {
        // Если загружен файл сценария, он определяет лимит шагов;
        // -n может только уменьшить это число, но не увеличить.
        let sim_len = self.sim_steps.len();
        let limit = if sim_len > 0 {
            self.max_steps.map_or(sim_len, |n| n.min(sim_len))
        } else {
            self.max_steps.unwrap_or(usize::MAX)
        };
        let mut completed = 0usize;

        for step_no in 0..limit {
            let sim_step: Option<SimStep> = self.sim_steps.get(step_no).cloned();

            // Применяем входные порты
            if let Some(step) = &sim_step {
                self.apply_step_inputs(step);
            }

            // Выполняем шаг
            let tick_result = self.unit.tick();
            if let TickResult::Failed(details) = &tick_result {
                return Ok(RunResult::EvalFailed {
                    step: completed + 1,
                    details: details.clone(),
                });
            }
            completed += 1;

            // Выводим информацию о шаге
            self.print_step(completed);

            // Записываем кадры в графику (если нужно)
            if self.graphics_recorder.is_some() {
                // Highlight-кадры для каждого сработавшего перехода (включая параллельные)
                let transitions = self.unit.take_last_transitions();
                for (from, to, _pred) in &transitions {
                    self.capture_frame_with_highlight(Some((from.as_str(), to.as_str())))?;
                }
                // Обычный кадр с новым активным состоянием
                self.capture_frame()?;
            }

            // Проверяем guard
            if let Some(step) = &sim_step {
                if let Some(guard) = &step.guard {
                    let guard = guard.clone();
                    self.check_guard(&guard, step_no + 1)?;
                }
            }

            // Проверяем терминальность
            if tick_result == TickResult::Terminated {
                return Ok(RunResult::Terminated { steps: completed });
            }
        }

        Ok(RunResult::StepsReached { steps: completed })
    }

    /// Сохраняет результат записи (вызывается после завершения run).
    pub fn save_output(self) -> Result<(), String> {
        if let Some(recorder) = self.graphics_recorder {
            recorder.save()?;
        }
        Ok(())
    }

    /// Возвращает ссылку на Unit для чтения состояния после завершения симуляции.
    pub fn unit(&self) -> &Unit {
        &self.unit
    }

    // ── Вспомогательные методы ────────────────────────────────────────────────

    fn print_step(&self, step_no: usize) {
        let states = self.unit.active_states();
        let states_str = if states.is_empty() {
            "—".to_string()
        } else {
            states.join(", ")
        };

        let fmt_group = |names: &[String]| -> String {
            names
                .iter()
                .filter_map(|n| {
                    self.unit
                        .get_value(n)
                        .map(|v| format!("{}={}", n, format_value(&v)))
                })
                .collect::<Vec<_>>()
                .join("  ")
        };

        print!("Шаг {:3}:  [{}]", step_no, states_str);

        for (label, names) in [
            ("in", self.port_names.in_ports.as_slice()),
            ("out", self.port_names.out_ports.as_slice()),
            ("inout", self.port_names.inout_ports.as_slice()),
            ("vars", self.port_names.vars.as_slice()),
        ] {
            let s = fmt_group(names);
            if !s.is_empty() {
                print!("  {}:{}", label, s);
            }
        }
        println!();
    }

    fn apply_step_inputs(&mut self, step: &SimStep) {
        if let Some(in_vals) = &step.in_ports {
            for (i, json_val) in in_vals.iter().enumerate() {
                if let (Some(name), Some(value)) =
                    (self.port_names.in_ports.get(i), json_to_value(json_val))
                {
                    let name = name.clone();
                    self.unit.set_value(&name, value);
                }
            }
        }
        if let Some(inout_vals) = &step.inout {
            for (i, json_val) in inout_vals.iter().enumerate() {
                if let (Some(name), Some(value)) =
                    (self.port_names.inout_ports.get(i), json_to_value(json_val))
                {
                    let name = name.clone();
                    self.unit.set_value(&name, value);
                }
            }
        }
    }

    fn check_guard(&self, guard: &Guard, step_no: usize) -> Result<(), String> {
        if let Some(out_vals) = &guard.out {
            for (i, expected_json) in out_vals.iter().enumerate() {
                let Some(name) = self.port_names.out_ports.get(i) else {
                    continue;
                };
                let Some(expected) = json_to_value(expected_json) else {
                    continue;
                };
                let actual = self.unit.get_value(name);
                if !values_match(&actual, &expected) {
                    return Err(format!(
                        "Guard шага {step_no}: out[{i}] ({name}): ожидалось {:?}, получено {:?}",
                        expected, actual
                    ));
                }
            }
        }
        if let Some(inout_vals) = &guard.inout {
            for (i, expected_json) in inout_vals.iter().enumerate() {
                let Some(name) = self.port_names.inout_ports.get(i) else {
                    continue;
                };
                let Some(expected) = json_to_value(expected_json) else {
                    continue;
                };
                let actual = self.unit.get_value(name);
                if !values_match(&actual, &expected) {
                    return Err(format!(
                        "Guard шага {step_no}: inout[{i}] ({name}): ожидалось {:?}, получено {:?}",
                        expected, actual
                    ));
                }
            }
        }
        if let Some(vars) = &guard.vars {
            for (var_name, expected_json) in vars {
                let Some(expected) = json_to_value(expected_json) else {
                    continue;
                };
                let actual = self.unit.get_value(var_name);
                if !values_match(&actual, &expected) {
                    return Err(format!(
                        "Guard шага {step_no}: vars[{var_name}]: ожидалось {:?}, получено {:?}",
                        expected, actual
                    ));
                }
            }
        }
        Ok(())
    }

    fn capture_frame(&mut self) -> Result<(), String> {
        self.capture_frame_impl(None)
    }

    fn capture_frame_with_highlight(&mut self, edge: Option<(&str, &str)>) -> Result<(), String> {
        // Гарантируем, что раскладка вычислена до поиска индексов
        if self.cached_layout.is_none() && self.gif_frame_size.is_some() {
            self.cached_layout = Some(compute_layout(&self.unit, &self.gif_config));
        }
        let highlighted = edge.and_then(|(from, to)| {
            let layout = self.cached_layout.as_ref()?;
            let fi = layout.node_labels.iter().position(|n| n == from)?;
            let ti = layout.node_labels.iter().position(|n| n == to)?;
            Some((fi, ti))
        });
        self.capture_frame_impl(highlighted)
    }

    fn capture_frame_impl(
        &mut self,
        highlighted_edge: Option<(usize, usize)>,
    ) -> Result<(), String> {
        let (w, h) = match self.gif_frame_size {
            Some(s) => s,
            None => return Ok(()),
        };

        // Раскладка вычисляется один раз: имитация отжига дорогая, но структура
        // модели не меняется в ходе симуляции.
        let layout_ms = if self.cached_layout.is_none() {
            let t = std::time::Instant::now();
            self.cached_layout = Some(compute_layout(&self.unit, &self.gif_config));
            Some(t.elapsed().as_millis())
        } else {
            None
        };

        let active = self.unit.active_states();
        let active_refs: Vec<&str> = active.iter().map(String::as_str).collect();
        let legend = build_legend(&self.unit, &self.port_names);

        let t_vp = std::time::Instant::now();
        let is_svg = self
            .graphics_recorder
            .as_ref()
            .map(|r| r.is_svg())
            .unwrap_or(false);
        let viewport = render_from_layout(
            self.cached_layout.as_ref().unwrap(),
            &self.gif_config,
            &active_refs,
            Some(&legend),
            self.model_name.as_deref(),
            highlighted_edge,
            is_svg,
        )
        .map_err(|d| format!("Ошибка viewport: {}", d.message))?;
        let vp_ms = t_vp.elapsed().as_millis();

        // Highlight-кадры показываются дольше — задержка из конфигурации.
        let delay = if highlighted_edge.is_some() {
            Some(self.gif_config.canvas.highlight_frame_delay_cs)
        } else {
            None
        };
        let frame_timing = if let Some(rec) = &mut self.graphics_recorder {
            Some((rec.is_svg(), rec.add_frame(&viewport, w, h, delay)?))
        } else {
            None
        };

        // Вывод тайминга
        if let Some((is_svg, ft)) = frame_timing {
            if is_svg {
                if let Some(lms) = layout_ms {
                    println!(
                        "           SVG:  раскладка={lms} мс  viewport={vp_ms} мс  запись={} мс",
                        ft.serial_ms
                    );
                } else {
                    println!(
                        "           SVG:  viewport={vp_ms} мс  запись={} мс",
                        ft.serial_ms
                    );
                }
            } else {
                let rast_detail = format!(
                    "svg={} мс  usvg={} мс  render={} мс  quant={} мс",
                    ft.serial_ms, ft.parse_ms, ft.render_ms, ft.quant_ms
                );
                if let Some(lms) = layout_ms {
                    println!(
                        "           GIF:  раскладка={lms} мс  viewport={vp_ms} мс  {rast_detail}"
                    );
                } else {
                    println!("           GIF:  viewport={vp_ms} мс  {rast_detail}");
                }
            }
        }

        Ok(())
    }
}

// ── Вспомогательные функции ───────────────────────────────────────────────────

fn values_match(actual: &Option<Value>, expected: &Value) -> bool {
    match actual {
        None => false,
        Some(v) => match (v, expected) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Real(a), Value::Real(b)) => (a - b).abs() < 1e-9,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Number(a), Value::Real(b)) => (*a as f64 - b).abs() < 1e-9,
            (Value::Real(a), Value::Number(b)) => (a - *b as f64).abs() < 1e-9,
            _ => false,
        },
    }
}

fn format_value(v: &Value) -> String {
    match v {
        Value::Number(n) => n.to_string(),
        Value::Real(f) => format!("{f:.4}"),
        Value::Boolean(b) => b.to_string(),
        // q(m, n): показываем вещественное значение repr·2⁻ⁿ.
        Value::Fixed { repr, n, .. } => format!("{:.4}", *repr as f64 / (1u64 << n) as f64),
        Value::Array(arr) => format!(
            "[{}]",
            arr.iter().map(format_value).collect::<Vec<_>>().join(",")
        ),
        // Структура (фича 0034): `Point{x=7,y=300}` — читаемо и в объявленном
        // порядке полей.
        Value::Struct { name, fields } => format!(
            "{name}{{{}}}",
            fields
                .iter()
                .map(|(f, v)| format!("{f}={}", format_value(v)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn build_legend(unit: &Unit, port_names: &PortNames) -> LegendData {
    let to_entries = |names: &[String]| -> Vec<(String, String)> {
        names
            .iter()
            .map(|n| {
                let v = unit
                    .get_value(n)
                    .map(|v| format_value(&v))
                    .unwrap_or_else(|| "?".to_string());
                (n.clone(), v)
            })
            .collect()
    };
    LegendData {
        in_ports: to_entries(&port_names.in_ports),
        out_ports: to_entries(&port_names.out_ports),
        inout_ports: to_entries(&port_names.inout_ports),
        vars: to_entries(&port_names.vars),
    }
}
