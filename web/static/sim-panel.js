// Интерактивная панель прогона: входы модели вручную и выбранные выходы.
//
// # Чего здесь нет
//
// Знания о языке. Состав портов, их типы, границы и варианты перечислений
// отдаёт модуль при открытии прогона (`ports`), значения после такта - эталон
// (`values`). Панель только строит поле по роду и переводит набранное в
// значение той формы, которую ждёт сценарий.
//
// # Куда уходит набранное
//
// В поток прогона, а оттуда в эталон: значение ложится перед следующим тактом
// после шага сценария и удерживается. Разбирает его эталон той же воронкой,
// что шаг сценария, - отказ приходит его текстом.

/** Роды портов, которые вводятся полем. Составной порт только наблюдается. */
const EDITABLE = ["bit", "bool", "integer", "duration", "fixed", "float", "enum"];

/** Вводится ли порт полем панели. */
export function editable(port) {
  return (port.direction === "in" || port.direction === "inout") && EDITABLE.includes(port.kind);
}

/** Наблюдается ли порт на вкладке выходов. */
export function observable(port) {
  return port.direction === "out" || port.direction === "inout";
}

/**
 * Значение поля в форме сценария либо отказ с ключом словаря.
 *
 * Границы целого приходят строками: `u64` и `i64` в число JavaScript без потери
 * не укладываются, поэтому сравнение идёт `BigInt`. Значение за пределами
 * точного целого JavaScript отвергается словами, а не округляется.
 *
 * @param {object} port порт из ответа модуля
 * @param {string|boolean} raw набранное: текст поля либо отметка переключателя
 * @returns {{value: number|boolean}|{key: string, params: object}} итог
 */
export function fieldValue(port, raw) {
  const name = port.name;
  switch (port.kind) {
    case "bit":
      return { value: raw === true || raw === "1" ? 1 : 0 };
    case "bool":
      return { value: raw === true || raw === "true" };
    case "enum": {
      const variant = (port.variants ?? []).find((item) => item.value === String(raw) || item.name === raw);
      return variant ? { value: Number(variant.value) } : { key: "simPanel.badVariant", params: { name } };
    }
    case "integer":
    case "duration": {
      const text = String(raw).trim();
      if (!/^-?\d+$/.test(text)) return { key: "simPanel.notInteger", params: { name } };
      const big = BigInt(text);
      const min = port.kind === "duration" ? 0n : BigInt(port.min);
      const max = port.kind === "duration" ? BigInt(Number.MAX_SAFE_INTEGER) : BigInt(port.max);
      if (big < min || big > max) {
        return { key: "simPanel.outOfRange", params: { name, min: String(min), max: String(max) } };
      }
      if (big > BigInt(Number.MAX_SAFE_INTEGER) || big < BigInt(Number.MIN_SAFE_INTEGER)) {
        return { key: "simPanel.tooWide", params: { name } };
      }
      return { value: Number(big) };
    }
    case "fixed":
    case "float": {
      const number = Number(String(raw).trim().replace(",", "."));
      if (String(raw).trim() === "" || !Number.isFinite(number)) {
        return { key: "simPanel.notNumber", params: { name } };
      }
      return { value: number };
    }
    default:
      return { key: "simPanel.notEditable", params: { name } };
  }
}

/**
 * Запрос ручного ввода: значение ложится в поле шага по направлению порта.
 *
 * @param {object} port порт из ответа модуля
 * @param {number|boolean} value значение в форме сценария
 * @returns {{in_ports: object, inout: object}} запрос модулю
 */
export function inputRequest(port, value) {
  const field = port.direction === "inout" ? "inout" : "in_ports";
  return { in_ports: {}, inout: {}, [field]: { [port.name]: value } };
}

/**
 * Отмеченные выходы, которых в модели уже нет, отбрасываются, порядок - модели.
 *
 * @param {object[]} ports порты из ответа модуля
 * @param {string[]} watched сохранённые имена
 * @returns {string[]} имена наблюдаемых портов
 */
export function keptWatch(ports, watched) {
  const wanted = new Set(watched ?? []);
  return ports.filter((port) => observable(port) && wanted.has(port.name)).map((port) => port.name);
}

/** Панель на странице: строит поля по портам и показывает значения. */
export class SimPanel {
  /**
   * @param {object} dom узлы: `root`, `inputsTab`, `outputsTab`, `inputs`, `outputs`, `empty`
   * @param {object} options `t` - словарь, `send(request)` - ручной ввод,
   *   `say(text, kind)` - полоса речи, `onWatch(names)` - смена набора выходов
   */
  constructor(dom, options) {
    this.dom = dom;
    this.t = options.t;
    this.send = options.send;
    this.say = options.say;
    this.onWatch = options.onWatch ?? (() => {});
    this.ports = [];
    this.watched = [];
    this.values = {};
    this.tab = "inputs";
    dom.inputsTab.addEventListener("click", () => this.showTab("inputs"));
    dom.outputsTab.addEventListener("click", () => this.showTab("outputs"));
    this.showTab("inputs");
  }

