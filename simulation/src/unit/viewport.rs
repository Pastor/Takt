use svg::Document;
use svg::node::element::{Circle, Definitions, Group, Line, Marker, Path, Rectangle, Style, Text};

use crate::unit::Unit;
use grammar::diagnostics::Diagnostic;

// ── Данные легенды ────────────────────────────────────────────────────────────

/// Данные для отображения легенды портов и переменных на кадре симуляции.
pub(crate) struct LegendData {
    pub in_ports: Vec<(String, String)>,
    pub out_ports: Vec<(String, String)>,
    pub inout_ports: Vec<(String, String)>,
    pub vars: Vec<(String, String)>,
}

// ── Общий тип позиций ─────────────────────────────────────────────────────────

/// Вектор позиций узлов: каждая запись — координаты центра (x, y).
type Positions = Vec<(f64, f64)>;

// ── Конфигурация ─────────────────────────────────────────────────────────────

/// Параметры генерации Viewport: формат вывода и все SVG-константы.
///
/// Создаётся через `Configuration::default()`, после чего поля можно изменить.
/// Передаётся в [`create_viewport`] по значению.
pub struct Configuration {
    /// Целевой формат вывода.
    pub format: Format,
    /// Ширина холста в пикселях.
    pub width: f64,
    /// Высота холста в пикселях.
    pub height: f64,
    /// Радиус кружка состояния.
    pub radius: f64,
    /// Минимальное расстояние между центрами узлов (без радиуса).
    pub min_distance: f64,
    /// Размер маркера-стрелки (ширина и высота).
    pub arrow_size: f64,
    /// Базовый коэффициент искривления рёбер.
    pub curve_coefficient: f64,
    /// Размер шрифта подписи ребра в px.
    pub edge_label_font_size: f64,
    /// Ширина одного символа подписи ребра в px (оценка для кириллицы при 11 px).
    pub edge_label_char_width: f64,
    /// Зазор вокруг bounding-box подписи при проверке перекрытий.
    pub label_margin: f64,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            format: Format::SVG,
            width: 800.0,
            height: 600.0,
            radius: 25.0,
            min_distance: 25.0,
            arrow_size: 4.0,
            curve_coefficient: 0.10,
            edge_label_font_size: 11.0,
            edge_label_char_width: 9.0,
            label_margin: 3.0,
        }
    }
}

// ── Перечисления вывода ───────────────────────────────────────────────────────

/// Поддерживаемые форматы вывода.
pub(crate) enum Format {
    SVG,
}

/// Результат отрисовки: обёртка над конкретным документом.
pub(crate) enum Viewport {
    SVG(Document),
}

impl Viewport {
    /// Сохраняет Viewport в файл по указанному пути.
    pub fn save_to_file(&self, path: &str) -> Result<(), std::io::Error> {
        match self {
            Viewport::SVG(document) => Ok(svg::save(path, document)?),
        }
    }
}

// ── Публичный API ─────────────────────────────────────────────────────────────

/// Кэшированный результат вычисления раскладки графа.
///
/// Раскладка (positions) зависит только от структуры модели и не меняется
/// в ходе симуляции, поэтому её достаточно вычислить один раз.
pub(crate) struct CachedLayout {
    /// Полные имена состояний — используются для сопоставления с active_states.
    pub(crate) node_labels: Vec<String>,
    /// Краткие псевдонимы S1/S2/... — отображаются в кружках на графе.
    pub(crate) node_aliases: Vec<String>,
    /// Рёбра с полными именами предикатов.
    pub(crate) edges_vec: Vec<(usize, usize, String)>,
    pub(crate) positions: Positions,
}

/// Вычисляет раскладку графа для `unit`.
///
/// Дорогостоящий шаг (имитация отжига): вызывать один раз перед записью GIF,
/// затем передавать результат в [`render_from_layout`] для каждого кадра.
pub(crate) fn compute_layout(unit: &Unit, cfg: &Configuration) -> CachedLayout {
    let g = graph::unit_to_graph(unit);
    let (node_labels, edges_vec, positions) = graph::calculate_graph(g, cfg);
    let node_aliases: Vec<String> = (1..=node_labels.len()).map(|i| format!("S{i}")).collect();
    CachedLayout {
        node_labels,
        node_aliases,
        edges_vec,
        positions,
    }
}

/// Отрисовывает кадр из заранее вычисленной раскладки.
///
/// `highlighted_edge` — индексы рёбра `(from_idx, to_idx)`, которое нужно подсветить;
/// `None` означает обычный кадр без подсветки.
pub(crate) fn render_from_layout(
    layout: &CachedLayout,
    cfg: &Configuration,
    active_states: &[&str],
    legend: Option<&LegendData>,
    model_name: Option<&str>,
    highlighted_edge: Option<(usize, usize)>,
) -> Result<Viewport, Diagnostic> {
    match cfg.format {
        Format::SVG => {
            let document = create_svg(
                &layout.node_labels,
                &layout.node_aliases,
                &layout.edges_vec,
                &layout.positions,
                cfg,
                active_states,
                legend,
                model_name,
                highlighted_edge,
            );
            Ok(Viewport::SVG(document))
        }
    }
}

/// Создаёт [`Viewport`] из симуляционного [`Unit`].
///
/// `active_states` — срез имён состояний, которые нужно подсветить.
/// Каждый вызов пересчитывает раскладку. Для GIF-записи используйте
/// [`compute_layout`] + [`render_from_layout`].
pub(crate) fn create_viewport(
    unit: &Unit,
    configuration: Configuration,
    active_states: &[&str],
    legend: Option<&LegendData>,
) -> Result<Viewport, Diagnostic> {
    let layout = compute_layout(unit, &configuration);
    render_from_layout(&layout, &configuration, active_states, legend, None, None)
}

// ── Геометрические вспомогательные функции для SVG ───────────────────────────

