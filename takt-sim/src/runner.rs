use crate::context::Context;
use crate::eval::value::Value;
use crate::gif::GifRecorder;
use crate::graphics_config::{GraphicsConfig, OutputMode};
use crate::json_input::{Guard, PortValues, SimStep, json_to_value};
// Реестр имён вынесен в свой модуль (фикс 0150-01), но потребители зовут его
// прежним путём `takt_sim::runner::PortNames` — реэкспорт держит контракт.
pub use crate::port_names::{PortDirectionKind, PortNames};
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
    /// Прогон в **мягком** режиме инвариантов (фича 0087) завершился, но по ходу
    /// были нарушения — записаны, а не прерваны. `terminated` = дошёл ли автомат
    /// до терминального состояния (иначе — исчерпал бюджет шагов). `violations`
    /// — пары `(шаг, детали)` в порядке возникновения.
    CompletedWithInvariantViolations {
        steps: usize,
        terminated: bool,
        violations: Vec<(usize, String)>,
    },
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
    /// Мягкий режим инвариантов (фича 0087): нарушение записывается и прогон
    /// продолжается, вместо останова. Умолчание — `false` (жёсткий режим 0044).
    soft_invariants: bool,
    /// Модельное время прогона (наносекунды) — виртуальные часы (фича 0134).
    ///
    /// Часов реального мира в эталоне нет ни при каких условиях: трасса обязана
    /// воспроизводиться, иначе все сверки станут мигающими.
    now_ns: i64,
    /// На сколько продвигать часы за такт, если шаг сценария не сказал иного.
    ///
    /// Умолчание — **1 мс**: прогон без указания времени должен оставаться
    /// возможным, а неявной частоты здесь не появляется — это свойство прогона,
    /// а не модели. Объявленная моделью частота (`clock`) задаёт период такта.
    tick_period_ns: i64,
    /// Сказано ли уже, что сценарий пользуется устаревшей позиционной формой
    /// (фича 0150, `SIM-037`).
    ///
    /// ⚠️ Предупреждение печатается **один раз за прогон**, а не на каждый шаг:
    /// сценарий в сотню шагов дал бы сотню одинаковых строк, и следующее —
    /// настоящее — предупреждение потерялось бы среди повторов. `Cell`, потому
    /// что разбор значений идёт по `&self`.
    positional_form_warned: std::cell::Cell<bool>,
}