  /** Показывает вкладку. */
  showTab(which) {
    this.tab = which === "outputs" ? "outputs" : "inputs";
    const outputs = this.tab === "outputs";
    this.dom.inputs.hidden = outputs;
    this.dom.outputs.hidden = !outputs;
    this.dom.inputsTab.setAttribute("aria-pressed", String(!outputs));
    this.dom.outputsTab.setAttribute("aria-pressed", String(outputs));
  }

  /** Новый прогон: порты, сохранённый набор выходов и значения до первого такта. */
  setPorts(ports, values, watched) {
    this.ports = ports ?? [];
    this.watched = keptWatch(this.ports, watched);
    this.values = values ?? {};
    this.render();
  }

  /** Набор выходов пришёл снаружи - из проекта. */
  setWatched(watched) {
    this.watched = keptWatch(this.ports, watched);
    this.renderOutputs();
  }

  /** Значения после такта. */
  setValues(values) {
    this.values = { ...this.values, ...(values ?? {}) };
    for (const node of this.dom.root.querySelectorAll("[data-value-of]")) {
      node.textContent = this.values[node.dataset.valueOf] ?? "";
    }
  }

  render() {
    this.dom.empty.hidden = this.ports.length > 0;
    this.renderInputs();
    this.renderOutputs();
  }

  renderInputs() {
    const list = this.dom.inputs;
    list.replaceChildren();
    const ports = this.ports.filter(editable);
    if (ports.length === 0 && this.ports.length > 0) list.appendChild(note(this.t("simPanel.noInputs")));
    for (const port of ports) list.appendChild(this.inputRow(port));
  }

  renderOutputs() {
    const list = this.dom.outputs;
    list.replaceChildren();
    const ports = this.ports.filter(observable);
    if (ports.length === 0 && this.ports.length > 0) list.appendChild(note(this.t("simPanel.noOutputs")));
    for (const port of ports) list.appendChild(this.outputRow(port));
  }

  inputRow(port) {
    const row = element("div", "sim-io-row");
    const label = element("label", "sim-io-name");
    const id = `sim-in-${port.name.replace(/[^A-Za-z0-9_-]/g, "_")}`;
    label.htmlFor = id;
    label.textContent = port.name;
    label.dataset.tip = port.type;
    const control = this.control(port, id);
    const value = element("span", "sim-io-value");
    value.dataset.valueOf = port.name;
    value.textContent = this.values[port.name] ?? "";
    row.append(label, control, value);
    return row;
  }

  control(port, id) {
    const commit = (raw) => {
      const got = fieldValue(port, raw);
      if ("key" in got) {
        this.say(this.t(got.key, got.params), "warning");
        return;
      }
      this.send(inputRequest(port, got.value));
    };
    if (port.kind === "bit" || port.kind === "bool") {
      const box = element("input", "sim-io-switch");
      box.type = "checkbox";
      box.id = id;
      box.checked = ["1", "true"].includes(this.values[port.name]);
      box.addEventListener("change", () => commit(box.checked));
      return box;
    }
    if (port.kind === "enum") {
      const select = element("select", "sim-io-field");
      select.id = id;
      for (const variant of port.variants ?? []) {
        const option = document.createElement("option");
        option.value = variant.value;
        option.textContent = variant.name;
        select.appendChild(option);
      }
      select.value = this.values[port.name] ?? "";
      select.addEventListener("change", () => commit(select.value));
      return select;
    }
    const input = element("input", "sim-io-field");
    input.id = id;
    input.type = "text";
    input.inputMode = port.kind === "integer" || port.kind === "duration" ? "numeric" : "decimal";
    input.placeholder = port.kind === "duration" ? this.t("simPanel.millis") : port.type;
    // Значение уходит по Enter и уходу из поля: по каждой букве эталон получал бы
    // промежуточные числа, и прогон шёл бы с тем, чего читатель не набирал.
    input.addEventListener("change", () => commit(input.value));
    // Enter подтверждает набранное: без этого значение уходило бы только с уходом
    // из поля, и читатель, нажавший Enter, ждал бы эталон, который ничего не получил.
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        input.blur();
      }
    });
    return input;
  }

  outputRow(port) {
    const row = element("div", "sim-io-row");
    const box = element("input", "sim-io-switch");
    box.type = "checkbox";
    const id = `sim-out-${port.name.replace(/[^A-Za-z0-9_-]/g, "_")}`;
    box.id = id;
    box.checked = this.watched.includes(port.name);
    box.addEventListener("change", () => {
      const wanted = new Set(this.watched);
      if (box.checked) wanted.add(port.name);
      else wanted.delete(port.name);
      this.watched = keptWatch(this.ports, [...wanted]);
      this.renderOutputs();
      this.onWatch(this.watched);
    });
    const label = element("label", "sim-io-name");
    label.htmlFor = id;
    label.textContent = port.name;
    label.dataset.tip = port.type;
    const value = element("span", "sim-io-value");
    // Значение показывается только у отмеченного: список наблюдения и есть выбор
    // того, на что смотреть.
    if (box.checked) {
      value.dataset.valueOf = port.name;
      value.textContent = this.values[port.name] ?? "";
    }
    row.append(box, label, value);
    return row;
  }
}

function element(tag, className) {
  const node = document.createElement(tag);
  node.className = className;
  return node;
}

function note(text) {
  const node = element("div", "sim-io-note");
  node.textContent = text;
  return node;
}