/// Проверяет, пересекается ли прямоугольник AABB `(rx1, ry1)-(rx2, ry2)` с кругом
/// центром `(cx, cy)` и радиусом `r`.
///
/// Алгоритм: ближайшая точка прямоугольника к центру круга находится через зажим
/// (clamp), затем проверяется строгое неравенство `d² < r²`.
fn rect_overlaps_circle(rx1: f64, ry1: f64, rx2: f64, ry2: f64, cx: f64, cy: f64, r: f64) -> bool {
    let nx = cx.clamp(rx1, rx2);
    let ny = cy.clamp(ry1, ry2);
    (cx - nx).powi(2) + (cy - ny).powi(2) < r * r
}

/// Возвращает площадь пересечения двух выровненных прямоугольников AABB.
///
/// Прямоугольники задаются парами углов: `(ax1, ay1)-(ax2, ay2)` и `(bx1, by1)-(bx2, by2)`.
/// Если прямоугольники не перекрываются — возвращает `0.0`.
#[allow(clippy::too_many_arguments)]
fn rects_intersection_area(
    ax1: f64,
    ay1: f64,
    ax2: f64,
    ay2: f64,
    bx1: f64,
    by1: f64,
    bx2: f64,
    by2: f64,
) -> f64 {
    ((ax2.min(bx2) - ax1.max(bx1)).max(0.0)) * ((ay2.min(by2) - ay1.max(by1)).max(0.0))
}

// ── Генератор SVG ─────────────────────────────────────────────────────────────

/// Отрисовывает граф в SVG-документ.
///
/// Шаги отрисовки:
/// 1. Создаёт холст с параметрами из `cfg`.
/// 2. Добавляет стили шрифта ГОСТ и маркер-стрелку для рёбер.
/// 3. Рисует рёбра в виде кривых Безье со стрелками и подписями.
///    - Параллельные рёбра между одной парой узлов разводятся разным изгибом.
///    - Подписи рёбер размещаются жадным алгоритмом на концентрических кольцах
///      вокруг середины кривой Безье, минимизируя перекрытия с узлами и другими подписями.
/// 4. Рисует кружки узлов с подписями.
///
/// # Параметры
/// - `node_labels` — имена состояний в порядке индексации.
/// - `edges_vec` — рёбра `(индекс_источника, индекс_цели, подпись)`.
/// - `positions` — координаты центров узлов, соответствующие `node_labels`.
/// - `cfg` — конфигурация с размерами холста и параметрами отрисовки.
pub(crate) const LEGEND_WIDTH: f64 = 190.0;

