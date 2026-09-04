// Редактор текста с подсветкой по токенам модуля (фича 0531, R6).
//
// # Почему свой, а не библиотека
//
// Решение заказчика 2026-09-04: фронтенд без зависимостей, как в референсе.
// Отсюда объём: выделение, отмена и ввод берутся у самого браузера
// (`contenteditable`), а от редактора требуется немногое — красить текст и
// знать, где стоит курсор.
//
// # Чего здесь нет
//
// Знания о языке Takt. Ни списка ключевых слов, ни разбора: цвет каждому
// отрезку назначает модуль (`takt_tokens` → `bridge.spans`), а страница лишь
// раскладывает отрезки по строкам. Заведи здесь свой словарь — и подсветка
// начнёт расходиться с компилятором молча, как расходились параллельные списки
// у LSP (класс 0232).

import { spans } from "./bridge.js";

/** Редактор поверх `contenteditable`-узла. */
export class Editor {
  /**
   * @param {HTMLElement} root узел редактора
   * @param {() => void} onChange зовётся после правки текста
   */
  constructor(root, onChange) {
    this.root = root;
    this.onChange = onChange ?? (() => {});
    this.text = "";
    this.root.setAttribute("contenteditable", "true");
    this.root.setAttribute("spellcheck", "false");
    this.root.setAttribute("autocorrect", "off");
    this.root.setAttribute("autocapitalize", "off");

    this.root.addEventListener("input", () => {
      this.text = readText(this.root);
      this.onChange();
    });

    // Вставка идёт ТОЛЬКО текстом: иначе в редактор попадает разметка со
    // страницы-источника (цвета, шрифты, ссылки), и текст модели перестаёт
    // быть текстом.
    this.root.addEventListener("paste", (event) => {
      event.preventDefault();
      const text = event.clipboardData?.getData("text/plain") ?? "";
      document.execCommand("insertText", false, text);
    });

    // Tab — отступ, а не уход фокуса: редактор кода, в котором нельзя набрать
    // отступ, бесполезен. Уйти с него можно Escape + Tab.
    this.root.addEventListener("keydown", (event) => {
      if (event.key === "Tab") {
        event.preventDefault();
        document.execCommand("insertText", false, "    ");
      }
    });
  }

  /** Текущий текст документа. */
  value() {
    return this.text;
  }

  /** Заменяет текст целиком (открытие ссылки, форматирование, черновик). */
  setValue(text) {
    this.text = text;
    this.root.textContent = text;
    this.onChange();
  }

  /** Позиция курсора: строка и колонка с нуля, как в LSP. */
  position() {
    const offset = caretOffset(this.root);
    return offsetToPosition(this.text, offset);
  }

  /**
   * Красит текст отрезками из ответа `takt_tokens`.
   *
   * ⚠️ Курсор сохраняется по СМЕЩЕНИЮ в тексте: перестройка разметки уносит
   * узел, в котором он стоял, и без сохранения каждая покраска отправляла бы
   * курсор в начало документа — то есть печатать было бы нельзя.
   */
  highlight(tokens, diagnostics) {
    const offset = caretOffset(this.root);
    const marks = spans(tokens ?? {});
    const lines = this.text.split("\n");
    const problems = new Map();
    for (const d of diagnostics ?? []) {
      const line = d.range?.start_line ?? 0;
      if (!problems.has(line)) problems.set(line, d.severity ?? "error");
    }

    const fragment = document.createDocumentFragment();
    for (let i = 0; i < lines.length; i += 1) {
      const lineNode = document.createElement("div");
      lineNode.className = "line";
      const severity = problems.get(i);
      if (severity) lineNode.classList.add(`has-${severity}`);
      paintLine(lineNode, lines[i], marks.filter((m) => m.line === i));
      fragment.appendChild(lineNode);
    }
    this.root.replaceChildren(fragment);
    setCaretOffset(this.root, offset);
  }
}

