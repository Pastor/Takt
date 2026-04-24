use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use rand::RngExt;

use std::collections::HashMap;
use svg::Document;
use svg::node::element::{Circle, Definitions, Group, Line, Marker, Path, Rectangle, Style, Text};

// ------------------------------------------------------------
// Параметры холста и отрисовки
// ------------------------------------------------------------
const WIDTH: f64 = 800.0;
const HEIGHT: f64 = 600.0;
const RADIUS: f64 = 25.0; // радиус всех кружков
const MIN_DISTANCE: f64 = RADIUS; // минимальное расстояние между узлами
const ARROW_SIZE: f64 = 4.0; // размер стрелки рёбер
const CURVE_COEFFICIENT: f64 = 0.10; // коэффициент искривления рёбер
const EDGE_LABEL_FONT_SIZE: f64 = 11.0; // px, размер шрифта подписи ребра
const EDGE_LABEL_CHAR_WIDTH: f64 = 9.0; // px/символ при 11px (кириллица)
const LABEL_MARGIN: f64 = 3.0;          // зазор вокруг подписи ребра

// ------------------------------------------------------------
// Структура для хранения позиций узлов во время оптимизации
// ------------------------------------------------------------
type Positions = Vec<(f64, f64)>;

/// Эвклидово расстояние между двумя точками
fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Проверка пересечения двух отрезков (p1-p2) и (p3-p4)
fn segments_intersect(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), p4: (f64, f64)) -> bool {
    fn orient(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    }
    let o1 = orient(p1, p2, p3);
    let o2 = orient(p1, p2, p4);
    let o3 = orient(p3, p4, p1);
    let o4 = orient(p3, p4, p2);

    // Общий случай: отрезки пересекаются, если ориентации разные
    if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
        return true;
    }
    // Дополнительно можно обработать коллинеарность, но для наших целей это не критично
    false
}

/// Пересечение прямоугольника AABB и круга
fn rect_overlaps_circle(rx1: f64, ry1: f64, rx2: f64, ry2: f64, cx: f64, cy: f64, r: f64) -> bool {
    let nx = cx.clamp(rx1, rx2);
    let ny = cy.clamp(ry1, ry2);
    (cx - nx).powi(2) + (cy - ny).powi(2) < r * r
}

/// Площадь пересечения двух AABB
fn rects_intersection_area(
    ax1: f64, ay1: f64, ax2: f64, ay2: f64,
    bx1: f64, by1: f64, bx2: f64, by2: f64,
) -> f64 {
    ((ax2.min(bx2) - ax1.max(bx1)).max(0.0)) * ((ay2.min(by2) - ay1.max(by1)).max(0.0))
}

/// Полная энергия конфигурации
fn energy(
    positions: &Positions,
    edges: &[(usize, usize)], // пары индексов узлов
    radius: f64,
    min_distance: f64,
    w_overlap: f64,
    w_length: f64,
    w_cross: f64,
    cross_penalty: f64,
) -> f64 {
    let n = positions.len();
    let mut e = 0.0;
    let min_d = 2.0 * radius + min_distance;

    // 1. Штраф за перекрытия кружков
    for i in 0..n {
        for j in (i + 1)..n {
            let d = dist(positions[i], positions[j]);
            if d < min_d {
                e += w_overlap * (min_d - d).powi(2);
            }
        }
    }

    // 2. Длина рёбер (притяжение)
    for &(i, j) in edges {
        let d = dist(positions[i], positions[j]);
        e += w_length * d.powi(2);
    }

    // 3. Пересечения рёбер
    let m = edges.len();
    // Для каждого ребра сохраняем его индексы и координаты концов
    let mut edge_segments = Vec::with_capacity(m);
    for &(u, v) in edges {
        edge_segments.push((positions[u], positions[v]));
    }
    for i in 0..m {
        for j in (i + 1)..m {
            // Пропускаем рёбра, имеющие общую вершину
            let (u1, v1) = edges[i];
            let (u2, v2) = edges[j];
            if u1 == u2 || u1 == v2 || v1 == u2 || v1 == v2 {
                continue;
            }
            if segments_intersect(
                edge_segments[i].0,
                edge_segments[i].1,
                edge_segments[j].0,
                edge_segments[j].1,
            ) {
                e += w_cross * cross_penalty;
            }
        }
    }
    e
}