fn create_svg(
    node_labels: &[String],
    node_aliases: &[String],
    edges_vec: &Vec<(usize, usize, String)>,
    positions: &Positions,
    cfg: &Configuration,
    active_states: &[&str],
    legend: Option<&LegendData>,
    model_name: Option<&str>,
    highlighted_edge: Option<(usize, usize)>,
) -> Document {
    let total_width = cfg.width + if legend.is_some() { LEGEND_WIDTH } else { 0.0 };
    let mut document = Document::new()
        .set("viewBox", (0, 0, total_width, cfg.height))
        .set("xmlns", "http://www.w3.org/2000/svg");

    document = document.add(Style::new(
        r#"
        .gost-text {
            font-family: "GOST 2.304-81 Type A", "GOST type A", "OpenGOST Type A", sans-serif;
            font-size: 14px;
            fill: black;
            text-anchor: middle;
            dominant-baseline: central;
        }
        .edge-label {
            font-family: "GOST 2.304-81 Type A", "GOST type A", "OpenGOST Type A", sans-serif;
            font-size: 11px;
            fill: #222;
            text-anchor: middle;
            dominant-baseline: central;
        }
        "#,
    ));

    document = document.add(
        Definitions::new()
            .add(
                Marker::new()
                    .set("id", "arrow")
                    .set("viewBox", "0 0 10 10")
                    .set("refX", "10")
                    .set("refY", "5")
                    .set("markerWidth", cfg.arrow_size)
                    .set("markerHeight", cfg.arrow_size)
                    .set("orient", "auto")
                    .add(
                        Path::new()
                            .set("d", "M 0 0 L 10 5 L 0 10 z")
                            .set("fill", "black"),
                    ),
            )
            .add(
                Marker::new()
                    .set("id", "arrow-hl")
                    .set("viewBox", "0 0 10 10")
                    .set("refX", "10")
                    .set("refY", "5")
                    .set("markerWidth", cfg.arrow_size)
                    .set("markerHeight", cfg.arrow_size)
                    .set("orient", "auto")
                    .add(
                        Path::new()
                            .set("d", "M 0 0 L 10 5 L 0 10 z")
                            .set("fill", "#FF8C00"),
                    ),
            ),
    );

    // Центры узлов сохраняются для проверки перекрытий подписей рёбер
    let node_circles: Vec<(f64, f64)> = positions.clone();
    let mut placed_boxes: Vec<(f64, f64, f64, f64)> = Vec::new();

    // Подсчёт кратности: сколько раз встречается пара (min(i,j), max(i,j)).
    // Кратные рёбра между одной парой узлов получают разный изгиб.
    let mut connection_counts: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();
    let mut edge_multiplicities = Vec::with_capacity(edges_vec.len());
    for &(i, j, _) in edges_vec.iter() {
        let key = if i < j { (i, j) } else { (j, i) };
        let count = connection_counts.entry(key).or_insert(0);
        edge_multiplicities.push(*count);
        *count += 1;
    }

    // ── Отрисовка рёбер ───────────────────────────────────────────────────────
    for (idx, (i, j, edge_label)) in edges_vec.iter().enumerate() {
        let is_highlighted = highlighted_edge.map_or(false, |(hi, hj)| *i == hi && *j == hj);
        let multiplicity = edge_multiplicities[idx];
        let (x1, y1) = positions[*i];
        let (x2, y2) = positions[*j];

        let dx = x2 - x1;
        let dy = y2 - y1;
        let d = (dx * dx + dy * dy).sqrt();
        if d < 1e-6 {
            continue; // вырожденное ребро — узлы в одной точке
        }
        let ux = dx / d; // единичный вектор по направлению ребра
        let uy = dy / d;

        // Смещение точек входа/выхода от центров кружков: разные источники в одном узле
        // не сливаются в одну точку благодаря синусоидальному сдвигу по индексу.
        let entry_offset = (*i as f64 * 0.5).sin() * 5.0;
        let exit_offset = (*j as f64 * 0.5).sin() * 5.0;

        let start_x = x1 + ux * cfg.radius + uy * entry_offset;
        let start_y = y1 + uy * cfg.radius - ux * entry_offset;
        let end_x = x2 - ux * cfg.radius + uy * exit_offset;
        let end_y = y2 - uy * cfg.radius - ux * exit_offset;

        // Контрольная точка квадратичной кривой Безье — перпендикуляр к середине ребра.
        // Коэффициент изгиба: базовый + добавка за кратность + малый шум по индексу узла.
        let mid_x = (start_x + end_x) / 2.0;
        let mid_y = (start_y + end_y) / 2.0;
        let curve_factor =
            cfg.curve_coefficient + (multiplicity as f64) * 0.10 + (*i as f64 * 0.01) % 0.04;
        let cp_x = mid_x - uy * d * curve_factor;
        let cp_y = mid_y + ux * d * curve_factor;

        let (edge_stroke, edge_width, arrow_marker) = if is_highlighted {
            ("#FF8C00", 3.5_f64, "url(#arrow-hl)")
        } else {
            ("#444", 2.0_f64, "url(#arrow)")
        };
        document = document.add(
            Path::new()
                .set(
                    "d",
                    format!(
                        "M {} {} Q {} {} {} {}",
                        start_x, start_y, cp_x, cp_y, end_x, end_y
                    ),
                )
                .set("fill", "none")
                .set("stroke", edge_stroke)
                .set("stroke-width", edge_width)
                .set("marker-end", arrow_marker),
        );

        // ── Подпись ребра: только для подсвеченного перехода ──────────────────
        if is_highlighted {
            let bez_mid_x = 0.25 * start_x + 0.5 * cp_x + 0.25 * end_x;
            let bez_mid_y = 0.25 * start_y + 0.5 * cp_y + 0.25 * end_y;

            let box_w = edge_label.chars().count() as f64 * cfg.edge_label_char_width + 8.0;
            let box_h = cfg.edge_label_font_size + 4.0;

            let search_radii: &[f64] = &[0.0, 15.0, 30.0, 50.0, 70.0, 90.0, 115.0, 145.0, 180.0];
            const N_ANGLES: usize = 16;

            let mut best_center = (bez_mid_x, bez_mid_y);
            let mut best_score = f64::MAX;

            'search: for &r in search_radii {
                let n = if r < 1.0 { 1 } else { N_ANGLES };
                for k in 0..n {
                    let angle = (k as f64) * std::f64::consts::TAU / (n as f64);
                    let cx = bez_mid_x + r * angle.cos();
                    let cy = bez_mid_y + r * angle.sin();

                    let bx1 = cx - box_w / 2.0 - cfg.label_margin;
                    let by1 = cy - box_h / 2.0 - cfg.label_margin;
                    let bx2 = cx + box_w / 2.0 + cfg.label_margin;
                    let by2 = cy + box_h / 2.0 + cfg.label_margin;

                    let mut score = r;
                    if bx1 < 0.0 || bx2 > cfg.width || by1 < 0.0 || by2 > cfg.height {
                        score += 1e8;
                    }
                    for &(nx, ny) in &node_circles {
                        if rect_overlaps_circle(
                            bx1, by1, bx2, by2, nx, ny, cfg.radius + cfg.label_margin,
                        ) {
                            score += 1e5;
                        }
                    }
                    for &(lx1, ly1, lx2, ly2) in &placed_boxes {
                        let area =
                            rects_intersection_area(bx1, by1, bx2, by2, lx1, ly1, lx2, ly2);
                        if area > 0.0 {
                            score += 500.0 * area;
                        }
                    }
                    if score < best_score {
                        best_score = score;
                        best_center = (cx, cy);
                    }
                    if score < 1.0 {
                        break 'search;
                    }
                }
            }

            let (label_x, label_y) = best_center;
            placed_boxes.push((
                label_x - box_w / 2.0,
                label_y - box_h / 2.0,
                label_x + box_w / 2.0,
                label_y + box_h / 2.0,
            ));

            let mut grp = Group::new();
            let lead_dist =
                ((label_x - bez_mid_x).powi(2) + (label_y - bez_mid_y).powi(2)).sqrt();
            if lead_dist > 5.0 {
                grp = grp.add(
                    Line::new()
                        .set("x1", bez_mid_x)
                        .set("y1", bez_mid_y)
                        .set("x2", label_x)
                        .set("y2", label_y)
                        .set("stroke", "#FF8C00")
                        .set("stroke-width", 1.0)
                        .set("stroke-dasharray", "3,2"),
                );
            }
            let bg = Rectangle::new()
                .set("x", label_x - box_w / 2.0)
                .set("y", label_y - box_h / 2.0)
                .set("width", box_w)
                .set("height", box_h)
                .set("fill", "#FFF3CD")
                .set("stroke", "#FF8C00")
                .set("stroke-width", 1.5)
                .set("rx", 2.0)
                .set("ry", 2.0);
            let lbl = Text::new(edge_label.clone())
                .set("x", label_x)
                .set("y", label_y)
                .set("class", "edge-label");
            document = document.add(grp.add(bg).add(lbl));
        }
    }

    // ── Отрисовка узлов ───────────────────────────────────────────────────────
    for (i, full_label) in node_labels.iter().enumerate() {
        let (cx, cy) = positions[i];
        let is_active = active_states.contains(&full_label.as_str());
        let fill = if is_active { "#FFE066" } else { "#cce5ff" };
        let stroke_w = if is_active { 3 } else { 2 };
        let alias = node_aliases.get(i).map(String::as_str).unwrap_or(full_label.as_str());
        document = document.add(
            Circle::new()
                .set("cx", cx)
                .set("cy", cy)
                .set("r", cfg.radius)
                .set("fill", fill)
                .set("stroke", "#333")
                .set("stroke-width", stroke_w),
        );
        document = document.add(
            Text::new(alias)
                .set("x", cx)
                .set("y", cy)
                .set("class", "gost-text"),
        );
    }

    // ── Имя модели ────────────────────────────────────────────────────────────
    if let Some(name) = model_name {
        document = document.add(
            Text::new(name)
                .set("x", cfg.width / 2.0)
                .set("y", 16.0)
                .set("font-family", "monospace")
                .set("font-size", "14px")
                .set("font-weight", "bold")
                .set("fill", "#333")
                .set("text-anchor", "middle")
                .set("dominant-baseline", "middle"),
        );
    }

    // ── Легенда портов, переменных и цветов ───────────────────────────────────
    if let Some(leg) = legend {
        let lx = cfg.width + 5.0;
        let line_h = 15.0;
        let pad = 5.0;

        let mut all_lines: Vec<(&str, &[(String, String)])> = vec![];
        if !leg.in_ports.is_empty() {
            all_lines.push(("IN:", &leg.in_ports));
        }
        if !leg.out_ports.is_empty() {
            all_lines.push(("OUT:", &leg.out_ports));
        }
        if !leg.inout_ports.is_empty() {
            all_lines.push(("INOUT:", &leg.inout_ports));
        }
        if !leg.vars.is_empty() {
            all_lines.push(("VARS:", &leg.vars));
        }

        // СОСТОЯНИЯ: + N псевдонимов; ЦВЕТА: + 2 записи
        let n_lines: usize = all_lines.iter().map(|(_, v)| v.len() + 1).sum::<usize>()
            + 3  // ЦВЕТА: + 2 строки цвета
            + 1 + node_labels.len(); // СОСТОЯНИЯ: + N строк
        let box_h = n_lines as f64 * line_h + 2.0 * pad;

        document = document.add(
            Rectangle::new()
                .set("x", lx)
                .set("y", pad)
                .set("width", LEGEND_WIDTH - 10.0)
                .set("height", box_h)
                .set("fill", "#f8f8f8")
                .set("stroke", "#bbb")
                .set("stroke-width", 1)
                .set("rx", 4)
                .set("ry", 4),
        );

        let mut y = pad + line_h;

        // ── Псевдонимы состояний ──────────────────────────────────────────────
        document = document.add(
            Text::new("СОСТОЯНИЯ:")
                .set("x", lx + pad)
                .set("y", y)
                .set("font-family", "monospace")
                .set("font-size", "10px")
                .set("font-weight", "bold")
                .set("fill", "#555")
                .set("dominant-baseline", "middle"),
        );
        y += line_h;
        for (alias, full) in node_aliases.iter().zip(node_labels.iter()) {
            let text = format!("  {alias} = {full}");
            document = document.add(
                Text::new(text)
                    .set("x", lx + pad)
                    .set("y", y)
                    .set("font-family", "monospace")
                    .set("font-size", "10px")
                    .set("fill", "#333")
                    .set("dominant-baseline", "middle"),
            );
            y += line_h;
        }

        // ── Цветовые обозначения ──────────────────────────────────────────────
        document = document.add(
            Text::new("ЦВЕТА:")
                .set("x", lx + pad)
                .set("y", y)
                .set("font-family", "monospace")
                .set("font-size", "10px")
                .set("font-weight", "bold")
                .set("fill", "#555")
                .set("dominant-baseline", "middle"),
        );
        y += line_h;

        for (color, label) in [("#FFE066", "активное"), ("#cce5ff", "неактивное")]
        {
            document = document.add(
                Circle::new()
                    .set("cx", lx + pad + 5.0)
                    .set("cy", y)
                    .set("r", 4.0)
                    .set("fill", color)
                    .set("stroke", "#555")
                    .set("stroke-width", 1),
            );
            document = document.add(
                Text::new(label)
                    .set("x", lx + pad + 13.0)
                    .set("y", y)
                    .set("font-family", "monospace")
                    .set("font-size", "10px")
                    .set("fill", "#333")
                    .set("dominant-baseline", "middle"),
            );
            y += line_h;
        }

        // ── Порты и переменные ────────────────────────────────────────────────
        for (header, entries) in &all_lines {
            document = document.add(
                Text::new(*header)
                    .set("x", lx + pad)
                    .set("y", y)
                    .set("font-family", "monospace")
                    .set("font-size", "10px")
                    .set("font-weight", "bold")
                    .set("fill", "#555")
                    .set("dominant-baseline", "middle"),
            );
            y += line_h;
            for (name, value) in *entries {
                let text = format!("  {}={}", name, value);
                document = document.add(
                    Text::new(text)
                        .set("x", lx + pad)
                        .set("y", y)
                        .set("font-family", "monospace")
                        .set("font-size", "10px")
                        .set("fill", "#333")
                        .set("dominant-baseline", "middle"),
                );
                y += line_h;
            }
        }
    }

    document
}