/** Раскладывает отрезки токенов по одной строке. */
function paintLine(lineNode, text, marks) {
  if (text.length === 0) {
    // Пустая строка обязана занимать высоту: узел без содержимого браузер
    // схлопывает, и документ «прыгает» при наборе.
    lineNode.appendChild(document.createElement("br"));
    return;
  }
  let at = 0;
  for (const mark of marks.sort((a, b) => a.column - b.column)) {
    if (mark.column < at) continue;
    if (mark.column > at) {
      lineNode.appendChild(document.createTextNode(text.slice(at, mark.column)));
    }
    const end = Math.min(mark.column + mark.length, text.length);
    const span = document.createElement("span");
    span.className = `tok tok-${mark.type}`;
    span.textContent = text.slice(mark.column, end);
    lineNode.appendChild(span);
    at = end;
  }
  if (at < text.length) {
    lineNode.appendChild(document.createTextNode(text.slice(at)));
  }
}

/**
 * Текст узла редактора — с переводами строк.
 *
 * ⚠️ Берётся `textContent` узлов-строк, а не `innerText` корня: каждая строка —
 * блочный узел, и `innerText` добавляет к ней перевод СВЕРХ того, что есть в
 * тексте. Прогон страницы 2026-09-04 показал это прямо: документ из семи строк
 * возвращался четырнадцатью, и лишние переводы уезжали в ссылку и в черновик.
 */
function readText(root) {
  const lines = root.querySelectorAll(".line");
  if (lines.length === 0) {
    return root.innerText.replace(/\r\n?/g, "\n").replace(/\n$/, "");
  }
  return [...lines].map((line) => line.textContent).join("\n");
}

/** Смещение курсора в символах от начала документа. */
function caretOffset(root) {
  const selection = root.ownerDocument.getSelection();
  if (!selection || selection.rangeCount === 0) return 0;
  const range = selection.getRangeAt(0).cloneRange();
  range.selectNodeContents(root);
  range.setEnd(selection.getRangeAt(0).endContainer, selection.getRangeAt(0).endOffset);
  return range.toString().length;
}

/** Ставит курсор на смещение в символах. */
function setCaretOffset(root, offset) {
  const document_ = root.ownerDocument;
  const selection = document_.getSelection();
  if (!selection) return;
  const walker = document_.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let seen = 0;
  let node = walker.nextNode();
  while (node) {
    const length = node.textContent.length;
    if (seen + length >= offset) {
      const range = document_.createRange();
      range.setStart(node, Math.max(0, offset - seen));
      range.collapse(true);
      selection.removeAllRanges();
      selection.addRange(range);
      return;
    }
    seen += length;
    node = walker.nextNode();
  }
  // Смещение за концом документа: курсор в конец — это верно и после
  // форматирования, которое текст укоротило.
  const range = document_.createRange();
  range.selectNodeContents(root);
  range.collapse(false);
  selection.removeAllRanges();
  selection.addRange(range);
}

/**
 * Переводит смещение в позицию LSP: строка и колонка с нуля.
 *
 * Экспортируется ради проверки в `node`: DOM там нет, а правило перевода
 * координат — есть, и ошибка в нём уводит наведение и переход на чужое место.
 */
export function offsetToPosition(text, offset) {
  const clamped = Math.max(0, Math.min(offset, text.length));
  const before = text.slice(0, clamped);
  const line = before.split("\n").length - 1;
  const lastBreak = before.lastIndexOf("\n");
  const character = clamped - (lastBreak + 1);
  return { line, character };
}

/** Обратный перевод: позиция LSP → смещение в символах. */
export function positionToOffset(text, line, character) {
  const lines = text.split("\n");
  let offset = 0;
  for (let i = 0; i < Math.min(line, lines.length); i += 1) {
    offset += lines[i].length + 1;
  }
  return Math.min(offset + character, text.length);
}