/// Ограничение координат, чтобы узлы не выходили за холст
fn clamp_to_canvas(x: f64, y: f64, radius: f64, width: f64, height: f64) -> (f64, f64) {
    let margin = radius;
    let x = x.clamp(margin, width - margin);
    let y = y.clamp(margin, height - margin);
    (x, y)
}

fn optimize_layout(
    positions: &mut Positions,
    edges: &[(usize, usize)],
    radius: f64,
    min_distance: f64,
    width: f64,
    height: f64,
) {
    let mut rng = rand::rng();
    let n = positions.len();

    let w_overlap = 1e6;
    let w_length = 1.0;
    let w_cross = 1.0;
    let cross_penalty = 500.0; // одно пересечение ~ хуже, чем большое суммарное расстояние

    let mut current_energy = energy(
        positions,
        edges,
        radius,
        min_distance,
        w_overlap,
        w_length,
        w_cross,
        cross_penalty,
    );

    let t_start = 500.0;
    let t_end = 0.1;
    let alpha = 0.995;
    let iterations_per_t = 100 * n;

    let mut t = t_start;

    while t > t_end {
        for _ in 0..iterations_per_t {
            // Выбираем случайный узел
            let idx = rng.random_range(0..n);
            let old_x = positions[idx].0;
            let old_y = positions[idx].1;

            // Генерируем смещение, масштабированное с температурой
            let sigma = 15.0 * (t / t_start as f64).max(0.01);
            let dx: f64 = rng.sample::<f64, _>(rand_distr::StandardNormal) * sigma;
            let dy: f64 = rng.sample::<f64, _>(rand_distr::StandardNormal) * sigma;
            let new_x = old_x + dx;
            let new_y = old_y + dy;

            let (new_x, new_y) = clamp_to_canvas(new_x, new_y, radius, width, height);
            if (new_x - old_x).abs() < 1e-6 && (new_y - old_y).abs() < 1e-6 {
                continue; // нет движения
            }

            // Временно применяем новое положение
            positions[idx] = (new_x, new_y);
            let new_energy = energy(
                positions,
                edges,
                radius,
                min_distance,
                w_overlap,
                w_length,
                w_cross,
                cross_penalty,
            );
            let delta = new_energy - current_energy;

            // Критерий принятия
            if delta < 0.0 || rng.random::<f64>() < (-delta / t).exp() {
                // Принимаем новое состояние
                current_energy = new_energy;
            } else {
                // Возвращаем старое
                positions[idx] = (old_x, old_y);
            }
        }
        t *= alpha;
    }
}