// ── Модуль: граф из Unit и оптимизация раскладки ─────────────────────────────

/// Функции построения ориентированного графа из [`Unit`] и размещения узлов.
///
/// Внутренний модуль скрывает детали алгоритма имитации отжига и обхода дерева Unit.
/// Снаружи доступны только [`unit_to_graph`] и [`calculate_graph`].
pub(super) mod graph {
    use petgraph::graph::{Graph, NodeIndex};
    use petgraph::visit::EdgeRef;
    use rand::RngExt;
    use std::collections::HashMap;

    use super::{Configuration, Positions};
    use crate::unit::Unit;

    // ── Преобразование Unit → Graph ───────────────────────────────────────────

    /// Строит ориентированный граф petgraph из дерева [`Unit`].
    ///
    /// - `Unit::None` порождает пустой граф.
    /// - `Unit::Node`: каждый ключ `state_transitions` становится узлом; каждый
    ///   переход `(from, to, _pred)` — ориентированным ребром.
    /// - `Unit::Parallel` и `Unit::Sequential`: узлы и рёбра дочерних Unit
    ///   добавляются в общий граф рекурсивно.
    ///
    /// Метки узлов — имена состояний (`String`); метки рёбер — пустые строки
    /// (предикаты не имеют строкового представления в модели симуляции).
    pub(super) fn unit_to_graph(unit: &Unit) -> Graph<String, String> {
        let mut graph = Graph::new();
        populate_graph(unit, &mut graph);
        graph
    }

