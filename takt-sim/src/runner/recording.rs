//! Сценарий выполненного такта с учётом приоритета ручных входов.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{ManualInput, SimulationRunner};
use crate::json_input::{PortValues, SimStep, json_to_value};
use crate::port_names::PortDirectionKind;

impl SimulationRunner {
    /// Возвращает сценарий последнего выполненного такта. Вызывать после шага с
    /// трассой: у ответа о завершении без трассы нового такта нет.
    pub fn recorded_step(&self, manual: &[ManualInput]) -> SimStep {
        let mut step = self
            .completed
            .checked_sub(1)
            .and_then(|index| self.sim_steps.get(index))
            .cloned()
            .unwrap_or_else(|| SimStep {
                // За последним шагом стенд остаётся установленным. Явный пустой
                // шаг сбросил бы его, поэтому повторяем последнюю установку.
                extern_stubs: self
                    .sim_steps
                    .last()
                    .and_then(|step| step.extern_stubs.clone()),
                ..SimStep::default()
            });
        if manual.is_empty() {
            return step;
        }

        // Разрешаем алиасы в порядке применения: входы сценария, двунаправленные
        // значения, затем ручные. В карту попадает последнее значение каждого порта.
        let mut inputs = BTreeMap::new();
        let mut inout = BTreeMap::new();
        for (values, direction, target) in [
            (&step.in_ports, PortDirectionKind::In, &mut inputs),
            (&step.inout, PortDirectionKind::InOut, &mut inout),
        ] {
            match values {
                Some(PortValues::Positional(values)) => {
                    for (name, value) in self.names_of(direction).iter().zip(values) {
                        self.record_input(target, name, value);
                    }
                }
                Some(PortValues::Named(values)) => {
                    for (name, value) in values {
                        self.record_input(target, name, value);
                    }
                }
                None => {}
            }
        }
        for input in manual {
            let mut value = BTreeMap::new();
            self.record_input(&mut value, &input.name, &input.value);
            for (name, value) in value {
                inputs.remove(&name);
                inout.remove(&name);
                let target = if input.field == "inout" {
                    &mut inout
                } else {
                    &mut inputs
                };
                target.insert(name, value);
            }
        }
        step.in_ports = self.recorded_ports(inputs, PortDirectionKind::In);
        step.inout = self.recorded_ports(inout, PortDirectionKind::InOut);
        step
    }

    /// Подставляет новые шаги с текущего такта. Пройденный префикс оставляет
    /// прежним, включая последнюю установку стенда за концом сценария.
    pub fn replace_unplayed_steps(&mut self, steps: Vec<SimStep>) {
        self.sim_steps.truncate(self.completed);
        while self.sim_steps.len() < self.completed.min(steps.len()) {
            let extern_stubs = self
                .sim_steps
                .last()
                .and_then(|step| step.extern_stubs.clone());
            self.sim_steps.push(SimStep {
                extern_stubs,
                ..SimStep::default()
            });
        }
        self.sim_steps
            .extend(steps.into_iter().skip(self.completed));
    }

    /// Старый позиционный ввод по голому двусмысленному имени расходится по
    /// ветвям. Делаем это явным, чтобы один ручной порт мог перекрыть свою ветвь.
    fn record_input(&self, target: &mut BTreeMap<String, Value>, name: &str, value: &Value) {
        if json_to_value(value).is_none() {
            return;
        }
        let bare = name.rsplit("::").next().unwrap_or(name);
        let ambiguous = self
            .port_names
            .ambiguous
            .iter()
            .find(|(port, _)| port == bare);
        if let Some((_, owners)) = ambiguous {
            if name == bare {
                target.extend(owners.iter().map(|owner| (owner.clone(), value.clone())));
            } else {
                target.insert(name.to_owned(), value.clone());
            }
        } else {
            let qualified = if name == bare {
                self.port_names
                    .qualified
                    .iter()
                    .find(|key| key.rsplit_once("::").is_some_and(|(_, port)| port == bare))
            } else {
                self.port_names.qualified.get(name)
            };
            target.insert(
                qualified.map_or_else(|| name.to_owned(), String::clone),
                value.clone(),
            );
        }
    }

    /// Выводит краткое имя, если оно однозначно и допустимо для поля шага.
    fn recorded_ports(
        &self,
        ports: BTreeMap<String, Value>,
        direction: PortDirectionKind,
    ) -> Option<PortValues> {
        (!ports.is_empty()).then(|| {
            let named = ports
                .into_iter()
                .map(|(name, value)| {
                    let bare = name.rsplit("::").next().unwrap_or(&name);
                    let unambiguous = !self
                        .port_names
                        .ambiguous
                        .iter()
                        .any(|(port, _)| port == bare);
                    let key = if unambiguous
                        && self.names_of(direction).iter().any(|port| port == bare)
                    {
                        bare.to_owned()
                    } else {
                        name
                    };
                    (key, value)
                })
                .collect();
            PortValues::Named(named)
        })
    }
}
