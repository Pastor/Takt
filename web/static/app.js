// Страница онлайн-редактора Takt (фича 0531, R6/R7/R13).
//
// Связывает четыре вещи: редактор (`editor.js`), модуль (`bridge.js`), прогон
// в отдельном потоке (`worker.js`) и ссылку с черновиком (`share.js`,
// `draft.js`). Своей логики языка здесь нет — только показ ответов модуля.

import { Bridge, spans } from "./bridge.js";
import { Editor, paintCode, positionToOffset } from "./editor.js";
import * as draft from "./draft.js";
import { encodeState, decodeState } from "./share.js";

/** Адрес модуля. Версия — в пути (решение A5): ссылка открывается СВОИМ модулем. */
const WASM_DEFAULT = "takt.wasm";

/**
 * Порог, за которым вывод цели показывается без подсветки.
 *
 * Подсветка стоит разметки: у файла в сотни тысяч символов узлов-строк со
 * span-ами становится столько, что вкладка перестаёт открываться мгновенно.
 * Предел назван словами в шапке файла, а не молчаливо снят: «почему тут нет
 * цвета» — вопрос, на который автор должен получать ответ.
 */
const HIGHLIGHT_LIMIT = 200_000;

/** Модель, с которой открывается пустой редактор. */
const SAMPLE = `// Термореле: греет, пока холодно, и ждёт, пока не остынет.
in temperature: u8;
out heater: bit;

const HOT := 24;
const COLD := 20;

start Heating {
    always {
        heater := 1;
    }

    ref Cooling: temperature >= HOT;
}

state Cooling {
    always {
        heater := 0;
    }

    ref Heating: temperature <= COLD;
}
`;

const state = {
  bridge: null,
  editor: null,
  worker: null,
  target: "c",
  args: "",
  scenario: "",
  version: "",
  running: false,
};

const dom = {};

/** Точка входа страницы. */
export async function main() {
  cache();
  state.bridge = await Bridge.load(WASM_DEFAULT);
  const version = state.bridge.version();
  state.version = version.takt_lang ?? "";
  dom.version.textContent = `язык ${version.language} · модуль ${version.takt_lang}`;
  fillTargets(version.targets ?? []);

  state.editor = new Editor(dom.editor, onEdit);
  wire();

  const restored = (await decodeState(location.hash)) ?? draft.load(localStorage);
  applyState(restored ?? { source: SAMPLE });
  refresh();
}

/** Находит узлы страницы один раз: поиск в обработчике — лишняя работа. */
function cache() {
  for (const id of [
    "editor", "diagnostics", "output", "trace", "version", "target", "args",
    "scenario", "budget", "share", "run", "stop", "format", "status", "tabs", "modes",
  ]) {
    dom[id] = document.getElementById(id);
  }
}

function fillTargets(targets) {
  dom.target.replaceChildren();
  for (const target of targets) {
    const option = document.createElement("option");
    option.value = target;
    option.textContent = target;
    dom.target.appendChild(option);
  }
  dom.target.value = state.target;
}

function wire() {
  dom.target.addEventListener("change", () => {
    state.target = dom.target.value;
    compile();
    saveDraft();
  });
  dom.args.addEventListener("input", () => {
    state.args = dom.args.value;
    compile();
    saveDraft();
  });
  dom.scenario.addEventListener("input", () => {
    state.scenario = dom.scenario.value;
    saveDraft();
  });
  dom.format.addEventListener("click", format);
  dom.run.addEventListener("click", run);
  dom.stop.addEventListener("click", stop);
  dom.share.addEventListener("click", share);
  dom.tabs.addEventListener("click", (event) => {
    const tab = event.target.closest("[data-tab]");
    if (tab) selectTab(tab.dataset.tab);
  });
  dom.modes.addEventListener("click", (event) => {
    const mode = event.target.closest("[data-mode]");
    if (mode) selectMode(mode.dataset.mode);
  });

  // Наведение: подсказка идёт в строку состояния, а не всплывающим окном.
  // Всплывашка над кодом закрывает сам код, а на устройстве без наведения её
  // не существует вовсе (правило референса: `hover` есть не везде).
  dom.editor.addEventListener("mousemove", onHover);
  dom.editor.addEventListener("mouseleave", () => say("", "ok"));

  // Alt+клик — переход к объявлению и подсветка использований: правая рука
  // остаётся на мыши, а сочетание не занято браузером.
  dom.editor.addEventListener("click", (event) => {
    if (!event.altKey) return;
    event.preventDefault();
    declarationAndUses();
  });

  // F2 — переименование, как в редакторах на машине.
  dom.editor.addEventListener("keydown", (event) => {
    if (event.key === "F2") {
      event.preventDefault();
      renameSymbol();
    }
  });

  // Несохранённая работа не теряется молча (R13): подтверждение ухода —
  // единственное, что браузер позволяет здесь сделать.
  window.addEventListener("beforeunload", (event) => {
    if (state.editor?.value()) {
      event.preventDefault();
      event.returnValue = "";
    }
  });
}

