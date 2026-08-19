# Задача 0278-01: Снятие мёртвой упаковки последовательной композиции

> Фича: [../features/0278-compact-implement-dead-branch.md](../features/0278-compact-implement-dead-branch.md) · ADR: [../adr/0278-compact-implement-dead-branch.md](../adr/0278-compact-implement-dead-branch.md) · анализ: [../analyze/0278-compact-implement-dead-branch.md](../analyze/0278-compact-implement-dead-branch.md)

## Что было

`semantic/extend.rs::compact_implement` (55 строк) упаковывал плоскую
`Extend::Concatenation` в синтетическую модель `<State>_Sequence` со ступенями
`Step0 = M1 { next Step1 }`, `Step1 = M2`. Док-комментарий обещал: «Вызывается
сразу после `unroll_extend_expression` в стадии stage1».

Вызова не было с **7 апреля 2026** (коммит `9401f269` закомментировал его при
постороннем рефакторинге, фича 0199 удалила закомментированную строку). Путь при
этом не забыт, а **отвергнут**: ADR 0057 рассматривал его как Option B и назвал
дефект — безусловный `next` между ступенями продвигался бы каждый такт.

## Что сделано

- удалена функция `compact_implement`;
- снята ссылка на неё из документации `Extend::Model` (после удаления
  `Location::Codegen` у этого варианта не строит никто, кроме тестов);
- переписаны три комментария, объяснявших мир через «отключён в `tree.rs`»:
  в юнит-тесте `extend.rs`, в двух тестах `semantic_tests.rs` и в примечании
  `validate/implemented.rs`. Теперь они называют **правило** — реализация в
  дереве плоская — и ссылаются на решение ADR 0057, а не на несуществующий код;
- удалён юнит-тест `test_extend_predicates` вместе со снятыми предикатами
  (задача 0278-02).

**Функциональность:** `takt-lang`; `takt-sim` и цели не затронуты — изымается
код, который не исполнялся.

## Проверки

```sh
cargo build --bin taktc
cargo test --all-features        # 3291 тест, 0 провалов
```

`grep -rn "compact_implement\|X_Sequence"` по обоим крейтам — пусто.
Поведение `A + B` проверено сверками sim ≡ C ≡ SV и гейтом синтеза
`batch_cycle` в составе `precheck.sh`.