    /// Рекурсивно добавляет состояния и переходы из `unit` в `graph`.
    ///
    /// Вызывается из [`unit_to_graph`]. Для `Unit::Node` сначала создаются все узлы,
    /// затем рёбра — чтобы переходы к ещё не добавленным состояниям не терялись.
    /// Если целевое состояние перехода отсутствует в `state_transitions`, ребро пропускается.
    ///
    /// Метка ребра — `String` из кортежа `Predicate = Rc<(String, dyn Fn)>`,
    /// то есть имя предиката (`pred.0`).
    fn populate_graph(unit: &Unit, graph: &mut Graph<String, String>) {
        match unit {
            Unit::None => {}
            Unit::Node {
                state_transitions, ..
            } => {
                let mut node_map: HashMap<String, NodeIndex> = HashMap::new();
                for name in state_transitions.keys() {
                    let idx = graph.add_node(name.clone());
                    node_map.insert(name.clone(), idx);
                }
                for (from, transitions) in state_transitions {
                    for (to, pred) in transitions {
                        if let (Some(&fi), Some(&ti)) = (node_map.get(from), node_map.get(to)) {
                            graph.add_edge(fi, ti, pred.name.clone());
                        }
                    }
                }
            }
            Unit::Parallel { units, .. } | Unit::Sequential { units, .. } => {
                for u in units {
                    populate_graph(&u.borrow(), graph);
                }
            }
        }
    }

    // ── Вычисление позиций ────────────────────────────────────────────────────

    /// Извлекает узлы и рёбра из графа, генерирует начальное случайное размещение
    /// и оптимизирует его методом имитации отжига.
    ///
    /// Возвращает:
    /// - `Vec<String>` — метки узлов в порядке индексации (0..N).
    /// - `Vec<(usize, usize, String)>` — рёбра как `(источник, цель, метка)`.
    /// - [`Positions`] — оптимизированные координаты центров узлов.
    ///
    /// Позиции гарантированно находятся в пределах `[cfg.radius, cfg.width - cfg.radius]`
    /// × `[cfg.radius, cfg.height - cfg.radius]`.
    pub(super) fn calculate_graph(
        graph: Graph<String, String>,
        cfg: &Configuration,
    ) -> (Vec<String>, Vec<(usize, usize, String)>, Positions) {
        let nodes: Vec<(NodeIndex, String)> = graph
            .node_indices()
            .map(|idx| (idx, graph[idx].clone()))
            .collect();

        let node_labels: Vec<String> = nodes.iter().map(|(_, label)| label.clone()).collect();

        let mut index_map: HashMap<NodeIndex, usize> = HashMap::new();
        for (i, (node_idx, _)) in nodes.iter().enumerate() {
            index_map.insert(*node_idx, i);
        }

        let mut edges_vec: Vec<(usize, usize, String)> = Vec::new();
        for edge in graph.edge_references() {
            let u = index_map[&edge.source()];
            let v = index_map[&edge.target()];
            edges_vec.push((u, v, edge.weight().clone()));
        }

        let n = node_labels.len();
        let mut rng = rand::rng();
        let mut positions: Positions = (0..n)
            .map(|_| {
                (
                    rng.random_range(cfg.radius..cfg.width - cfg.radius),
                    rng.random_range(cfg.radius..cfg.height - cfg.radius),
                )
            })
            .collect();

        let edges_for_layout: Vec<(usize, usize)> =
            edges_vec.iter().map(|&(u, v, _)| (u, v)).collect();
        optimize_layout(
            &mut positions,
            &edges_for_layout,
            cfg.radius,
            cfg.min_distance,
            cfg.width,
            cfg.height,
        );

        (node_labels, edges_vec, positions)
    }

    // ── Оптимизация размещения ────────────────────────────────────────────────