const saveDraft = draft.debounce(() => {
  const problem = draft.save(localStorage, {
    source: state.editor.value(),
    scenario: state.scenario,
    target: state.target,
    args: state.args,
  });
  if (problem) say(problem, "warning");
}, 400);

function applyState(restored) {
  state.target = restored.target || state.target;
  state.args = restored.args ?? "";
  state.scenario = restored.scenario ?? "";
  dom.target.value = state.target;
  dom.args.value = state.args;
  dom.scenario.value = state.scenario;
  state.editor.setValue(restored.source ?? SAMPLE);
}

/** Правка текста: подсветка, диагностики, вывод цели. */
function onEdit() {
  refresh();
  saveDraft();
}

function refresh() {
  const source = state.editor.value();
  const diagnostics = state.bridge.diagnostics(source);
  const tokens = state.bridge.tokens(source);
  state.editor.highlight(tokens, diagnostics.diagnostics ?? []);
  showDiagnostics(diagnostics.diagnostics ?? []);
  compile();
}

function showDiagnostics(items) {
  dom.diagnostics.replaceChildren();
  if (items.length === 0) {
    dom.diagnostics.appendChild(row("Ошибок нет", "ok"));
    return;
  }
  for (const item of items) {
    // Диагностика показывается СЛОВАМИ, а не только подчёркиванием: цвет —
    // не единственный носитель состояния (доступность, замер задачи 07).
    const line = (item.range?.start_line ?? 0) + 1;
    const column = (item.range?.start_character ?? 0) + 1;
    const code = item.code ? `[${item.code}] ` : "";
    const node = row(`${line}:${column}: ${code}${item.message}`, item.severity ?? "error");
    node.addEventListener("click", () => jump(item.range?.start_line ?? 0, item.range?.start_character ?? 0));
    dom.diagnostics.appendChild(node);
  }
}

function jump(line, character) {
  const offset = positionToOffset(state.editor.value(), line, character);
  dom.editor.focus();
  const selection = document.getSelection();
  if (!selection) return;
  const walker = document.createTreeWalker(dom.editor, NodeFilter.SHOW_TEXT);
  let seen = 0;
  let node = walker.nextNode();
  while (node) {
    if (seen + node.textContent.length >= offset) {
      const range = document.createRange();
      range.setStart(node, offset - seen);
      range.collapse(true);
      selection.removeAllRanges();
      selection.addRange(range);
      return;
    }
    seen += node.textContent.length;
    node = walker.nextNode();
  }
}

/** Подсказка при наведении: тип и объявление под курсором мыши. */
const onHover = draft.debounce((event) => {
  const at = positionAt(event.clientX, event.clientY);
  if (!at) return;
  const reply = state.bridge.hover(state.editor.value(), at.line, at.character);
  // Пусто — не ошибка: под курсором просто нет имени, и молчание здесь верно.
  say(reply.ok ? (reply.contents ?? "") : "", "ok");
}, 120);

/** Позиция документа под точкой экрана. */
function positionAt(x, y) {
  const range = document.caretRangeFromPoint?.(x, y)
    ?? document.caretPositionFromPoint?.(x, y);
  if (!range) return null;
  const node = range.startContainer ?? range.offsetNode;
  const offsetInNode = range.startOffset ?? range.offset;
  const walker = document.createTreeWalker(dom.editor, NodeFilter.SHOW_TEXT);
  let seen = 0;
  let current = walker.nextNode();
  let lineBreaks = 0;
  while (current) {
    if (current === node) break;
    seen += current.textContent.length;
    // Узлы-строки блочные: перевод строки в тексте есть, а в узлах — нет.
    const parent = current.parentElement?.closest(".line");
    const nextParent = walker.currentNode?.parentElement?.closest(".line");
    if (parent && nextParent && parent !== nextParent) lineBreaks += 1;
    current = walker.nextNode();
  }
  const offset = seen + lineBreaks + offsetInNode;
  const { line, character } = offsetToPositionSafe(state.editor.value(), offset);
  return { line, character };
}

