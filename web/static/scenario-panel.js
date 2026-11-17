// Редактируемый сценарий и текущая позиция прогона живут отдельно.

import { readFields, readSteps, writeFields, writeSteps } from "./scenario-steps.js";

const FIELDS = ["in_ports", "guard", "extern", "_step"];

/** Управляет навигацией по JSON-шагам и коммитит правку в буфер файла. */
export class ScenarioPanel {
  constructor(dom, host) {
    this.dom = dom;
    this.host = host;
    this.index = 0;
    this.steps = null;
    this.pendingRestart = false;
    this.open = false;
    this.completed = 0;
    this.fields = dom["scenario-fields"];
    this.fields.addEventListener("submit", (event) => {
      event.preventDefault();
      this.apply();
    });
    this.fields.addEventListener("change", () => this.apply());
    dom["scenario-edit"].addEventListener("click", () => this.toggle());
    dom["scenario-prev"].addEventListener("click", () => this.move(-1));
    dom["scenario-next"].addEventListener("click", () => this.move(1));
    dom["scenario-insert"].addEventListener("click", () => this.insert());
    dom["scenario-delete"].addEventListener("click", () => this.remove());
    dom["scenario-save"].addEventListener("click", () => {
      if (this.apply()) host.save();
    });
    dom["scenario-restart"].addEventListener("click", () => {
      if (this.apply()) host.restart();
    });
  }

  toggle() {
    if (this.open && !this.apply()) return;
    if (!this.open) {
      try {
        this.steps = readSteps(this.host.text());
      } catch (error) {
        this.refuse(error);
        return;
      }
      this.index = 0;
      this.reset();
      this.host.stopRecording();
    }
    this.setOpen(!this.open);
    this.host.modeChanged();
  }

  setOpen(open) {
    this.open = open;
    this.dom["scenario-edit"].setAttribute("aria-pressed", String(open));
    this.dom["scenario-tools"].hidden = !open;
    this.dom["simio-live"].hidden = open;
    this.render();
  }

  /** При внешней замене текста прежние поля теряют связь с открытым файлом. */
  invalidate() {
    if (!this.open) return;
    this.setOpen(false);
    this.host.modeChanged();
  }

  move(offset) {
    if (!this.apply()) return;
    this.index += offset;
    this.render();
  }

  insert() {
    if (!this.apply()) return;
    if (this.steps.length) this.index += 1;
    this.steps.splice(this.index, 0, "{}");
    this.changed(this.index, true);
    this.render();
  }

  remove() {
    this.steps.splice(this.index, 1);
    const removed = this.index;
    this.index = Math.max(0, Math.min(this.index, this.steps.length - 1));
    this.changed(removed, true);
    this.render();
  }

  /** Проверяет введённый JSON до обновления черновика. */
  apply() {
    if (!this.open || !this.steps.length) return true;
    const fields = readFields(this.steps[this.index]);
    let dirty = false;
    for (const key of FIELDS) {
      const field = this.fields.elements.namedItem(key);
      const value = field.value.trim();
      if (value === (fields.get(key) ?? "")) continue;
      try {
        if (value) {
          const parsed = JSON.parse(value);
          if (key !== "_step" && parsed !== null && (typeof parsed !== "object"
              || (key !== "in_ports" && Array.isArray(parsed)))) {
            throw new Error(this.host.t("scenarioEdit.badShape"));
          }
        }
      } catch (error) {
        this.host.say(this.host.t("scenarioEdit.badField", { field: key, error: error.message }), "warning");
        field.focus();
        return false;
      }
      if (value) fields.set(key, value);
      else fields.delete(key);
      dirty = true;
    }
    if (dirty) {
      this.steps[this.index] = writeFields(fields);
      this.changed(this.index);
    }
    return true;
  }

  changed(index, structural = false) {
    if (structural && index < this.completed) this.pendingRestart = true;
    const text = writeSteps(this.steps);
    this.host.change(text);
    if (!this.pendingRestart) this.snapshot(text);
    if (this.pendingRestart || index < this.completed) {
      this.host.say(this.host.t("scenarioEdit.afterRestart"), "warning");
    }
  }

  /** Сдвиг пройденных шагов требует заново пройти весь сценарий. */
  reset() {
    this.pendingRestart = false;
    this.snapshot(this.host.text());
    this.progress(0);
  }

  snapshot(text) {
    this.playText = text;
    this.playLength = this.steps?.length ?? 0;
  }

  progress(completed, follow = false) {
    this.completed = completed;
    if (this.open && follow && completed > 0 && !this.pendingRestart) {
      this.index = Math.min(completed - 1, this.steps.length - 1);
      this.render();
    }
    this.paintProgress();
  }

  paintProgress() {
    this.dom["scenario-progress"].textContent = this.host.t("scenarioEdit.progress", { n: this.completed });
  }

  render() {
    if (!this.open) return;
    const step = this.steps[this.index];
    const fields = step ? readFields(step) : new Map();
    for (const key of FIELDS) {
      const field = this.fields.elements.namedItem(key);
      field.value = fields.get(key) ?? "";
      field.disabled = !step || this.host.running();
    }
    this.dom["scenario-prev"].disabled = this.index === 0 || this.host.running();
    this.dom["scenario-next"].disabled = this.index >= this.steps.length - 1 || this.host.running();
    this.dom["scenario-delete"].disabled = !step || this.host.running();
    this.dom["scenario-apply"].disabled = !step || this.host.running();
    this.dom["scenario-insert"].disabled = this.host.running();
    this.dom["scenario-restart"].disabled = this.host.running();
    this.dom["scenario-save"].hidden = !this.host.writable();
    this.dom["scenario-save"].disabled = this.host.running();
    this.dom["scenario-position"].textContent = this.host.t("scenarioEdit.position", {
      n: step ? this.index + 1 : 0,
      all: this.steps.length,
    });
    this.paintProgress();
  }

  refuse(error) {
    this.host.say(this.host.t("scenarioEdit.badField", { field: "JSON", error: error instanceof SyntaxError
      ? error.message
      : this.host.t("scenarioEdit.badSteps") }), "warning");
  }
}