fn main() {
    // ------------------------------------------------------------
    // Создаём граф с помощью petgraph
    // ------------------------------------------------------------
    let mut graph = Graph::<&str, &str>::new();

    let a = graph.add_node("А");
    let b = graph.add_node("Б");
    let c = graph.add_node("В");
    let d = graph.add_node("Г");
    let e = graph.add_node("Д");
    let f = graph.add_node("Е");

    graph.add_edge(a, b, "связь1");
    graph.add_edge(b, c, "связь2");
    graph.add_edge(c, d, "связь3");
    graph.add_edge(d, e, "связь4");
    graph.add_edge(e, f, "связь5");
    graph.add_edge(f, a, "связь6");
    graph.add_edge(a, c, "связь7");
    graph.add_edge(b, e, "связь8");
    graph.add_edge(c, f, "связь9");

    // Извлекаем список рёбер в виде индексов (порядок узлов: 0,1,2,...)
    let nodes: Vec<(NodeIndex, &str)> = graph.node_indices().map(|idx| (idx, graph[idx])).collect();
    let node_labels: Vec<&str> = nodes.iter().map(|&(_, label)| label).collect();

    // Строим соответствие: старый NodeIndex -> порядковый номер 0..N
    let mut index_map = std::collections::HashMap::new();
    for (i, &(node_idx, _)) in nodes.iter().enumerate() {
        index_map.insert(node_idx, i);
    }

    // Список рёбер как тройки (индекс_источника, индекс_цели, подпись)
    let mut edges_vec: Vec<(usize, usize, &str)> = Vec::new();
    for edge in graph.edge_references() {
        let u = index_map[&edge.source()];
        let v = index_map[&edge.target()];
        edges_vec.push((u, v, *edge.weight()));
    }

    let n = node_labels.len();

    // ------------------------------------------------------------
    // Начальное размещение – случайное, но равномерное внутри холста
    // ------------------------------------------------------------
    let mut rng = rand::rng();
    let mut positions: Positions = (0..n)
        .map(|_| {
            (
                rng.random_range(RADIUS..WIDTH - RADIUS),
                rng.random_range(RADIUS..HEIGHT - RADIUS),
            )
        })
        .collect();

    // ------------------------------------------------------------
    // Запуск оптимизации
    // ------------------------------------------------------------
    let edges_for_layout: Vec<(usize, usize)> =
        edges_vec.iter().map(|&(u, v, _)| (u, v)).collect();
    optimize_layout(
        &mut positions,
        &edges_for_layout,
        RADIUS,
        MIN_DISTANCE,
        WIDTH,
        HEIGHT,
    );

    // ------------------------------------------------------------
    // Построение SVG
    // ------------------------------------------------------------
    let mut document = Document::new()
        .set("viewBox", (0, 0, WIDTH, HEIGHT))
        .set("xmlns", "http://www.w3.org/2000/svg");

    // Стиль для шрифта ГОСТ
    let style = Style::new(
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
    );
    document = document.add(style);

    // Определение маркера-стрелки
    let marker = Marker::new()
        .set("id", "arrow")
        .set("viewBox", "0 0 10 10")
        .set("refX", "10")
        .set("refY", "5")
        .set("markerWidth", ARROW_SIZE)
        .set("markerHeight", ARROW_SIZE)
        .set("orient", "auto")
        .add(
            Path::new()
                .set("d", "M 0 0 L 10 5 L 0 10 z")
                .set("fill", "black"),
        );
    document = document.add(Definitions::new().add(marker));

    // Центры узлов и список bounding-box'ов размещённых подписей — для проверки перекрытий
    let node_circles: Vec<(f64, f64)> = positions.clone();
    let mut placed_boxes: Vec<(f64, f64, f64, f64)> = Vec::new();

    // Рисуем рёбра как изогнутые стрелки
    // 1. Подсчет кратности ребер для разведения параллельных связей
    let mut connection_counts = HashMap::new();
    let mut edge_multiplicities = Vec::with_capacity(edges_vec.len());
    for &(i, j, _) in &edges_vec {
        let key = if i < j { (i, j) } else { (j, i) };
        let count = connection_counts.entry(key).or_insert(0);
        edge_multiplicities.push(*count);
        *count += 1;
    }

    // 2. Отрисовка
    for (idx, &(i, j, edge_label)) in edges_vec.iter().enumerate() {
        let multiplicity = edge_multiplicities[idx];
        let (x1, y1) = positions[i];
        let (x2, y2) = positions[j];

        // Вычисляем точки на границах кружков (чтобы стрелка упиралась в край)
        let dx = x2 - x1;
        let dy = y2 - y1;
        let d = (dx * dx + dy * dy).sqrt();
        if d < 1e-6 {
            continue;
        }
        let ux = dx / d;
        let uy = dy / d;

        // Небольшое смещение точек входа/выхода в зависимости от начального/конечного узла,
        // чтобы рёбра из разных источников не сливались в одну точку.
        let entry_offset = (i as f64 * 0.5).sin() * 5.0;
        let exit_offset = (j as f64 * 0.5).sin() * 5.0;

        let start_x = x1 + ux * RADIUS + uy * entry_offset;
        let start_y = y1 + uy * RADIUS - ux * entry_offset;
        let end_x = x2 - ux * RADIUS + uy * exit_offset;
        let end_y = y2 - uy * RADIUS - ux * exit_offset;

        // Контрольная точка для изгиба (перпендикуляр к середине)
        let mid_x = (start_x + end_x) / 2.0;
        let mid_y = (start_y + end_y) / 2.0;

        // Индивидуальный коэффициент изгиба:
        // Базовый + добавка за кратность + небольшая вариативность по индексу узла.
        let curve_factor =
            CURVE_COEFFICIENT + (multiplicity as f64) * 0.10 + (i as f64 * 0.01) % 0.04;

        let offset = d * curve_factor;
        let cp_x = mid_x - uy * offset;
        let cp_y = mid_y + ux * offset;

        let path_data = format!(
            "M {} {} Q {} {} {} {}",
            start_x, start_y, cp_x, cp_y, end_x, end_y
        );

        let path = Path::new()
            .set("d", path_data)
            .set("fill", "none")
            .set("stroke", "#444")
            .set("stroke-width", 2.0)
            .set("marker-end", "url(#arrow)");
        document = document.add(path);

        // ── Подпись ребра ───────────────────────────────────────────
        // Середина кривой Безье при t=0.5: B(0.5) = 0.25·P0 + 0.5·P1 + 0.25·P2
        let bez_mid_x = 0.25 * start_x + 0.5 * cp_x + 0.25 * end_x;
        let bez_mid_y = 0.25 * start_y + 0.5 * cp_y + 0.25 * end_y;

        let box_w = edge_label.chars().count() as f64 * EDGE_LABEL_CHAR_WIDTH + 8.0;
        let box_h = EDGE_LABEL_FONT_SIZE + 4.0;

        // Перебираем кандидатные позиции на концентрических кольцах вокруг bez_mid.
        // Для каждой считаем штраф: пересечение с узлами и уже размещёнными подписями.
        // Выбираем кандидата с минимальным штрафом (ближний без пересечений — идеально).
        let search_radii: &[f64] = &[0.0, 15.0, 30.0, 50.0, 70.0, 90.0, 115.0, 145.0, 180.0];
        let n_angles: usize = 16; // шаг 22.5°

        let mut best_center = (bez_mid_x, bez_mid_y);
        let mut best_score = f64::MAX;

        'search: for &r in search_radii {
            let n = if r < 1.0 { 1 } else { n_angles };
            for k in 0..n {
                let angle = (k as f64) * std::f64::consts::TAU / (n as f64);
                let cx = bez_mid_x + r * angle.cos();
                let cy = bez_mid_y + r * angle.sin();

                // Расширенный bounding box для проверок (учитывает зазор)
                let x1 = cx - box_w / 2.0 - LABEL_MARGIN;
                let y1 = cy - box_h / 2.0 - LABEL_MARGIN;
                let x2 = cx + box_w / 2.0 + LABEL_MARGIN;
                let y2 = cy + box_h / 2.0 + LABEL_MARGIN;

                let mut score = r; // предпочесть ближние позиции

                // Штраф за выход за границы холста
                if x1 < 0.0 || x2 > WIDTH || y1 < 0.0 || y2 > HEIGHT {
                    score += 1e8;
                }

                // Штраф за перекрытие с кружками узлов
                for &(nx, ny) in &node_circles {
                    if rect_overlaps_circle(x1, y1, x2, y2, nx, ny, RADIUS + LABEL_MARGIN) {
                        score += 1e5;
                    }
                }

                // Штраф за перекрытие с уже размещёнными подписями
                for &(lx1, ly1, lx2, ly2) in &placed_boxes {
                    let area = rects_intersection_area(x1, y1, x2, y2, lx1, ly1, lx2, ly2);
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

        // Лидерная линия — только если подпись сдвинута от середины ребра
        let lead_dist =
            ((label_x - bez_mid_x).powi(2) + (label_y - bez_mid_y).powi(2)).sqrt();
        let mut grp = Group::new();
        if lead_dist > 5.0 {
            let leader = Line::new()
                .set("x1", bez_mid_x)
                .set("y1", bez_mid_y)
                .set("x2", label_x)
                .set("y2", label_y)
                .set("stroke", "#888")
                .set("stroke-width", 1.0)
                .set("stroke-dasharray", "3,2");
            grp = grp.add(leader);
        }
        let bg = Rectangle::new()
            .set("x", label_x - box_w / 2.0)
            .set("y", label_y - box_h / 2.0)
            .set("width", box_w)
            .set("height", box_h)
            .set("fill", "white")
            .set("stroke", "#aaa")
            .set("stroke-width", 1.0)
            .set("rx", 2.0)
            .set("ry", 2.0);
        let lbl = Text::new(edge_label.to_string())
            .set("x", label_x)
            .set("y", label_y)
            .set("class", "edge-label");
        document = document.add(grp.add(bg).add(lbl));
        // ── Конец подписи ребра ─────────────────────────────────────
    }

    // Рисуем кружки и подписи
    for (i, label) in node_labels.iter().enumerate() {
        let (cx, cy) = positions[i];

        let circle = Circle::new()
            .set("cx", cx)
            .set("cy", cy)
            .set("r", RADIUS)
            .set("fill", "#cce5ff")
            .set("stroke", "#333")
            .set("stroke-width", 2);
        document = document.add(circle);

        // Подпись внутри кружка шрифтом ГОСТ
        let text = Text::new(label.to_string())
            .set("x", cx)
            .set("y", cy)
            .set("class", "gost-text");
        document = document.add(text);
    }

    // Сохраняем
    svg::save("graph.svg", &document).expect("Не удалось сохранить SVG");
    println!("Граф оптимизирован и сохранён в graph.svg");
}