function offsetToPositionSafe(text, offset) {
  const clamped = Math.max(0, Math.min(offset, text.length));
  const before = text.slice(0, clamped);
  return {
    line: before.split("\n").length - 1,
    character: clamped - (before.lastIndexOf("\n") + 1),
  };
}

/** Переход к объявлению и показ использований символа под курсором. */
function declarationAndUses() {
  const at = state.editor.position();
  const source = state.editor.value();
  const uses = state.bridge.references(source, at.line, at.character);
  const target = state.bridge.goto(source, at.line, at.character);
  if (target.ok && target.range) {
    jump(target.range.start_line, target.range.start_character);
    say(`объявление: строка ${target.range.start_line + 1}; использований: ${uses.ranges?.length ?? 0}`, "ok");
    return;
  }
  say("под курсором нет имени с объявлением", "warning");
}

/** Переименование символа под курсором. */
function renameSymbol() {
  const at = state.editor.position();
  const source = state.editor.value();
  const newName = prompt("Новое имя:");
  if (!newName) return;
  const reply = state.bridge.rename(source, at.line, at.character, newName);
  if (!reply.ok) {
    // Отказ переименования назван причиной слоя: «полнота или отказ» — его
    // правило, и прятать причину значило бы оставить автора в догадках.
    say(reply.error?.message ?? "переименование недоступно", "warning");
    return;
  }
  // Правки применяются С КОНЦА: иначе каждая сдвигала бы координаты
  // следующих, и текст расползся бы.
  const edits = [...(reply.edits ?? [])].sort(
    (a, b) => b.range.start_line - a.range.start_line || b.range.start_character - a.range.start_character
  );
  let text = source;
  for (const edit of edits) {
    const from = positionToOffset(text, edit.range.start_line, edit.range.start_character);
    const to = positionToOffset(text, edit.range.end_line, edit.range.end_character);
    text = text.slice(0, from) + edit.new_text + text.slice(to);
  }
  state.editor.setValue(text);
  say(`переименовано: ${edits.length} вхождений`, "ok");
}

/** Компилирует текущей целью и показывает вывод. */
function compile() {
  const reply = state.bridge.compile(state.target, state.args, state.editor.value());
  dom.output.replaceChildren();
  if (!reply.ok) {
    // Отказ цели показывается КАК ЕЁ ДИАГНОСТИКА (критерий 5 фичи): код,
    // позиция и текст — те же, что печатает `taktc`.
    const code = reply.error?.code ? `[${reply.error.code}] ` : "";
    const where = reply.error?.line ? `${reply.error.line}:${reply.error.column}: ` : "";
    dom.output.appendChild(row(`${where}${code}${reply.error?.message ?? "отказ"}`, "error"));
    return;
  }
  for (const file of reply.files ?? []) {
    const header = document.createElement("div");
    header.className = "file-name";
    header.textContent = file.name;
    const body = document.createElement("pre");
    body.className = "file-text";
    paintOutput(body, file.text, header);
    dom.output.append(header, body);
  }
  for (const warning of reply.warnings ?? []) {
    dom.output.appendChild(row(`[${warning.code ?? "?"}] ${warning.message}`, "warning"));
  }
}

/**
 * Красит порождённый файл по правилам ЕГО языка (задача 06, требование R11).
 *
 * Отрезки приходят от модуля (`takt_highlight`) в той же форме, что токены
 * исходника, и раскладывает их тот же `paintCode`: своего разбора C, ST, Rust,
 * SystemVerilog или PlantUML в браузере нет — он разошёлся бы и с целями, и с
 * подсветкой блоков кода в документе.
 */
function paintOutput(body, text, header) {
  if (text.length > HIGHLIGHT_LIMIT) {
    body.textContent = text;
    header.append(note(`без подсветки: файл длиннее ${HIGHLIGHT_LIMIT} символов`));
    return;
  }
  const reply = state.bridge.highlight(state.target, text);
  if (!reply.ok) {
    // Отказ подсветки — не отказ сборки: файл показывается как есть, а причина
    // называется. Молчаливый чёрный текст выглядел бы дефектом вёрстки.
    body.textContent = text;
    header.append(note(reply.error?.message ?? "подсветка недоступна"));
    return;
  }
  header.append(note(reply.language));
  body.replaceChildren(paintCode(text, spans(reply)));
}

