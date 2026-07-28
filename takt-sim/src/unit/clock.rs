//! Модельные часы узла симуляции (фича 0134, задачи 0134-03/05).
//!
//! Вынесено из `unit/mod.rs`: файл вместе с этим кодом давал 1048 строк при
//! лимите 1000, а часы — самостоятельная тема. Дочерний модуль по отношению к
//! `unit`, поэтому приватные поля `UnitKind::Node` ему видны.
//!
//! Здесь **нет** часов реального мира: время ставит `runner` (`set_time_ns`), а
//! счётчик тактов растёт в конце такта. Иначе трасса перестала бы
//! воспроизводиться, и все потактовые сверки стали бы мигающими.

use super::{Unit, UnitKind};

impl Unit {
    /// Ставит модельное время (наносекунды) во **все** узлы дерева (фича 0134).
    ///
    /// Рекурсивно, как `set_value`: ветви композиции живут в одном времени —
    /// иначе выдержка в одной ветви шла бы по своим часам, и трасса перестала
    /// бы быть воспроизводимой.
    pub fn set_time_ns(&mut self, now_ns: i64) {
        match &mut self.0 {
            UnitKind::Node { time_ns, .. } => *time_ns = now_ns,
            UnitKind::Parallel { units, .. } | UnitKind::Sequential { units, .. } => {
                for unit in units {
                    unit.borrow_mut().set_time_ns(now_ns);
                }
            }
            UnitKind::None => {}
        }
    }

    /// Сколько модельного времени прошло с входа в текущее состояние.
    pub(crate) fn since_state_entry_ns(&self) -> i64 {
        match &self.0 {
            UnitKind::Node {
                time_ns,
                state_entered_ns,
                ..
            } => time_ns.saturating_sub(*state_entered_ns),
            UnitKind::Parallel { units, .. } | UnitKind::Sequential { units, .. } => units
                .first()
                .map_or(0, |u| u.borrow().since_state_entry_ns()),
            UnitKind::None => 0,
        }
    }

    /// Тактов с входа в текущее состояние (фича 0134).
    pub(crate) fn ticks_in_state(&self) -> u64 {
        match &self.0 {
            UnitKind::Node { ticks_in_state, .. } => *ticks_in_state,
            UnitKind::Parallel { units, .. } | UnitKind::Sequential { units, .. } => {
                units.first().map_or(0, |u| u.borrow().ticks_in_state())
            }
            UnitKind::None => 0,
        }
    }

    /// Отмечает вход в состояние текущим модельным временем (фича 0134).
    pub(super) fn mark_state_entry(&mut self) {
        if let UnitKind::Node {
            time_ns,
            state_entered_ns,
            ticks_in_state,
            ..
        } = &mut self.0
        {
            *state_entered_ns = *time_ns;
            // Такт входа — нулевой: на нём с момента входа не прошло ни одного
            // такта (как и модельного времени).
            *ticks_in_state = 0;
        }
    }

    /// Увеличивает счётчик тактов, проведённых в состоянии (фича 0134).
    ///
    /// Зовётся в **конце** такта: значение, видимое условиям на такте M, равно
    /// числу тактов с входа — ровно как счётчик `takt_dwell` порождённого C.
    pub(super) fn advance_state_ticks(&mut self) {
        match &mut self.0 {
            UnitKind::Node { ticks_in_state, .. } => {
                *ticks_in_state = ticks_in_state.saturating_add(1);
            }
            UnitKind::Parallel { units, .. } | UnitKind::Sequential { units, .. } => {
                for unit in units {
                    unit.borrow_mut().advance_state_ticks();
                }
            }
            UnitKind::None => {}
        }
    }
}
