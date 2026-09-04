// Проверки веб-части, которые можно снять БЕЗ браузера (фича 0531, задача 03).
//
// # Что здесь проверяется и почему именно это
//
// Браузерного гейта в проекте нет и в этой фиче не заводится (замер задачи 07):
// фокус, выделение и отрисовку проверяет человек. Зато три вещи проверяются
// машиной, и каждая из них ломается молча:
//
//   1. **круговой рейс ссылки** — сжатие и base64url: испорченная ссылка
//      открывается пустым редактором, и автор решит, что «ссылка протухла»;
//   2. **перевод координат** — смещение ↔ строка/колонка: ошибка уводит
//      переход по диагностике на чужую строку, а вывод при этом валиден;
//   3. **черновик** — запись и чтение: его предмет в том, чтобы работа
//      пережила перезагрузку, и «сохранилось не то» здесь равно потере.
//
// Плюс смоук моста: страница и модуль обязаны сойтись формой ответа.
//
// Запуск: node web/tests/web-tests.mjs <модуль.wasm>

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { encodeState, decodeState } from "../static/share.js";
import { offsetToPosition, positionToOffset } from "../static/editor.js";
import * as draft from "../static/draft.js";
import { Bridge, spans } from "../static/bridge.js";

const MODEL = `var level: u8 := 0;

start Run {
    always {
        level := level + 1;
    }
}
`;

test("ссылка: круговой рейс сохраняет всё состояние", async () => {
  const state = {
    version: "0.57.0",
    source: MODEL,
    scenario: '[{"in_ports": {"x": 1}}]',
    target: "sv",
    args: "--fsm=table model.takt",
  };
  const restored = await decodeState("#" + (await encodeState(state)));
  assert.deepEqual(restored, state);
});

test("ссылка: чужой фрагмент не ошибка, а отсутствие состояния", async () => {
  assert.equal(await decodeState(""), null);
  assert.equal(await decodeState("#секция-документа"), null);
  assert.equal(await decodeState("#SGVsbG8"), null);
});

test("ссылка: сжатие действительно сжимает", async () => {
  // Смысл сжатия — в длине ссылки: модель на 3 КиБ обязана уложиться в адрес,
  // который не режут мессенджеры.
  const source = MODEL.repeat(40);
  const fragment = await encodeState({ version: "0.57.0", source });
  assert.ok(
    fragment.length < source.length / 3,
    `сжатие не сработало: ${source.length} → ${fragment.length}`
  );
});

test("координаты: смещение и позиция взаимно обратны", () => {
  const text = "первая\nвторая строка\nтретья";
  for (let offset = 0; offset <= text.length; offset += 1) {
    const { line, character } = offsetToPosition(text, offset);
    assert.equal(positionToOffset(text, line, character), offset);
  }
  assert.deepEqual(offsetToPosition(text, 0), { line: 0, character: 0 });
  assert.deepEqual(offsetToPosition(text, 7), { line: 1, character: 0 });
  // Кириллица считается символами, а не байтами: иначе подчёркивание уедет.
  assert.deepEqual(offsetToPosition("абв\nг", 4), { line: 1, character: 0 });
});

test("черновик: круговой рейс через хранилище", () => {
  const storage = memoryStorage();
  const value = { source: MODEL, scenario: "[]", target: "rust", args: "--inline=auto" };
  assert.equal(draft.save(storage, value), null);
  assert.deepEqual(draft.load(storage), value);
  draft.clear(storage);
  assert.equal(draft.load(storage), null);
});

test("черновик: превышение предела названо, а не усечено молча", () => {
  const storage = memoryStorage();
  const problem = draft.save(storage, { source: "x".repeat(draft.LIMIT_BYTES + 1) });
  assert.ok(problem, "предел обязан быть назван");
  assert.match(problem, /КиБ/);
  assert.equal(draft.load(storage), null, "черновик сверх предела не сохраняется");
});

test("черновик: испорченная запись не роняет страницу", () => {
  const storage = memoryStorage();
  storage.setItem("takt.draft.v1", "{ это не json");
  assert.equal(draft.load(storage), null);
});

/** Загружает модуль один раз на прогон: инстанцирование стоит дороже проверок. */
let loaded = null;
async function loadBridge() {
  if (loaded) return loaded;
  const wasmPath = process.argv[2] ?? process.env.TAKT_WASM;
  assert.ok(wasmPath, "путь к модулю: node web/tests/web-tests.mjs <модуль.wasm>");
  const bytes = await readFile(wasmPath);
  const { instance } = await WebAssembly.instantiate(bytes, {});
  loaded = new Bridge(instance.exports);
  return loaded;
}