/** Приписка у имени файла: язык вывода либо причина, по которой цвета нет. */
function note(text) {
  const node = document.createElement("span");
  node.className = "file-note";
  node.textContent = text;
  return node;
}

function format() {
  const reply = state.bridge.format(state.editor.value());
  if (!reply.ok) {
    say(reply.error?.message ?? "форматирование недоступно", "warning");
    return;
  }
  if (reply.text === null || reply.text === undefined) {
    say("документ уже в каноне", "ok");
    return;
  }
  state.editor.setValue(reply.text);
  say("отформатировано", "ok");
}

/** Запускает прогон в отдельном потоке. */
function run() {
  if (state.running) return;
  selectTab("trace");
  selectMode("trace");
  dom.trace.replaceChildren();
  state.running = true;
  dom.run.disabled = true;
  dom.stop.disabled = false;

  state.worker?.terminate();
  state.worker = new Worker("worker.js", { type: "module" });
  state.worker.onmessage = (event) => onWorker(event.data ?? {});
  state.worker.postMessage({
    type: "run",
    wasmUrl: new URL(WASM_DEFAULT, location.href).href,
    source: state.editor.value(),
    scenario: state.scenario,
    tickMs: 0,
    budget: Number(dom.budget.value) || 10_000,
  });
}

function stop() {
  state.worker?.postMessage({ type: "stop" });
}

function onWorker(message) {
  switch (message.type) {
    case "lines":
      for (const line of message.lines) dom.trace.appendChild(row(line, "trace"));
      dom.trace.scrollTop = dom.trace.scrollHeight;
      break;
    case "warnings":
      for (const line of message.lines) dom.trace.appendChild(row(line, "warning"));
      break;
    case "finished":
      for (const line of message.info ?? []) dom.trace.appendChild(row(line, "ok"));
      for (const line of message.errors ?? []) dom.trace.appendChild(row(line, "error"));
      finish();
      break;
    case "halted":
      // Останов называется словами — и по бюджету, и по просьбе автора:
      // молчаливо оборванный прогон неотличим от завершившегося.
      dom.trace.appendChild(row(message.reason, "warning"));
      finish();
      break;
    case "failed":
      dom.trace.appendChild(row(message.message, "error"));
      finish();
      break;
    default:
      break;
  }
}

function finish() {
  state.running = false;
  dom.run.disabled = false;
  dom.stop.disabled = true;
}

/** Кладёт состояние редактора в адресную строку и в буфер обмена. */
async function share() {
  const fragment = await encodeState({
    version: state.version,
    source: state.editor.value(),
    scenario: state.scenario,
    target: state.target,
    args: state.args,
  });
  const url = `${location.origin}${location.pathname}#${fragment}`;
  history.replaceState(null, "", `#${fragment}`);
  try {
    await navigator.clipboard.writeText(url);
    say(`ссылка скопирована (${url.length} символов)`, "ok");
  } catch {
    // Буфер обмена требует разрешения и жеста; ссылка уже в адресной строке —
    // сказать об этом важнее, чем промолчать об отказе.
    say("ссылка в адресной строке — скопируйте её", "warning");
  }
}

function selectTab(name) {
  for (const tab of dom.tabs.querySelectorAll("[data-tab]")) {
    const active = tab.dataset.tab === name;
    tab.classList.toggle("active", active);
    tab.setAttribute("aria-selected", String(active));
  }
  for (const panel of document.querySelectorAll("[data-panel]")) {
    panel.hidden = panel.dataset.panel !== name;
  }
}

/**
 * Выбирает область на узком экране: модель, вывод или прогон.
 *
 * ⚠️ Слушателя изменения размера окна здесь нет и не нужно: какая раскладка
 * действует, решает CSS по ширине. Расширив окно, автор видит обе области при
 * любом выбранном режиме — а `resize`-обработчик пришлось бы держать
 * согласованным с медиазапросом, то есть завести второй носитель порога.
 */
function selectMode(name) {
  document.body.dataset.mode = name;
  for (const mode of dom.modes.querySelectorAll("[data-mode]")) {
    const active = mode.dataset.mode === name;
    mode.classList.toggle("active", active);
    mode.setAttribute("aria-selected", String(active));
  }
  if (name !== "source") selectTab(name);
}

function row(text, kind) {
  const node = document.createElement("div");
  node.className = `row row-${kind}`;
  node.textContent = text;
  return node;
}

function say(text, kind) {
  dom.status.textContent = text;
  dom.status.className = `status status-${kind}`;
}