    /// Оптимизирует позиции узлов методом имитации отжига (simulated annealing).
    ///
    /// На каждом шаге случайно выбирается узел и смещается на гауссовский вектор
    /// с дисперсией, пропорциональной текущей температуре. Новое состояние принимается,
    /// если оно снижает энергию или с вероятностью `exp(-ΔE / T)`.
    ///
    /// Функция энергии [`delta_energy`] штрафует за:
    /// 1. Перекрытие кружков узлов.
    /// 2. Большую суммарную длину рёбер.
    /// 3. Пересечения рёбер.
    ///
    /// Для пустого графа функция завершается немедленно.
    fn optimize_layout(
        positions: &mut Positions,
        edges: &[(usize, usize)],
        radius: f64,
        min_distance: f64,
        width: f64,
        height: f64,
    ) {
        let n = positions.len();
        if n == 0 {
            return;
        }

        // Предвычисляем список рёбер, инцидентных каждому узлу, чтобы не
        // перебирать все рёбра при каждом вычислении delta_energy.
        let mut incident: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (ei, &(u, v)) in edges.iter().enumerate() {
            incident[u].push(ei);
            incident[v].push(ei);
        }

        let mut rng = rand::rng();
        let w_overlap = 1e6;
        let w_length = 1.0;
        let w_cross = 1.0;
        let cross_penalty = 500.0; // штраф за одно пересечение рёбер

        let t_start: f64 = 500.0;
        let t_end = 0.1;
        let alpha = 0.995; // коэффициент охлаждения
        let iterations_per_t = 100 * n;
        let mut t = t_start;

        while t > t_end {
            for _ in 0..iterations_per_t {
                let idx = rng.random_range(0..n);
                let old_pos = positions[idx];

                let sigma = 15.0 * (t / t_start).max(0.01);
                let dx: f64 = rng.sample::<f64, _>(rand_distr::StandardNormal) * sigma;
                let dy: f64 = rng.sample::<f64, _>(rand_distr::StandardNormal) * sigma;
                let new_pos =
                    clamp_to_canvas(old_pos.0 + dx, old_pos.1 + dy, radius, width, height);

                if (new_pos.0 - old_pos.0).abs() < 1e-6 && (new_pos.1 - old_pos.1).abs() < 1e-6 {
                    continue; // смещение подавлено зажимом — пропускаем
                }

                // Инкрементальная дельта-энергия: пересчитываем только вклад
                // перемещённого узла — O(n) вместо O(n²) полного пересчёта.
                let delta = delta_energy(
                    positions,
                    idx,
                    old_pos,
                    new_pos,
                    edges,
                    &incident,
                    radius,
                    min_distance,
                    w_overlap,
                    w_length,
                    w_cross,
                    cross_penalty,
                );

                if delta < 0.0 || rng.random::<f64>() < (-delta / t).exp() {
                    positions[idx] = new_pos;
                }
            }
            t *= alpha;
        }
    }

    /// Вычисляет изменение энергии при перемещении узла `idx` из `old_pos` в `new_pos`.
    ///
    /// Работает за O(n + degree × m) вместо O(n² + m²) полного пересчёта:
    /// перебираются только пары, в которых участвует перемещённый узел.
    #[allow(clippy::too_many_arguments)]
    fn delta_energy(
        positions: &Positions,
        idx: usize,
        old_pos: (f64, f64),
        new_pos: (f64, f64),
        edges: &[(usize, usize)],
        incident: &[Vec<usize>],
        radius: f64,
        min_distance: f64,
        w_overlap: f64,
        w_length: f64,
        w_cross: f64,
        cross_penalty: f64,
    ) -> f64 {
        let n = positions.len();
        let min_d = 2.0 * radius + min_distance;
        let mut delta = 0.0;

        // Перекрытие: пары (idx, j) для всех j ≠ idx
        for j in 0..n {
            if j == idx {
                continue;
            }
            let pj = positions[j];
            let old_d = dist(old_pos, pj);
            if old_d < min_d {
                delta -= w_overlap * (min_d - old_d).powi(2);
            }
            let new_d = dist(new_pos, pj);
            if new_d < min_d {
                delta += w_overlap * (min_d - new_d).powi(2);
            }
        }

        // Длина рёбер: только рёбра, инцидентные idx
        for &ei in &incident[idx] {
            let (u, v) = edges[ei];
            let other = if u == idx { positions[v] } else { positions[u] };
            delta -= w_length * dist(old_pos, other).powi(2);
            delta += w_length * dist(new_pos, other).powi(2);
        }

        // Пересечения: рёбра, инцидентные idx, против остальных рёбер
        for &ei in &incident[idx] {
            let (u1, v1) = edges[ei];
            let other_node = if u1 == idx { v1 } else { u1 };
            let p_other = positions[other_node];

            for (ej, &(u2, v2)) in edges.iter().enumerate() {
                if ej == ei {
                    continue;
                }
                // Пропускаем рёбра, разделяющие вершину с ei
                if u2 == u1 || u2 == v1 || v2 == u1 || v2 == v1 {
                    continue;
                }
                let q1 = positions[u2];
                let q2 = positions[v2];
                if segments_intersect(old_pos, p_other, q1, q2) {
                    delta -= w_cross * cross_penalty;
                }
                if segments_intersect(new_pos, p_other, q1, q2) {
                    delta += w_cross * cross_penalty;
                }
            }
        }

        delta
    }

    /// Зажимает координаты точки так, чтобы кружок радиуса `radius` не выходил за холст.
    ///
    /// Допустимая область для центра: `[radius, width - radius] × [radius, height - radius]`.
    fn clamp_to_canvas(x: f64, y: f64, radius: f64, width: f64, height: f64) -> (f64, f64) {
        (
            x.clamp(radius, width - radius),
            y.clamp(radius, height - radius),
        )
    }

