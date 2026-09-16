//! Ручной ввод значений портов посреди прогона.
//!
//! Значение, поставленное читателем, ложится перед следующим тактом **после** шага
//! сценария и потому перекрывает его на этом такте. Дальше действует обычное правило
//! эталона: значение порта удерживается, пока его не сменит шаг сценария либо новый
//! ручной ввод.
//!
//! Разбор идёт той же воронкой, что у шага сценария (`resolve_values`): имена,
//! квалифицированные имена, длительность в миллисекундах и отказы `SIM-030`/`SIM-031`
//! одни. Второй разбор входа разошёлся бы со сценарием молча - и запись ручного
//! прогона воспроизводилась бы не тем, что видел читатель. Разбор стоит при
//! постановке, а не на такте: отказ приходит сразу, пока читатель смотрит на поле.

use std::collections::BTreeMap;

use serde::Serialize;
use takt_lang::diagnostics::lang::keys;
use takt_lang::msg;

use super::SimulationRunner;
use crate::eval::value::Value;
use crate::json_input::PortValues;
use crate::port_names::PortDirectionKind;

/// Ручное значение, применённое на такте: имя, сторона и значение в форме сценария.
///
/// Форма сценария - а не внутреннее значение эталона: запись прогона кладёт его в
/// файл сценария как есть, и такой файл воспроизводит прогон без преобразования.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManualInput {
    pub name: String,
    /// `in` либо `inout` - поле шага сценария, куда значение ляжет при записи.
    pub field: &'static str,
    pub value: serde_json::Value,
}

/// Поставленное, но ещё не применённое значение.
#[derive(Debug, Clone)]
pub(super) struct Pending {
    input: ManualInput,
    value: Value,
}

impl SimulationRunner {
    /// Ставит значения входных и двунаправленных портов перед следующим тактом.
    ///
    /// Повторная постановка того же имени до такта заменяет значение. Отказ имени
    /// назван номером следующего такта - того, на котором значение легло бы.
    ///
    /// # Ошибки
    /// Имени нет либо оно двусмысленно - текст эталона с кодом `SIM-030`/`SIM-031`;
    /// значение не число, не логическое и не массив.
    pub fn set_manual_inputs(
        &mut self,
        in_ports: &BTreeMap<String, serde_json::Value>,
        inout: &BTreeMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        let next_tick = self.completed + 1;
        let mut staged = Vec::new();
        for (values, direction, field) in [
            (in_ports, PortDirectionKind::In, "in"),
            (inout, PortDirectionKind::InOut, "inout"),
        ] {
            let named = PortValues::Named(values.clone());
            let resolved = self.resolve_values(&named, direction, next_tick)?;
            if resolved.len() != values.len() {
                // `json_to_value` пропускает значение, которого не понимает; у
                // сценария это молчание, а у поля ввода - потерянный ввод.
                let (name, value) = values
                    .iter()
                    .find(|(name, _)| !resolved.iter().any(|(got, _)| got == *name))
                    .map(|(name, value)| (name.clone(), value.to_string()))
                    .unwrap_or_default();
                return Err(msg!(
                    keys::SIM_MANUAL_VALUE_UNREADABLE,
                    tick = next_tick,
                    name = name,
                    value = value
                ));
            }
            for (name, value) in resolved {
                let json = values[&name].clone();
                staged.push(Pending {
                    input: ManualInput {
                        name,
                        field,
                        value: json,
                    },
                    value,
                });
            }
        }
        for item in staged {
            self.manual_pending
                .retain(|held| held.input.name != item.input.name);
            self.manual_pending.push(item);
        }
        Ok(())
    }

    /// Применяет поставленные значения и отдаёт их для ответа такта.
    pub(super) fn apply_manual_inputs(&mut self) -> Vec<ManualInput> {
        let pending = std::mem::take(&mut self.manual_pending);
        pending
            .into_iter()
            .map(|item| {
                self.unit.set_port(&item.input.name, item.value);
                item.input
            })
            .collect()
    }
}