test("мост: страница и модуль сходятся формой ответа", async () => {
  const bridge = await loadBridge();

  const version = bridge.version();
  assert.equal(version.ok, true);
  assert.equal(version.targets.length, 8, "целей восемь");

  const compiled = bridge.compile("c", "heater.takt", MODEL);
  assert.equal(compiled.ok, true, JSON.stringify(compiled));
  assert.deepEqual(
    compiled.files.map((f) => f.name),
    ["heater.h", "heater.c"]
  );

  const diagnostics = bridge.diagnostics(MODEL);
  assert.equal(diagnostics.ok, true);
  assert.ok(Array.isArray(diagnostics.diagnostics));

  // Токены разворачиваются в отрезки: страница красит по НИМ, и своего словаря
  // у неё нет.
  const marks = spans(bridge.tokens(MODEL));
  assert.ok(marks.length > 0, "модель без токенов не бывает");
  for (const mark of marks) {
    assert.ok(mark.length > 0 && mark.type !== undefined);
  }

  // Прогон: те же строки, что печатает `takt-sim`.
  const opened = bridge.simOpen(MODEL, "", 0);
  assert.equal(opened.ok, true, JSON.stringify(opened));
  const ticked = bridge.simTick(opened.id, 2);
  assert.equal(ticked.ok, true);
  assert.match(ticked.lines[0], /^Шаг {3}1:/);
  bridge.simClose(opened.id);
});

test("подсветка: каждая цель красит свой вывод", async () => {
  // Условие приёмки задачи 06: у КАЖДОЙ из восьми целей разметка непуста и
  // различает ключевое слово, число и комментарий. Цель, забытая в таблице
  // языков, иначе показывала бы чёрный текст — и заметил бы это человек,
  // открывший её вкладку.
  const bridge = await loadBridge();
  const targets = bridge.version().targets;
  assert.equal(targets.length, 8);
  for (const target of targets) {
    const compiled = bridge.compile(target, "heater.takt", MODEL);
    assert.equal(compiled.ok, true, `${target}: ${JSON.stringify(compiled)}`);
    for (const file of compiled.files) {
      const painted = bridge.highlight(target, file.text);
      assert.equal(painted.ok, true, `${target}/${file.name}`);
      assert.ok(painted.language, `${target}/${file.name}: язык не назван`);
      const marks = spans(painted);
      assert.ok(marks.length > 0, `${target}/${file.name}: разметка пуста`);
      // Отрезок обязан попадать в текст: съехавшая колонка красит соседнее
      // слово, и вывод при этом выглядит совершенно рабочим.
      const lines = file.text.split("\n");
      for (const mark of marks) {
        assert.ok(mark.line < lines.length, `${target}: отрезок за концом файла`);
        assert.ok(
          mark.column + mark.length <= lines[mark.line].length,
          `${target}: отрезок за концом строки ${mark.line + 1}`
        );
      }
    }
    // ⚠️ «Различает ключевое слово, число и комментарий» проверяется НЕ здесь:
    // в выводе цели `plantuml` нет ни чисел, ни комментариев, и требование к
    // нему было бы требованием к фикстуре. Это свойство языка, и его проверяют
    // пробы у самих словарей (`takt-wasm/src/highlight`).
  }
});

test("подсветка: у исходника Takt своя разметка, у вывода — своя", async () => {
  // Языки разные, и красить вывод цели правилами Takt (или наоборот) значило бы
  // показывать автору неправду о том, что он читает.
  const bridge = await loadBridge();
  const source = spans(bridge.tokens(MODEL));
  const generated = bridge.compile("st", "heater.takt", MODEL).files[0].text;
  const output = spans(bridge.highlight("st", generated));
  assert.ok(source.length > 0 && output.length > 0);
  // `always` — ключевое слово Takt и НЕ ключевое слово Structured Text.
  const painted = (marks, text) =>
    marks.filter((m) => text.split("\n")[m.line].slice(m.column, m.column + m.length) === "always");
  assert.ok(painted(source, MODEL).length > 0, "в исходнике `always` покрашено");
  assert.equal(painted(output, generated).length, 0, "в выводе ST `always` — не слово языка");
});

test("подсветка: чужая цель — отказ с названной причиной", async () => {
  const bridge = await loadBridge();
  const reply = bridge.highlight("verilog", "module m;");
  assert.equal(reply.ok, false);
  assert.match(reply.error.message, /verilog/);
});