    /// Возвращает евклидово расстояние между двумя точками на плоскости.
    fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
        ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
    }

    /// Проверяет, пересекаются ли два отрезка `p1-p2` и `p3-p4` в общей точке (не на конце).
    ///
    /// Алгоритм ориентированных площадей: отрезки пересекаются тогда и только тогда,
    /// когда конечные точки каждого из них лежат по разные стороны от другого отрезка.
    /// Коллинеарные случаи (пересечение на вершине) не обнаруживаются — это приемлемо
    /// для задачи минимизации пересечений рёбер в визуализации графа.
    fn segments_intersect(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), p4: (f64, f64)) -> bool {
        fn orient(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
            (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
        }
        let o1 = orient(p1, p2, p3);
        let o2 = orient(p1, p2, p4);
        let o3 = orient(p3, p4, p1);
        let o4 = orient(p3, p4, p2);
        o1 * o2 < 0.0 && o3 * o4 < 0.0
    }

    // ── Тесты модуля graph ────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;
        use petgraph::graph::Graph;
        use std::collections::HashMap;

        // ── dist ──────────────────────────────────────────────────────────────

        #[test]
        fn test_dist_pythagorean_triple() {
            let d = dist((0.0, 0.0), (3.0, 4.0));
            assert!((d - 5.0).abs() < 1e-10, "ожидалось 5.0, получено {d}");
        }

        #[test]
        fn test_dist_same_point_is_zero() {
            assert!(dist((2.5, -1.0), (2.5, -1.0)).abs() < 1e-10);
        }

        #[test]
        fn test_dist_horizontal() {
            let d = dist((0.0, 0.0), (7.0, 0.0));
            assert!((d - 7.0).abs() < 1e-10);
        }

        // ── segments_intersect ────────────────────────────────────────────────

        #[test]
        fn test_segments_cross_at_center() {
            // Крест: (-1,0)-(1,0) и (0,-1)-(0,1)
            assert!(segments_intersect(
                (-1.0, 0.0),
                (1.0, 0.0),
                (0.0, -1.0),
                (0.0, 1.0),
            ));
        }

        #[test]
        fn test_segments_diagonal_cross() {
            // X-образное: (0,0)-(1,1) и (0,1)-(1,0)
            assert!(segments_intersect(
                (0.0, 0.0),
                (1.0, 1.0),
                (0.0, 1.0),
                (1.0, 0.0),
            ));
        }

        #[test]
        fn test_segments_parallel_no_cross() {
            assert!(!segments_intersect(
                (0.0, 0.0),
                (1.0, 0.0),
                (0.0, 1.0),
                (1.0, 1.0),
            ));
        }

        #[test]
        fn test_segments_t_shape_no_cross() {
            // T-образное: (1,0) лежит на первом отрезке, но коллинеарность не детектируется
            assert!(!segments_intersect(
                (0.0, 0.0),
                (2.0, 0.0),
                (1.0, 0.0),
                (1.0, 1.0),
            ));
        }

        #[test]
        fn test_segments_no_cross_far_apart() {
            assert!(!segments_intersect(
                (0.0, 0.0),
                (1.0, 0.0),
                (5.0, 5.0),
                (6.0, 6.0),
            ));
        }

        // ── clamp_to_canvas ───────────────────────────────────────────────────

        #[test]
        fn test_clamp_point_within_bounds_unchanged() {
            let (x, y) = clamp_to_canvas(400.0, 300.0, 25.0, 800.0, 600.0);
            assert_eq!(x, 400.0);
            assert_eq!(y, 300.0);
        }

        #[test]
        fn test_clamp_left_top_corner() {
            // Точка (0, 0) зажимается до (radius, radius) = (25, 25)
            let (x, y) = clamp_to_canvas(0.0, 0.0, 25.0, 800.0, 600.0);
            assert_eq!(x, 25.0);
            assert_eq!(y, 25.0);
        }

        #[test]
        fn test_clamp_right_bottom_corner() {
            // Точка (900, 700) зажимается до (775, 575)
            let (x, y) = clamp_to_canvas(900.0, 700.0, 25.0, 800.0, 600.0);
            assert_eq!(x, 775.0);
            assert_eq!(y, 575.0);
        }

        // ── unit_to_graph ─────────────────────────────────────────────────────

        #[test]
        fn test_unit_to_graph_none_is_empty() {
            let g = unit_to_graph(&Unit::None);
            assert_eq!(g.node_count(), 0);
            assert_eq!(g.edge_count(), 0);
        }

        #[test]
        fn test_unit_to_graph_node_states_become_nodes() {
            let mut transitions = HashMap::new();
            transitions.insert("A".to_string(), vec![]);
            transitions.insert("B".to_string(), vec![]);
            let unit = make_node(transitions);
            let g = unit_to_graph(&unit);
            assert_eq!(g.node_count(), 2);
            assert_eq!(g.edge_count(), 0);
        }

        #[test]
        fn test_unit_to_graph_transitions_become_edges() {
            let pred =
                crate::unit::Predicate::new("is_ready", |_: &dyn crate::context::Context| true);
            let mut transitions = HashMap::new();
            transitions.insert("A".to_string(), vec![("B".to_string(), pred)]);
            transitions.insert("B".to_string(), vec![]);
            let unit = make_node(transitions);
            let g = unit_to_graph(&unit);
            assert_eq!(g.node_count(), 2);
            assert_eq!(g.edge_count(), 1);
            let label = g.edge_references().next().unwrap().weight();
            assert_eq!(
                label, "is_ready",
                "метка ребра должна совпадать с именем предиката"
            );
        }

        #[test]
        fn test_unit_to_graph_parallel_merges_children() {
            use std::cell::RefCell;
            use std::rc::Rc;
            let child1 = {
                let mut t = HashMap::new();
                t.insert("A".to_string(), vec![]);
                make_node(t)
            };
            let child2 = {
                let mut t = HashMap::new();
                t.insert("B".to_string(), vec![]);
                make_node(t)
            };
            let unit = Unit::Parallel {
                units: vec![Rc::new(RefCell::new(child1)), Rc::new(RefCell::new(child2))],
                executions: HashMap::new(),
            };
            let g = unit_to_graph(&unit);
            assert_eq!(g.node_count(), 2);
        }

        #[test]
        fn test_unit_to_graph_sequential_merges_children() {
            use std::cell::RefCell;
            use std::rc::Rc;
            let child1 = {
                let mut t = HashMap::new();
                t.insert("X".to_string(), vec![]);
                make_node(t)
            };
            let child2 = {
                let mut t = HashMap::new();
                t.insert("Y".to_string(), vec![]);
                t.insert("Z".to_string(), vec![]);
                make_node(t)
            };
            let unit = Unit::Sequential {
                units: vec![Rc::new(RefCell::new(child1)), Rc::new(RefCell::new(child2))],
                index: 0,
                executions: HashMap::new(),
            };
            let g = unit_to_graph(&unit);
            assert_eq!(g.node_count(), 3);
        }

        // ── calculate_graph ───────────────────────────────────────────────────

        #[test]
        fn test_calculate_graph_empty_returns_empty() {
            let cfg = crate::unit::viewport::Configuration::default();
            let g: Graph<String, String> = Graph::new();
            let (labels, edges, positions) = calculate_graph(g, &cfg);
            assert!(labels.is_empty());
            assert!(edges.is_empty());
            assert!(positions.is_empty());
        }

        #[test]
        fn test_calculate_graph_counts_match() {
            let cfg = crate::unit::viewport::Configuration::default();
            let mut g: Graph<String, String> = Graph::new();
            let a = g.add_node("A".to_string());
            let b = g.add_node("B".to_string());
            g.add_edge(a, b, String::new());
            let (labels, edges, positions) = calculate_graph(g, &cfg);
            assert_eq!(labels.len(), 2);
            assert_eq!(edges.len(), 1);
            assert_eq!(positions.len(), 2);
        }

        #[test]
        fn test_calculate_graph_positions_within_bounds() {
            let cfg = crate::unit::viewport::Configuration::default();
            let mut g: Graph<String, String> = Graph::new();
            for name in ["A", "B", "C", "D"] {
                g.add_node(name.to_string());
            }
            let (_, _, positions) = calculate_graph(g, &cfg);
            for &(x, y) in &positions {
                assert!(
                    x >= cfg.radius && x <= cfg.width - cfg.radius,
                    "x={x} вне холста"
                );
                assert!(
                    y >= cfg.radius && y <= cfg.height - cfg.radius,
                    "y={y} вне холста"
                );
            }
        }

        #[test]
        fn test_calculate_graph_labels_contain_all_nodes() {
            let cfg = crate::unit::viewport::Configuration::default();
            let mut g: Graph<String, String> = Graph::new();
            g.add_node("Alpha".to_string());
            g.add_node("Beta".to_string());
            let (labels, _, _) = calculate_graph(g, &cfg);
            assert!(labels.contains(&"Alpha".to_string()));
            assert!(labels.contains(&"Beta".to_string()));
        }

        // ── Вспомогательные конструкторы ──────────────────────────────────────

        fn make_node(
            state_transitions: HashMap<String, Vec<(String, crate::unit::Predicate)>>,
        ) -> Unit {
            Unit::Node {
                context: None,
                variables: HashMap::new(),
                executions: HashMap::new(),
                state: None,
                state_transitions,
                state_executions: HashMap::new(),
                last_transition: None,
            }
        }
    }
}

