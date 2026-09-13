//! Применимость флага сборки к цели - один носитель.

use crate::diagnostics::lang::{Key, keys};
use crate::msg;

/// Флаг, применимый не ко всякой цели.
struct Restricted {
    /// Как флаг записан автором - для текста отказа.
    flag: &'static str,
    /// Что флаг делает - вторая половина текста отказа, ключом каталога: текст строится
    /// на языке прогона в момент отказа.
    what: Key,
    /// Цели, которые флаг понимают.
    targets: &'static [&'static str],
}

/// Таблица ограниченных флагов.
const RESTRICTED: &[Restricted] = &[Restricted {
    flag: "--bus=apb",
    what: keys::CLI_COMPILE_BUS_ADAPTER,
    targets: &["sv-mmio"],
}];

/// Проверяет применимость поднятых флагов к цели.
///
/// `raised` - какие из ограниченных флагов заданы (по имени из таблицы). Возвращает
/// готовый текст отказа либо `Ok(())`.
pub fn check(target: &str, raised: &[&str]) -> Result<(), String> {
    for flag in raised {
        let Some(entry) = RESTRICTED.iter().find(|e| e.flag == *flag) else {
            continue;
        };
        if entry.targets.contains(&target) {
            continue;
        }
        return Err(msg!(
            keys::CLI_COMPILE_FLAG_NOT_FOR_TARGET,
            flag = entry.flag,
            target = target,
            what = msg!(entry.what),
            targets = entry.targets.join(", ")
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn bus_is_refused_for_every_target_but_sv_mmio() {
        for target in ["c", "c-hal", "rust", "st", "st-at", "sv"] {
            let err = check(target, &["--bus=apb"]).unwrap_err();
            assert!(err.contains("sv-mmio"), "цель {target}, текст: {err}");
        }
        assert!(check("sv-mmio", &["--bus=apb"]).is_ok());
    }

    #[test]
    fn unrestricted_flag_passes_anywhere() {
        // Табличную форму автомата печатают все цели - флаг ни одной не отвергается.
        assert!(check("c", &["--fsm=table"]).is_ok());
        assert!(check("sv", &[]).is_ok());
    }
}
