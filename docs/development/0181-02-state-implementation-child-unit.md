# Задача 0181-02: Реализация состояния строится дочерним юнитом с общим контекстом

> Фича: [../features/0181-sim-state-implementation-tick.md](../features/0181-sim-state-implementation-tick.md) · ADR: [../adr/0181-sim-state-implementation-tick.md](../adr/0181-sim-state-implementation-tick.md) · анализ: [../analyze/0181-sim-state-implementation-tick.md](../analyze/0181-sim-state-implementation-tick.md)

## Что было

Два дефекта построения (Д1 и Д2 анализа):

- `build_impl` строил реализацию состояния **только** по ветви `single_compound`
  (модель из одного состояния-реализации **без** переходов и `next`). Иначе
  управление уходило в `build_node`, который поле `implements` состояния не
  читал вовсе, — композиция молча исчезала;
- `build_extend`, ветвь `Extend::Concatenation`, передавала `shared_parent`
  **как есть**. На корне он `None`, поэтому каждый шаг `+` строил свой
  экземпляр контекста корневой модели: `stage`, записанный шагом A, шагу B был
  не виден. Ветвь `Extend::Parallel` при `None` общий контекст **создавала** —
  два пути одной композиции разошлись.

## Что сделано

`takt-sim/src/unit/mod.rs`:

- в `UnitKind::Node` добавлено поле
  `state_impls: HashMap<String, Rc<RefCell<Unit>>>` — реализация по имени
  состояния. Изменение **аддитивно**: разбор `UnitKind::Node { .. }` с `..` по
  всему крейту продолжает компилироваться, правки потребовали лишь
  конструкторы-литералы в тестах, `state_io` и `viewport`.

`takt-sim/src/unit/builder.rs`:

- `build_node` для каждого `StateNode::Implement` строит
  `build_extend(implements, Some(ctx_rc.clone()))` — контекст **этого узла** как
  общий родитель. Пустая реализация (`UnitKind::None`) не кладётся: узел без
  композиции обязан остаться прежним;
- общий контекст ветвей вынесен в функцию `shared_context` и зовётся из **обеих**
  ветвей — `Concatenation` и `Parallel`. Одна функция намеренно: разъехавшись,
  они дали бы разную видимость переменных для двух форм композиции одного
  языка — ровно тот дефект, что фича и закрывает;
- ветвь `single_compound` в `build_impl` **сохранена** — горячий путь корпуса
  (`start Stacker = CR | MC | LC;`, `start Main = Cabin | Motor;`) не тронут, то
  есть регресс там невозможен по построению.

Обратная функциональность (правило 11): затронут только крейт `takt-sim`; цели
генерации не изменялись — **н/п**.

## Проверки

`cargo build --all-targets` — чисто (на этом шаге ожидаемо предупреждение
`field state_impls is never read`: потребитель появляется в 0181-03).
`cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings` — чисто.
Поведенческая проверка — в задаче 0181-03 (без такта поле само по себе ничего не
меняет).