impl SimulationRunner {
    #[allow(clippy::too_many_arguments)]
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
            soft_invariants: false,
            positional_form_warned: std::cell::Cell::new(false),
            now_ns: 0,
            tick_period_ns: 1_000_000,
        })
    }

    /// Включает/выключает мягкий режим инвариантов (фича 0087). По умолчанию
    /// выключен (жёсткий режим 0044 — совпадает с `assert()` в C).
    pub fn set_invariant_soft(&mut self, on: bool) {
        self.soft_invariants = on;
    }

    /// Задаёт период такта модельных часов (фича 0134), в наносекундах.
    ///
    /// Источники, в порядке приоритета: поле шага сценария (`time_ms`) →
    /// это значение → умолчание 1 мс. Объявленная моделью частота (`clock`)
    /// переводится в период вызывающим: `1 с / f`.
    pub fn set_tick_period_ns(&mut self, period_ns: i64) {
        self.tick_period_ns = period_ns.max(0);
    }

    /// Текущее модельное время прогона (наносекунды).
    pub fn now_ns(&self) -> i64 {
        self.now_ns
    }

    /// Запускает главный цикл симуляции.
    pub fn run(&mut self) -> Result<RunResult, String> {
        self.warn_about_ambiguous_names();
        // Если загружен файл сценария, он определяет лимит шагов;
        // -n может только уменьшить это число, но не увеличить.
        let sim_len = self.sim_steps.len();
        let limit = if sim_len > 0 {
            self.max_steps.map_or(sim_len, |n| n.min(sim_len))
        } else {
            self.max_steps.unwrap_or(usize::MAX)
        };
        let mut completed = 0usize;
        // Фича 0087: накопленные нарушения инвариантов мягкого режима, с шагом.
        let mut soft_violations: Vec<(usize, String)> = Vec::new();

        for step_no in 0..limit {
            let sim_step: Option<SimStep> = self.sim_steps.get(step_no).cloned();

            // Модельное время (фича 0134) ставится ДО такта: показания часов на
            // такте N обязаны быть видны телу, исполняемому на такте N. Иначе
            // выдержка сдвинулась бы на такт относительно целей — а такой сдвиг
            // компилируется молча (тот же класс, что вход в стартовое состояние,
            // фича 0033).
            // ⚠️ Первый такт идёт при t = 0: часы двигаются ПЕРЕД каждым тактом,
            // кроме первого. Иначе модель входила бы в стартовое состояние уже
            // «спустя период», и выдержка отсчитывалась бы от чужого момента.
            if step_no > 0 {
                let advance_ns = sim_step
                    .as_ref()
                    .and_then(|step| step.time_ms)
                    .map_or(self.tick_period_ns, |ms| ms.saturating_mul(1_000_000));
                self.now_ns = self.now_ns.saturating_add(advance_ns);
            }
            self.unit.set_time_ns(self.now_ns);

            // Применяем входные порты и стенд внешних функций (фича 0209):
            // и то, и другое — вход шага сценария, и ставится оно перед тактом.
            if let Some(step) = &sim_step {
                self.apply_step_inputs(step, step_no + 1)?;
                self.unit
                    .set_extern_stubs(extern_stubs_of(step, step_no + 1)?);
            }

            // Выполняем шаг. В мягком режиме нарушения инвариантов не прерывают
            // такт, а записываются (фича 0087) — сливаем их и тегируем шагом.
            let tick_result = if self.soft_invariants {
                let r = self.unit.tick_soft();
                for details in self.unit.take_invariant_violations() {
                    soft_violations.push((completed + 1, details));
                }
                r
            } else {
                self.unit.tick()
            };
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
            if let Some(step) = &sim_step
                && let Some(guard) = &step.guard
            {
                let guard = guard.clone();
                self.check_guard(&guard, step_no + 1)?;
            }

            // Проверяем терминальность
            if tick_result == TickResult::Terminated {
                if !soft_violations.is_empty() {
                    return Ok(RunResult::CompletedWithInvariantViolations {
                        steps: completed,
                        terminated: true,
                        violations: soft_violations,
                    });
                }
                return Ok(RunResult::Terminated { steps: completed });
            }
        }

        if !soft_violations.is_empty() {
            return Ok(RunResult::CompletedWithInvariantViolations {
                steps: completed,
                terminated: false,
                violations: soft_violations,
            });
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

    /// Предупреждает об именах, объявленных несколькими моделями (фича 0135).
    ///
    /// Пространство имён значений плоское: по голому имени читается ПЕРВАЯ
    /// нашедшаяся ветвь, а запись расходится по всем. Прежде это происходило
    /// молча — модель с одноимёнными портами под-моделей выглядела работающей,
    /// хотя половина её состояния была недоступна. Теперь двусмысленность
    /// названа, и рядом показано, как адресовать точно.
    fn warn_about_ambiguous_names(&self) {
        for (bare, qualified) in &self.port_names.ambiguous {
            eprintln!(
                "ВНИМАНИЕ: имя '{bare}' объявлено несколькими моделями ({}). \
                 По голому имени адресуется первая из них; для точного обращения \
                 используйте квалифицированное имя.",
                qualified.join(", ")
            );
        }
    }

    fn print_step(&self, step_no: usize) {
        let states = self.unit.active_states();
        let states_str = if states.is_empty() {
            "—".to_string()
        } else {
            states.join(", ")
        };

        // Двусмысленное имя (фича 0135) печатается КВАЛИФИЦИРОВАННЫМИ формами:
        // показывать `val=1`, пока вторая под-модель держит `val=2`, — значит
        // скрывать половину состояния модели.
        let display_names = |names: &[String]| -> Vec<String> {
            let mut out = Vec::new();
            for n in names {
                match self.port_names.ambiguous.iter().find(|(bare, _)| bare == n) {
                    Some((_, qualified)) => out.extend(qualified.iter().cloned()),
                    None => out.push(n.clone()),
                }
            }
            out
        };

        let fmt_group = |names: &[String]| -> String {
            display_names(names)
                .iter()
                .filter_map(|n| {
                    self.unit
                        .get_value(n)
                        .map(|v| format!("{}={}", n, format_value(&v)))
                })
                .collect::<Vec<_>>()
                .join("  ")
        };

        // Трасса печатает и такт, и модельное время (фича 0134): без времени
        // не прочесть, почему выдержка сработала именно здесь, а без такта —
        // не сверить с целью. Время показывается, только когда часы идут не по
        // умолчанию либо уже сдвинулись, — иначе оно засоряло бы вывод моделям,
        // время не использующим.
        if self.now_ns > 0 {
            print!(
                "Шаг {:3} ({:>8}):  [{}]",
                step_no,
                format_duration(self.now_ns),
                states_str
            );
        } else {
            print!("Шаг {:3}:  [{}]", step_no, states_str);
        }

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

    /// Применяет входы шага: позиционно (историческая форма) либо по именам.
    ///
    /// Возвращает ошибку, если сценарий назвал порт, которого нет, либо имя
    /// двусмысленно. Прежде функция ошибок не возвращала вовсе: лишний элемент
    /// массива молча отбрасывался (фича 0132).
    fn apply_step_inputs(&mut self, step: &SimStep, step_no: usize) -> Result<(), String> {
        for (values, direction) in [
            (&step.in_ports, PortDirectionKind::In),
            (&step.inout, PortDirectionKind::InOut),
        ] {
            let Some(values) = values else { continue };
            for (name, value) in self.resolve_values(values, direction, step_no)? {
                self.unit.set_port(&name, value);
            }
        }
        Ok(())
    }

    /// Переводит значения шага в пары «имя порта → значение».
    ///
    /// Общая воронка для входов и для `guard`: разойдясь, они принимали бы
    /// разные имена, и сценарий вёл бы себя по-разному в зависимости от того, в
    /// какой половине шага написано имя.
    /// Говорит один раз за прогон, что сценарий пользуется устаревшей формой.
    ///
    /// ⚠️ Это **не** `SIM-032`: тот о несовпадении **длины** массива с числом
    /// портов, а этот — о самой форме, даже когда длина верна. Слить их значило
    /// бы потерять различие «массив не той длины» и «форма устарела»; на входе с
    /// коротким массивом печатаются оба.
    fn warn_positional_form_once(&self) {
        if self.positional_form_warned.replace(true) {
            return;
        }
        // ⚠️ Код — отдельным литералом, а не внутри текста: гейт
        // `scripts/check-diagnostic-codes.sh` ищет коды именно строковыми
        // литералами `"XX-NNN"`, и код, вплавленный в сообщение, для него
        // невидим — то есть выпадает и из реестра диагностик. Соседи
        // `SIM-030`…`SIM-032` живут так с 0132 и в реестре отсутствуют
        // (вынесено кандидатом).
        const CODE: &str = "SIM-037";
        eprintln!(
            "Предупреждение [{CODE}]: сценарий задаёт значения портов позиционным массивом — \
             форма устарела. Индекс в массиве привязан к месту имени в АЛФАВИТНОМ списке портов \
             модели и её под-моделей, поэтому добавление или переименование порта сдвигает весь \
             массив, и шаг начинает описывать другое событие — молча. Пользуйтесь именами: \
             `\"in_ports\": {{\"имя_порта\": значение}}`; при тёзках из разных моделей имя \
             уточняется как `Модель::порт`."
        );
    }

    fn resolve_values(
        &self,
        values: &PortValues,
        direction: PortDirectionKind,
        step_no: usize,
    ) -> Result<Vec<(String, Value)>, String> {
        let names = self.names_of(direction);
        let mut resolved = Vec::new();
        match values {
            PortValues::Positional(list) => {
                self.warn_positional_form_once();
                if list.len() != names.len() {
                    // Предупреждение, а не ошибка: корпус мог опираться на
                    // неполные массивы, и ломать его фича не должна.
                    eprintln!(
                        "Предупреждение [SIM-032]: шаг {step_no}: {} значений в позиционном \
                         массиве `{}`, а портов {} — лишние игнорируются, недостающие не задаются",
                        list.len(),
                        direction.field(),
                        names.len()
                    );
                }
                for (i, json_val) in list.iter().enumerate() {
                    if let (Some(name), Some(value)) = (names.get(i), json_to_value(json_val)) {
                        let value = self.as_port_value(name, value);
                        resolved.push((name.clone(), value));
                    }
                }
            }
            PortValues::Named(map) => {
                for (name, json_val) in map {
                    self.check_port_name(name, direction, step_no)?;
                    if let Some(value) = json_to_value(json_val) {
                        resolved.push((name.clone(), self.as_port_value(name, value)));
                    }
                }
            }
        }
        Ok(resolved)
    }

    /// Приводит значение сценария к типу значения модели (фича 0183).
    ///
    /// Сегодня приведение одно: число на значении типа `duration` трактуется как
    /// **миллисекунды** — та же единица, что у `as duration`. Прочие значения
    /// проходят как есть: JSON и так даёт числа, логические и вещественные.
    ///
    /// ⚠️ Имя ищется и в квалифицированной форме (`Модель::имя`, фича 0135):
    /// реестр типов собран по голым именам, поэтому квалификатор снимается.
    fn as_port_value(&self, name: &str, value: crate::Value) -> crate::Value {
        let bare = name.rsplit("::").next().unwrap_or(name);
        match value {
            crate::Value::Number(millis) if self.port_names.durations.contains(bare) => {
                match i64::try_from(millis)
                    .ok()
                    .and_then(takt_lang::semantic::duration::from_millis)
                {
                    Some(ns) => crate::Value::Duration(ns),
                    // Переполнение наносекунд: оставляем число — ошибку даст
                    // вычисление, и она назовёт место, а молчаливой подмены нет.
                    None => crate::Value::Number(millis),
                }
            }
            other => other,
        }
    }

    /// Имена портов заданного направления.
    fn names_of(&self, direction: PortDirectionKind) -> &[String] {
        match direction {
            PortDirectionKind::In => &self.port_names.in_ports,
            PortDirectionKind::Out => &self.port_names.out_ports,
            PortDirectionKind::InOut => &self.port_names.inout_ports,
        }
    }

    /// Проверяет, что имя из сценария адресует ровно один порт нужного
    /// направления.
    ///
    /// ⚠️ Направление проверяется намеренно: `in_ports: {"lamp": 1}` при выходном
    /// `lamp` — почти наверняка опечатка, а не задумка. Прежде такая запись
    /// молча ничего не делала.
    fn check_port_name(
        &self,
        name: &str,
        direction: PortDirectionKind,
        step_no: usize,
    ) -> Result<(), String> {
        if name.contains("::") {
            // Квалифицированное имя: проверяем существование пары «модель::имя».
            // Направление здесь не сужается — квалификация уже однозначна.
            if !self.port_names.qualified.contains(name) {
                return Err(format!(
                    "Ошибка [SIM-030]: шаг {step_no}: порт `{name}` не найден в модели"
                ));
            }
            return Ok(());
        }
        if let Some((_, variants)) = self
            .port_names
            .ambiguous
            .iter()
            .find(|(bare, _)| bare == name)
        {
            return Err(format!(
                "Ошибка [SIM-031]: шаг {step_no}: имя `{name}` объявлено несколькими моделями \
                 ({}) — укажите квалифицированное имя",
                variants.join(", ")
            ));
        }
        if !self.names_of(direction).iter().any(|n| n == name) {
            return Err(format!(
                "Ошибка [SIM-030]: шаг {step_no}: порт `{name}` не найден среди портов \
                 направления `{}`",
                direction.field()
            ));
        }
        Ok(())
    }

    fn check_guard(&self, guard: &Guard, step_no: usize) -> Result<(), String> {
        // Порты guard разрешаются ТОЙ ЖЕ воронкой, что и входы шага: иначе
        // именованная форма работала бы в одной половине файла и не работала в
        // другой (фича 0132).
        for (values, direction) in [
            (&guard.out, PortDirectionKind::Out),
            (&guard.inout, PortDirectionKind::InOut),
        ] {
            let Some(values) = values else { continue };
            for (name, expected) in self.resolve_values(values, direction, step_no)? {
                let actual = self.unit.get_value(&name);
                if !values_match(&actual, &expected) {
                    return Err(format!(
                        "Guard шага {step_no}: {} ({name}): ожидалось {:?}, получено {:?}",
                        direction.field(),
                        expected,
                        actual
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

/// Человекочитаемая запись длительности: `999ms`, `1s`, `1s1ms`, `1m30s`.
///
/// Разряды переносятся, как в литерале языка: пока значение укладывается в
/// младшую единицу — печатается ею (`999ms`), при переполнении появляется
/// старшая (`1000ms` → `1s`), а остаток дописывается справа (`1001ms` →
/// `1s1ms`). Так запись в трассе читается тем же способом, каким автор её
/// **писал** в исходнике, и `90000ms` не приходится делить в голове.
///
/// Нулевые разряды опускаются (`3600s` → `1h`, а не `1h0m0s`); нулевая
/// длительность печатается младшей содержательной единицей — `0ms`.
pub fn format_duration(nanos: i64) -> String {
    const UNITS: [(i64, &str); 6] = [
        (3_600_000_000_000, "h"),
        (60_000_000_000, "m"),
        (1_000_000_000, "s"),
        (1_000_000, "ms"),
        (1_000, "us"),
        (1, "ns"),
    ];
    if nanos == 0 {
        return "0ms".to_string();
    }
    let sign = if nanos < 0 { "-" } else { "" };
    // Модуль берётся с защитой от i64::MIN: `abs()` на нём паникует.
    let mut rest = nanos.unsigned_abs();
    let mut out = String::new();
    for (size, name) in UNITS {
        let size = size.unsigned_abs();
        if rest >= size {
            out.push_str(&format!("{}{}", rest / size, name));
            rest %= size;
        }
        if rest == 0 {
            break;
        }
    }
    format!("{sign}{out}")
}

fn format_value(v: &Value) -> String {
    match v {
        Value::Number(n) => n.to_string(),
        Value::Real(f) => format!("{f:.4}"),
        Value::Boolean(b) => b.to_string(),
        // q(m, n): показываем вещественное значение repr·2⁻ⁿ.
        Value::Fixed { repr, n, .. } => format!("{:.4}", *repr as f64 / (1u64 << n) as f64),
        // Длительность печатается человекочитаемо: наносекунды в трассе
        // нечитаемы, а выдержки задаются секундами и миллисекундами.
        Value::Duration(ns) => crate::runner::format_duration(*ns),
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

/// Переводит секцию `extern` шага сценария в стенд эталона (фича 0209).
///
/// ⚠️ Значение, которое не переводится в величину симулятора (строка, объект в
/// позиции значения), — **ошибка сценария**, а не молчаливый пропуск: автор
/// написал подмену, и она обязана сработать.
fn extern_stubs_of(
    step: &crate::json_input::SimStep,
    step_no: usize,
) -> Result<crate::context::ExternStubs, String> {
    use crate::json_input::ExternValue;
    let mut stubs = crate::context::ExternStubs::default();
    let Some(declared) = &step.extern_stubs else {
        return Ok(stubs);
    };
    for (name, value) in declared {
        match value {
            ExternValue::Any(raw) => {
                let value = crate::json_input::json_to_value(raw).ok_or_else(|| {
                    format!("шаг {step_no}: значение extern-функции '{name}' не читается")
                })?;
                stubs.declare(name, crate::context::ExternStub::Any(value));
            }
            ExternValue::ByArgument(table) => {
                let mut by_arg = std::collections::HashMap::new();
                for (key, raw) in table {
                    let key: i128 = key.parse().map_err(|_| {
                        format!(
                            "шаг {step_no}: ключ '{key}' таблицы extern-функции '{name}' \
                             не число — таблица ищет по значению первого аргумента"
                        )
                    })?;
                    let value = crate::json_input::json_to_value(raw).ok_or_else(|| {
                        format!(
                            "шаг {step_no}: значение extern-функции '{name}' при аргументе \
                             {key} не читается"
                        )
                    })?;
                    by_arg.insert(key, value);
                }
                stubs.declare(name, crate::context::ExternStub::ByArgument(by_arg));
            }
        }
    }
    Ok(stubs)
}