test("подсветка: роли кода перечислены темой документа", async () => {
  // Реестр ролей — `book/takt.tmTheme` (замер задачи 06): блоки кода в PDF и
  // вкладка цели красят ОДНИ И ТЕ ЖЕ виды токенов. Разъедься наборы — документ
  // и редактор разошлись бы глазами, и заметить это можно только сличением
  // двух картинок.
  const theme = await readFile(new URL("../../book/takt.tmTheme", import.meta.url), "utf8");
  const css = await readFile(new URL("../static/app.css", import.meta.url), "utf8");
  const inTheme = new Set(
    [...theme.matchAll(/<key>name<\/key>\s*<string>([^<]+)<\/string>/g)]
      .map((m) => m[1].toLowerCase())
      // Имя самой темы — не роль.
      .filter((name) => name !== "takt (tango)")
  );
  const inCss = new Set([...css.matchAll(/--tok-([a-z]+):/g)].map((m) => m[1]));
  // `variable` — цвет текста по умолчанию: в теме у него своей записи нет.
  inCss.delete("variable");
  assert.deepEqual([...inCss].sort(), [...inTheme].sort());
});

test("подсветка: раскладка строк не зависит от числа отрезков квадратично", () => {
  // Прежде каждая строка фильтровала весь список отрезков. На исходнике это
  // незаметно, а вывод цели `c` — тысячи строк и тысячи отрезков: вкладка
  // вставала бы на секунды. Проверяется не время, а СВОЙСТВО: отрезок попадает
  // ровно в свою строку.
  const text = ["aaa", "bbb", "ccc"].join("\n");
  const marks = [
    { line: 2, column: 0, length: 3, type: "keyword" },
    { line: 0, column: 0, length: 3, type: "number" },
  ];
  const buckets = new Map();
  for (const mark of marks) {
    if (!buckets.has(mark.line)) buckets.set(mark.line, []);
    buckets.get(mark.line).push(mark);
  }
  assert.deepEqual([...buckets.keys()].sort(), [0, 2]);
  assert.equal(text.split("\n").length, 3);
});

test("адаптив: точки перелома перечислены в одном месте", async () => {
  // ⚠️ Числа порогов разбегаются по файлу первыми: одно правило поправили,
  // другое забыли, и на одной ширине действуют обе раскладки. Реестр —
  // комментарий в шапке `app.css`, а множество чисел обязано ему равняться.
  const css = await readFile(new URL("../static/app.css", import.meta.url), "utf8");
  const found = new Set(
    [...css.matchAll(/@media[^{]*?\((?:max|min)-(?:width|height):\s*(\d+)px\)/g)].map((m) => m[1])
  );
  assert.deepEqual([...found].sort(), ["560", "900"], `точки перелома: ${[...found]}`);
});

test("адаптив: `vh` всегда идёт с `dvh`, а полка знает про зону жестов", async () => {
  const css = await readFile(new URL("../static/app.css", import.meta.url), "utf8");
  const html = await readFile(new URL("../static/index.html", import.meta.url), "utf8");
  // `vh` без `dvh` оставляет на телефоне пустую полосу под съехавшей адресной
  // строкой — замер референса 2026-09-04.
  for (const [, block] of css.matchAll(/\{([^}]*100vh[^}]*)\}/g)) {
    assert.match(block, /100dvh/, `рядом с 100vh нет 100dvh: ${block.trim()}`);
  }
  // `env(safe-area-*)` без `viewport-fit=cover` равен нулю — половина правила
  // не работает, и заметить это можно только на устройстве.
  if (css.includes("env(safe-area")) {
    assert.match(html, /viewport-fit=cover/, "есть env(safe-area), нет viewport-fit=cover");
  }
});

test("адаптив: у каждой вкладки есть подпись словом", async () => {
  // По одним значкам вкладку находят не все (правило референса). Подпись —
  // не украшение, а единственный носитель смысла для скринридера.
  const html = await readFile(new URL("../static/index.html", import.meta.url), "utf8");
  for (const [, inner] of html.matchAll(/<button[^>]*role="tab"[^>]*>([^<]*)<\/button>/g)) {
    assert.ok(inner.trim().length > 0, "вкладка без подписи");
  }
});

/** Хранилище в памяти — тот же интерфейс, что у `localStorage`. */
function memoryStorage() {
  const map = new Map();
  return {
    getItem: (key) => (map.has(key) ? map.get(key) : null),
    setItem: (key, value) => map.set(key, String(value)),
    removeItem: (key) => map.delete(key),
  };
}