// ── Тесты viewport ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::Unit;

    // ── rect_overlaps_circle ──────────────────────────────────────────────────

    #[test]
    fn test_rect_circle_overlap_center_inside() {
        // Круг (5,5) r=3 полностью внутри прямоугольника (0,0)-(10,10)
        assert!(rect_overlaps_circle(0.0, 0.0, 10.0, 10.0, 5.0, 5.0, 3.0));
    }

    #[test]
    fn test_rect_circle_no_overlap_far_away() {
        assert!(!rect_overlaps_circle(0.0, 0.0, 2.0, 2.0, 10.0, 10.0, 1.0));
    }

    #[test]
    fn test_rect_circle_touching_edge_not_overlap() {
        // Ближайшая точка (5,2.5), расстояние = 2.0, r = 2.0 → строгое неравенство не выполняется
        assert!(!rect_overlaps_circle(0.0, 0.0, 5.0, 5.0, 7.0, 2.5, 2.0));
    }

    #[test]
    fn test_rect_circle_partially_inside() {
        // Круг центром на краю прямоугольника: (10, 5) r=2, ближайшая точка (10,5), d=0 < r
        assert!(rect_overlaps_circle(0.0, 0.0, 10.0, 10.0, 10.0, 5.0, 2.0));
    }

    // ── rects_intersection_area ───────────────────────────────────────────────

    #[test]
    fn test_rects_area_overlapping() {
        // (0,0,4,4) ∩ (2,2,6,6) = (2,2,4,4) → площадь 4
        let area = rects_intersection_area(0.0, 0.0, 4.0, 4.0, 2.0, 2.0, 6.0, 6.0);
        assert!((area - 4.0).abs() < 1e-10, "ожидалось 4.0, получено {area}");
    }

    #[test]
    fn test_rects_area_no_overlap() {
        let area = rects_intersection_area(0.0, 0.0, 1.0, 1.0, 5.0, 5.0, 6.0, 6.0);
        assert_eq!(area, 0.0);
    }

    #[test]
    fn test_rects_area_touching_edge_is_zero() {
        // Касание по вертикальному ребру: (0,0,2,2) и (2,0,4,2)
        let area = rects_intersection_area(0.0, 0.0, 2.0, 2.0, 2.0, 0.0, 4.0, 2.0);
        assert_eq!(area, 0.0);
    }

    #[test]
    fn test_rects_area_one_inside_other() {
        // (1,1,3,3) полностью внутри (0,0,4,4) → площадь = 4
        let area = rects_intersection_area(0.0, 0.0, 4.0, 4.0, 1.0, 1.0, 3.0, 3.0);
        assert!((area - 4.0).abs() < 1e-10);
    }

    // ── create_viewport ───────────────────────────────────────────────────────

    #[test]
    fn test_create_viewport_empty_unit_returns_ok() {
        let result = create_viewport(&Unit::None, Configuration::default(), &[], None);
        assert!(result.is_ok(), "ожидался Ok, получено Err");
    }

    #[test]
    fn test_create_viewport_node_unit_returns_ok() {
        use std::collections::HashMap;
        let pred = crate::unit::Predicate::new("cond", |_: &dyn crate::context::Context| true);
        let mut transitions = HashMap::new();
        transitions.insert("A".to_string(), vec![("B".to_string(), pred)]);
        transitions.insert("B".to_string(), vec![]);
        let unit = Unit::Node {
            context: None,
            variables: HashMap::new(),
            executions: HashMap::new(),
            state: Some("A".to_string()),
            state_transitions: transitions,
            state_executions: HashMap::new(),
            last_transition: None,
        };
        let result = create_viewport(&unit, Configuration::default(), &["A"], None);
        assert!(result.is_ok());
    }
}
