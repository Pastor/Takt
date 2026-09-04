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
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import test from "node:test";

import { encodeState, decodeState } from "../static/share.js";
import { offsetToPosition, positionToOffset } from "../static/editor.js";
import * as draft from "../static/draft.js";
import { Bridge, spans } from "../static/bridge.js";
import * as i18n from "../static/i18n.js";
import { inlineScripts, literalsWithText, nodesWithoutKey } from "./strings.mjs";
import { bundleOfUrl } from "../static/build.js";
import * as shell from "../static/shell.js";
import * as project from "../static/project.js";
import * as api from "../static/api.js";
import { feed } from "../static/showcase.js";

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
  // Причина возвращается КЛЮЧОМ словаря: текст строит главный поток страницы
  // (задача 10a), и второй копии словаря здесь нет.
  assert.equal(problem.key, "draft.tooBig");
  assert.equal(problem.params.limit, 64);
  assert.ok(problem.params.size > draft.LIMIT_BYTES);
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

/** Читает словарь языка с диска: `fetch` относительного пути в `node` нет. */
async function dictionary(lang) {
  const text = await readFile(new URL(`../static/i18n/${lang}.json`, import.meta.url), "utf8");
  return JSON.parse(text);
}

test("язык: словари полны — паритет ключей и подстановок", async () => {
  // ⚠️ Замер референса 2026-09-04: у него 163 ключа есть только в `ru`, и
  // непереведённое молча падает на русский. Правило проекта иное: язык либо
  // полон, либо не заведён, — и держит его эта проверка, а не дисциплина.
  const base = await dictionary(i18n.BASE);
  const names = (text) => new Set([...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]));
  for (const lang of Object.keys(i18n.LANGUAGES)) {
    if (lang === i18n.BASE) continue;
    const dict = await dictionary(lang);
    assert.deepEqual(
      Object.keys(dict).sort(),
      Object.keys(base).sort(),
      `словарь '${lang}' не равен базовому по составу ключей`
    );
    for (const key of Object.keys(base)) {
      assert.deepEqual(
        [...names(dict[key])].sort(),
        [...names(base[key])].sort(),
        `ключ '${key}' в '${lang}': другой набор подстановок`
      );
      assert.ok(dict[key].trim().length > 0, `ключ '${key}' в '${lang}' пуст`);
    }
  }
});

test("язык: список выпуска равен составу каталога словарей", async () => {
  // Язык без словаря даёт подписи-ключи; словарь без записи в списке не
  // выбрать ничем. И то и другое — тишина.
  const files = (await readdir(new URL("../static/i18n/", import.meta.url)))
    .filter((name) => name.endsWith(".json"))
    .map((name) => name.replace(/\.json$/, ""));
  assert.deepEqual(files.sort(), Object.keys(i18n.LANGUAGES).sort());
});

test("язык: каждый ключ разметки есть в словаре, и мёртвых ключей нет", async () => {
  const html = await readFile(new URL("../static/index.html", import.meta.url), "utf8");
  const scripts = await Promise.all(
    PAGE_SCRIPTS.map((name) =>
      readFile(new URL(`../static/${name}`, import.meta.url), "utf8")
    )
  );
  const base = await dictionary(i18n.BASE);

  const used = new Set();
  for (const [, key] of html.matchAll(/data-i18n="([^"]+)"/g)) used.add(key);
  for (const [, pairs] of html.matchAll(/data-i18n-attr="([^"]+)"/g)) {
    for (const pair of pairs.split(";")) used.add(pair.split(":")[1].trim());
  }
  const text = [html, ...scripts].join("\n");
  for (const [, key] of text.matchAll(/\bt\(\s*"([\w.]+)"/g)) used.add(key);
  // Ключи, которые страница строит не буквально: воркер и черновик возвращают
  // их полем `key`.
  for (const [, key] of text.matchAll(/key:\s*"([\w.]+)"/g)) used.add(key);
  // ⚠️ Подписи кнопок площадок приходят ОТ СЕРВЕРА (задача 09f-3): имён
  // площадок в коде страницы нет намеренно. Ключи берутся у него же — иначе
  // сверка объявила бы их мёртвыми и подтолкнула бы завести список в вебе.
  for (const key of await serverLabelKeys()) used.add(key);
  for (const [, a, b] of text.matchAll(/\?\s*"([\w.]+)"\s*:\s*"([\w.]+)"/g)) {
    used.add(a);
    used.add(b);
  }

  for (const key of used) {
    assert.ok(base[key], `ключ '${key}' используется, но его нет в словаре`);
  }
  const dead = Object.keys(base).filter((key) => !used.has(key));
  assert.deepEqual(dead, [], `мёртвые ключи словаря: ${dead.join(", ")}`);
});

test("язык: текста оболочки мимо словаря нет", async () => {
  // Строка, написанная в коде, не переводится никогда и не обнаруживается
  // ничем: страница выглядит рабочей, а подпись остаётся на чужом языке.
  // Исключения названы в `TEXT_EXEMPT`.
  const scripts = PAGE_SCRIPTS.filter((name) => !TEXT_EXEMPT.includes(name));
  for (const name of scripts) {
    const source = await readFile(new URL(`../static/${name}`, import.meta.url), "utf8");
    const found = literalsWithText(source);
    assert.deepEqual(
      found,
      [],
      `${name}: текст мимо словаря — ${found.map((f) => `строка ${f.line}: ${f.text}`).join("; ")}`
    );
  }
  const html = await readFile(new URL("../static/index.html", import.meta.url), "utf8");
  const inMarkup = nodesWithoutKey(html);
  assert.deepEqual(
    inMarkup,
    [],
    `index.html: текст без ключа — ${inMarkup.map((f) => `строка ${f.line}: ${f.text}`).join("; ")}`
  );
  // Встроенный скрипт разметки — такой же код страницы: строку оболочки в нём
  // не видно ни разбором тегов, ни обходом модулей.
  for (const script of inlineScripts(html)) {
    const found = literalsWithText(script.source);
    assert.deepEqual(
      found,
      [],
      `index.html, скрипт со строки ${script.line}: текст мимо словаря — ${found
        .map((f) => f.text)
        .join("; ")}`
    );
  }
});

test("язык: подстановки и падение на базовый", async () => {
  i18n.use("en", { "a.b": "hello {name}" }, { "a.b": "привет {name}", "c.d": "только база" });
  assert.equal(i18n.t("a.b", { name: "Takt" }), "hello Takt");
  // Ключа нет в выбранном языке — берётся базовый.
  assert.equal(i18n.t("c.d"), "только база");
  // Нет и там — сам ключ: пустая кнопка хуже кнопки с именем ключа.
  assert.equal(i18n.t("нет.такого"), "нет.такого");
  // Неизвестная подстановка остаётся как есть: молча съеденная выглядела бы
  // опечаткой автора словаря.
  assert.equal(i18n.t("a.b", { other: 1 }), "hello {name}");
});

test("язык: порядок выбора — сохранённый, браузер, база", () => {
  assert.equal(i18n.pick("en", ["ru-RU"]), "en", "сохранённый сильнее браузера");
  assert.equal(i18n.pick(null, ["en-GB", "ru"]), "en", "регион отбрасывается");
  assert.equal(i18n.pick(null, ["de-DE"]), i18n.BASE, "неизвестный язык — база");
  assert.equal(i18n.pick("de", ["en"]), "en", "сохранённый язык без словаря не берётся");
  assert.equal(i18n.pick(null, []), i18n.BASE);
});

/**
 * Модули страницы. Список ЯВНЫЙ: обход каталога подхватил бы и то, чего в
 * `index.html` нет, а забытый модуль остался бы без обеих проверок молча.
 */
const PAGE_SCRIPTS = [
  "account.js", "api.js", "app.js", "boot.js", "bridge.js", "build.js",
  "draft.js", "editor.js", "i18n.js", "pick.js", "project.js", "sample.js",
  "share.js", "shell.js", "showcase.js", "worker.js",
];

/**
 * Модули, которым русский текст в литералах разрешён, и почему:
 *   `sample.js` — стартовая МОДЕЛЬ, документ автора, а не оболочка;
 *   `i18n.js` — самоназвания языков (они не переводятся ни на какой язык) и
 *   отказ ЗАГРУЗКИ словаря: сообщить о нём словарём нечем — его нет.
 */
const TEXT_EXEMPT = ["sample.js", "i18n.js"];

/** Путь к собранной статике; проверки сборки без него пропускаются. */
const DIST = process.argv[3] ?? process.env.TAKT_WEB_DIST ?? null;

test("сборка: разметка ссылается только в каталог бандла", { skip: !DIST }, async () => {
  // «Содержимое задаёт адрес, адрес задаёт срок»: помеченное отпечатком живёт
  // год и `immutable`. Ссылка мимо бандла — файл, который кеш обязан считать
  // вечным, не будучи вечным, то есть молчаливая порча у всех, кто кешировал.
  const html = await readFile(join(DIST, "index.html"), "utf8");
  // ⚠️ `<base>` из разбора выброшен: он называет КОРЕНЬ адресов, а не файл, и
  // отпечатка у него быть не может. Именно он и делает остальные ссылки
  // считаемыми от корня — без него страница `/p/<id>` искала бы бандл под
  // собой (нашлось прогоном страницы, задача 09c).
  const base = /<base href="([^"]+)"/.exec(html);
  assert.ok(base, "в разметке нет корня адресов");
  assert.equal(base[1], "/", "корень адресов — не бандл и не подкаталог");
  const links = [...html.replace(/<base [^>]*>/g, "").matchAll(/(?:href|src)="([^"]+)"/g)]
    .map((m) => m[1]);
  assert.ok(links.length > 0, "в разметке нет ссылок вовсе");
  for (const link of links) {
    if (/^(https?:|data:|#)/.test(link)) continue;
    assert.match(link, /^b\/[0-9a-f]{6,}\//, `ссылка мимо каталога бандла: ${link}`);
  }
});

test("сборка: идентификатор бандла один — в адресе и в описи", { skip: !DIST }, async () => {
  // Носитель отпечатка ОДИН: страница читает свой из собственного адреса
  // (`import.meta.url`), а выложенный — из `version.json`. Разъедься они —
  // страница вечно звала бы обновиться либо не звала бы никогда.
  const version = JSON.parse(await readFile(join(DIST, "version.json"), "utf8"));
  const dirs = await readdir(join(DIST, "b"));
  assert.deepEqual(dirs, [version.bundle], "каталог бандла не равен описи");
  const html = await readFile(join(DIST, "index.html"), "utf8");
  assert.ok(html.includes(`b/${version.bundle}/`), "разметка ведёт в другой бандл");
  // Тот же разбор, которым страница узнаёт свой бандл.
  assert.equal(bundleOfUrl(`http://x/b/${version.bundle}/app.js`), version.bundle);
  assert.equal(bundleOfUrl("http://x/app.js"), null, "несобранная страница бандла не имеет");
});

test("сборка: опись модуля несёт его контрольную сумму", { skip: !DIST }, async () => {
  // Адрес `wasm/<версия>/` обещает НЕИЗМЕННОСТЬ, и выложить под ним другой
  // файл — порча у каждого, кто уже кешировал. Отказ выкладки на подмене
  // (задача 07c) стоит на этой сумме, и посчитана она обязана быть верно.
  const version = JSON.parse(await readFile(join(DIST, "version.json"), "utf8"));
  const dir = join(DIST, "wasm", version.takt_lang);
  const manifest = JSON.parse(await readFile(join(dir, "manifest.json"), "utf8"));
  const bytes = await readFile(join(dir, "takt.wasm"));
  assert.equal(manifest.sha256, createHash("sha256").update(bytes).digest("hex"));
  assert.equal(manifest.size, bytes.length);
  assert.equal(manifest.takt_lang, version.takt_lang);
  const index = JSON.parse(await readFile(join(DIST, "wasm", "index.json"), "utf8"));
  assert.equal(index.latest, version.takt_lang);
  assert.ok(index.versions.includes(version.takt_lang));

  // ⚠️ Обе версии НЕПУСТЫ. Поле `language` собиралось грепом по `lib.rs`, где
  // константа только реэкспортируется, и описи месяц несли пустую строку:
  // сервер отдавал её новому проекту, а увидеть это можно было лишь заглянув в
  // `version.json`. Пустая строка — не значение, и молчать о ней нельзя.
  for (const [where, value] of [
    ["version.json takt_lang", version.takt_lang],
    ["version.json language", version.language],
    ["manifest language", manifest.language],
  ]) {
    assert.match(value ?? "", /^\d+\.\d+\.\d+$/, `${where}: не версия — '${value}'`);
  }
});

test("сборка: текстовые файлы предсжаты", { skip: !DIST }, async () => {
  // Стенд ничего не считает на лету: модуль 3,3 МБ, и сжимать его каждому
  // первому заходу — лишняя работа. ⚠️ Описи `no-cache` предсжатию НЕ
  // подлежат: их читают ради свежести, а не ради объёма.
  const version = JSON.parse(await readFile(join(DIST, "version.json"), "utf8"));
  const must = [
    join(DIST, "index.html"),
    join(DIST, "b", version.bundle, "app.css"),
    join(DIST, "b", version.bundle, "app.js"),
    join(DIST, "wasm", version.takt_lang, "takt.wasm"),
  ];
  for (const file of must) {
    assert.ok(existsSync(`${file}.gz`), `нет предсжатого: ${file}.gz`);
  }
  assert.ok(!existsSync(join(DIST, "version.json.gz")), "опись сборки предсжата зря");
  assert.ok(!existsSync(join(DIST, "wasm", "index.json.gz")), "опись версий предсжата зря");
});

test("черновик: отложенную запись можно сделать немедленно", () => {
  // Перед перезагрузкой на новую сборку черновик обязан лечь на диск: запись,
  // отложенная на 400 мс, до перезагрузки не доживёт, и автор потеряет
  // последние набранные строки.
  let written = null;
  const save = draft.debounce((value) => (written = value), 10_000);
  save("первое");
  save("второе");
  assert.equal(written, null, "запись отложена");
  save.now();
  assert.equal(written, "второе", "записано последнее, а не первое");
  save.now();
  assert.equal(written, "второе", "повторный вызов ничего не пишет");
});

test("оболочка: ширина не выходит за окно и за наименьшую", () => {
  // Наибольшая ширина — размер окна (решение заказчика 2026-09-04): шире
  // монитора оболочки не бывает. Наименьшая — 640: уже неё две колонки кода
  // не имеют смысла.
  assert.equal(shell.clamp(2000, 1440), 1440, "шире окна оболочки не бывает");
  assert.equal(shell.clamp(100, 1440), shell.MIN_WIDTH, "уже предела не сужается");
  assert.equal(shell.clamp(900, 1440), 900, "внутри пределов — как просили");
  // ⚠️ Окно уже наименьшей ширины: предел обязан победить окно, иначе
  // оболочка схлопнется в ничто на узком мониторе.
  assert.equal(shell.clamp(700, 400), shell.MIN_WIDTH);
  assert.equal(shell.clamp(300, 400), shell.MIN_WIDTH);
});

test("оболочка: испорченная запись ширины не роняет страницу", () => {
  const bad = { getItem: () => "не число" };
  assert.equal(shell.stored(bad), null);
  assert.equal(shell.stored({ getItem: () => null }), null);
  assert.equal(shell.stored({ getItem: () => "0" }), null, "нулевая ширина — не ширина");
  assert.equal(shell.stored({ getItem: () => "900" }), 900);
  assert.equal(shell.stored({ getItem: () => { throw new Error("нет доступа"); } }), null);
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

test("страница проекта: адрес разбирается вместе с префиксом", () => {
  // ⚠️ Разбирается ПУТЬ, а не адрес целиком: сервис умеет стоять за обратным
  // прокси под префиксом, и `/takt/p/<id>` — тот же случай. Ошибись разбор — и
  // страница молча откроется черновиком вместо проекта.
  assert.equal(project.idInPath("/p/AbCd-_12"), "AbCd-_12");
  assert.equal(project.idInPath("/takt/p/AbCd-_12"), "AbCd-_12");
  assert.equal(project.idInPath("/p/AbCd-_12/"), "AbCd-_12");
  assert.equal(project.idInPath("/"), null, "корень — не проект");
  assert.equal(project.idInPath("/p/"), null, "адрес без идентификатора");
  assert.equal(project.idInPath("/p/чужое имя"), null, "не идентификатор");

  // Корень API считается ОТ ПУТИ страницы: относительный адрес от документа
  // `/p/<id>` увёл бы запрос в `/p/api/...` — нашлось бы только в браузере.
  assert.equal(project.apiRoot("/p/AbCd-_12"), "/");
  assert.equal(project.apiRoot("/takt/p/AbCd-_12"), "/takt/");
  assert.equal(project.apiRoot("/takt/"), "/takt/");
});

test("страница проекта: читается активный файл и сценарий рядом", async () => {
  const asked = [];
  const answers = {
    "/api/projects/AbCd": {
      id: "AbCd",
      name: "Термореле",
      owner: "ivan",
      visibility: "public",
      takt_lang: "0.58.0",
      main_file: "model.takt",
      files: [
        { name: "other.takt", kind: "takt", size_bytes: 3 },
        { name: "model.takt", kind: "takt", size_bytes: 9 },
        { name: "run.json", kind: "scenario", size_bytes: 2 },
      ],
    },
    "/api/projects/AbCd/files/model.takt": { name: "model.takt", text: MODEL },
    "/api/projects/AbCd/files/run.json": { name: "run.json", text: "[]" },
  };
  const get = async (url) => {
    asked.push(url);
    const body = answers[url];
    if (!body) return { ok: false, status: 404, json: async () => ({}) };
    return { ok: true, status: 200, json: async () => body };
  };

  const opened = await project.read("AbCd", "/", get);
  assert.equal(opened.source, MODEL, "открылся активный файл, а не первый по имени");
  assert.equal(opened.scenario, "[]");
  assert.equal(opened.owner, "ivan", "автор назван: чужая модель не выглядит своей");
  assert.equal(opened.version, "0.58.0", "версия модуля — свойство проекта (A5)");
  // Лишних обращений нет: витрина бывает длинной, и читать всё подряд незачем.
  assert.equal(asked.length, 3, `обращений ${asked.length}: ${asked.join(", ")}`);
});

test("страница проекта: закрытый отвечает названным отказом", async () => {
  // ⚠️ Отказ поднимается КЛЮЧОМ словаря, а не текстом: текст оболочки строит
  // одна точка — главный поток страницы (задача 10a).
  const get = async () => ({ ok: false, status: 404, json: async () => ({}) });
  await assert.rejects(
    () => project.read("AbCd", "/", get),
    (error) => error.key === "project.notFound",
  );

  const broken = async () => {
    throw new Error("сеть");
  };
  await assert.rejects(
    () => project.read("AbCd", "/", broken),
    (error) => error.key === "project.failed",
  );
});

test("язык: список модулей страницы полон", async () => {
  // ⚠️ Модуль, забытый в списке, ускользает от ОБЕИХ проверок разом — и от
  // сверки ключей, и от поиска текста мимо словаря. Список тем самым перестаёт
  // быть списком, оставаясь на вид полным.
  const files = (await readdir(new URL("../static/", import.meta.url)))
    .filter((name) => name.endsWith(".js"))
    .sort();
  assert.deepEqual(PAGE_SCRIPTS.slice().sort(), files, "список модулей отстал от каталога");
});

test("черновик v2: круговой рейс ключуется проектом и файлом", () => {
  const storage = memoryStorage();
  draft.saveFile(storage, {
    project: "p1", file: "model.takt", revision: 3, source: "первый", savedAt: 100,
  });
  draft.saveFile(storage, {
    project: "p2", file: "model.takt", revision: 7, source: "второй", savedAt: 200,
  });
  // ⚠️ Одинаковое имя файла в двух проектах — не один черновик: ключ несёт и
  // проект, иначе работа над одним затирала бы работу над другим.
  assert.equal(draft.loadFile(storage, "p1", "model.takt").source, "первый");
  assert.equal(draft.loadFile(storage, "p2", "model.takt").source, "второй");
  // Ревизия хранится вместе с текстом: без неё при возвращении нельзя сказать,
  // разошёлся ли черновик с сервером.
  assert.equal(draft.loadFile(storage, "p1", "model.takt").revision, 3);
  assert.equal(draft.loadFile(storage, "p3", "model.takt"), null, "чужого нет");

  draft.clearFile(storage, "p1", "model.takt");
  assert.equal(draft.loadFile(storage, "p1", "model.takt"), null, "успешное сохранение стирает");
  assert.equal(draft.loadFile(storage, "p2", "model.takt").source, "второй", "соседний цел");

  // Безымянный буфер живёт своей жизнью: им пользуется тот, кто не входил.
  draft.save(storage, { source: "буфер" });
  draft.saveFile(storage, { project: "p9", file: "a.takt", source: "проектный" });
  assert.equal(draft.load(storage).source, "буфер");
  assert.equal(draft.loadFile(storage, "p9", "a.takt").source, "проектный");
});

test("черновик v2: карта не растёт без предела", () => {
  // ⚠️ Предел нужен не ради места, а ради предела: `localStorage` кончается
  // молча и кончается НА ЗАПИСИ — то есть в момент сохранения работы.
  const storage = memoryStorage();
  for (let i = 0; i < draft.DRAFTS_KEPT + 5; i += 1) {
    draft.saveFile(storage, {
      project: "p", file: `f${i}.takt`, source: `текст ${i}`, savedAt: 1000 + i,
    });
  }
  const kept = JSON.parse(storage.getItem("takt.draft.v2"));
  assert.equal(Object.keys(kept).length, draft.DRAFTS_KEPT);
  // Уходят СТАРШИЕ: к черновику, до которого не возвращались двадцать файлов
  // назад, автор уже не вернётся.
  assert.equal(draft.loadFile(storage, "p", "f0.takt"), null, "старший вытеснен");
  assert.ok(draft.loadFile(storage, "p", `f${draft.DRAFTS_KEPT + 4}.takt`), "младший на месте");
});

test("сессия: пара переживает перезагрузку, а выход её забывает", async () => {
  const storage = memoryStorage();
  const answers = {
    "/api/token": { access_token: "A1", refresh_token: "R1" },
    "/api/me": { id: "u1", login: "ivan", role: "user" },
    "/api/revoke": null,
  };
  const get = async (url) => ({
    ok: true,
    status: answers[url] === null ? 204 : 200,
    json: async () => answers[url],
  });
  api.configure({ root: "/", fetch: get, storage });
  assert.equal(api.signed(), false, "без пары мы никто");
  const me = await api.signIn("ivan", "пароль-пароль");
  assert.equal(me.login, "ivan");
  assert.ok(storage.getItem("takt.session.v1"), "пара записана");

  // Перезагрузка страницы: новый клиент поднимает пару из хранилища.
  api.configure({ root: "/", fetch: get, storage });
  assert.equal(api.signed(), true, "после перезагрузки вход сохранён");
  assert.equal(api.who().login, "ivan");

  await api.signOut();
  assert.equal(api.signed(), false);
  assert.equal(storage.getItem("takt.session.v1"), null, "пара забыта");
});

test("сессия: просроченный доступ обновляется ОДИН раз на все запросы", async () => {
  // ⚠️ refresh одноразовый: две параллельные попытки гасят семейство целиком,
  // то есть выкидывают автора ровно тогда, когда он сохраняет работу.
  const storage = memoryStorage();
  storage.setItem(
    "takt.session.v1",
    JSON.stringify({ access: "старый", refresh: "R1", login: "ivan", role: "user" })
  );
  let refreshes = 0;
  let fresh = false;
  const get = async (url, options) => {
    if (url === "/api/token") {
      refreshes += 1;
      fresh = true;
      return { ok: true, status: 200, json: async () => ({ access_token: "A2", refresh_token: "R2" }) };
    }
    if (!fresh) return { ok: false, status: 401, json: async () => ({ error: "unauthorized" }) };
    assert.equal(options.headers.authorization, "Bearer A2", "запрос повторён свежим токеном");
    return { ok: true, status: 200, json: async () => [] };
  };
  api.configure({ root: "/", fetch: get, storage });
  await Promise.all([api.projects(), api.projects(), api.projects()]);
  assert.equal(refreshes, 1, `обновлений пары ${refreshes}, а должно быть одно`);
});

test("сессия: отказ приходит кодом и числами, а не разобранным текстом", async () => {
  const storage = memoryStorage();
  const get = async () => ({
    ok: false,
    status: 409,
    json: async () => ({
      error: "revision_conflict",
      message: "проект изменился: у вас ревизия 1, у проекта 2",
      seen: 1,
      revision: 2,
    }),
  });
  api.configure({ root: "/", fetch: get, storage });
  await assert.rejects(
    () => api.write("p1", "model.takt", "текст", 1),
    (error) => {
      // ⚠️ Числа взяты ПОЛЯМИ: разбирай их страница из сообщения — текст отказа
      // стал бы частью протокола и перестал бы переводиться.
      assert.equal(error.code, "revision_conflict");
      assert.equal(error.seen, 1);
      assert.equal(error.revision, 2);
      assert.equal(error.key, "api.failed", "ключ один на все коды сервера");
      return true;
    }
  );
});

test("разметка: каждый узел с именем найден страницей", async () => {
  // ⚠️ Список имён в `cache()` — второй носитель разметки, и отстаёт он молча:
  // забытый `signout` дал пустую страницу с сообщением «модуль не загружен»,
  // хотя модуль был загружен (нашлось прогоном страницы, задача 09e).
  const html = await readFile(new URL("../static/index.html", import.meta.url), "utf8");
  const app = await readFile(new URL("../static/app.js", import.meta.url), "utf8");
  const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1]);
  assert.ok(ids.length > 0, "в разметке нет именованных узлов");
  const cached = new Set(
    [...app.matchAll(/"([a-z][a-z0-9-]*)"/g)].map((m) => m[1])
  );
  const missing = ids.filter((id) => !cached.has(id));
  assert.deepEqual(missing, [], `узлы разметки не найдены страницей: ${missing.join(", ")}`);
});

test("разметка: скрытый узел действительно скрыт", async () => {
  // ⚠️ `display: flex` СИЛЬНЕЕ `hidden`: узел с таким классом остаётся на
  // экране, сколько его ни прячь. Класс ловился прогоном страницы дважды —
  // теперь его ловит машина: у каждого класса, которым помечен скрываемый
  // узел, обязано быть правило `[hidden]`, если этот класс задаёт `display`.
  const html = await readFile(new URL("../static/index.html", import.meta.url), "utf8");
  const css = await readFile(new URL("../static/app.css", import.meta.url), "utf8");
  const hiddenClasses = new Set();
  for (const [, tag] of html.matchAll(/<([^>]*\bhidden\b[^>]*)>/g)) {
    const classes = /class="([^"]+)"/.exec(tag);
    if (!classes) continue;
    for (const name of classes[1].split(/\s+/)) if (name) hiddenClasses.add(name);
  }
  const unprotected = [];
  for (const name of hiddenClasses) {
    const sets = new RegExp(`\\.${name}\\s*(,[^{]*)?\\{[^}]*display:`).test(css);
    const guards = css.includes(`.${name}[hidden]`);
    if (sets && !guards) unprotected.push(name);
  }
  assert.deepEqual(unprotected, [], `класс задаёт display и не гасится: ${unprotected.join(", ")}`);
});

test("язык: подписи площадок объявляет сервер, а словарь их знает", async () => {
  // ⚠️ Реестр сверяется ДВУМЯ сторонами: сервер объявляет ключ подписи, словарь
  // даёт текст. Заведи площадку на сервере без записи в словаре — кнопка
  // показала бы читателю служебный ключ, и увидел бы это он, а не гейт.
  const keys = await serverLabelKeys();
  assert.ok(keys.length >= 3, `сервер объявил подписей: ${keys.length}`);
  for (const lang of Object.keys(i18n.LANGUAGES)) {
    const dictionary = JSON.parse(
      await readFile(new URL(`../static/i18n/${lang}.json`, import.meta.url), "utf8")
    );
    for (const key of keys) {
      assert.ok(dictionary[key], `в словаре '${lang}' нет подписи площадки '${key}'`);
    }
  }
});

test("страница не знает имён площадок", async () => {
  // ⚠️ Приём тот же, что «нет списка ключевых слов Takt в вебе»: свой список
  // площадок разошёлся бы с настройкой стенда молча, и кнопка вела бы в никуда.
  for (const name of PAGE_SCRIPTS) {
    const source = await readFile(new URL(`../static/${name}`, import.meta.url), "utf8");
    const code = source
      .split("\n")
      .filter((line) => !line.trim().startsWith("//"))
      .join("\n");
    const found = /["'`](yandex|vk|mail_ru)["'`]/i.exec(code);
    assert.equal(found, null, `${name}: имя площадки в коде — ${found?.[0]}`);
  }
});

/** Ключи подписей площадок, объявленные сервером. */
async function serverLabelKeys() {
  const source = await readFile(
    new URL("../server/src/oauth/api.rs", import.meta.url),
    "utf8"
  );
  return [...source.matchAll(/label:\s*"([\w.]+)"/g)].map((match) => match[1]);
}

test("витрина и архив: страница просит у сервера ровно то, что нужно", async () => {
  const storage = memoryStorage();
  const asked = [];
  const get = async (url, options) => {
    asked.push(`${options?.method ?? "GET"} ${url}`);
    if (url.includes("/api/public")) {
      return {
        ok: true,
        status: 200,
        json: async () => ({ items: [{ id: "p1", name: "Термореле", owner: "ivan" }] }),
      };
    }
    if (url.includes("/archive")) {
      return { ok: true, status: 200, arrayBuffer: async () => new Uint8Array([80, 75]).buffer };
    }
    return { ok: true, status: 201, json: async () => ({ id: "p2", name: "Копия" }) };
  };
  api.configure({ root: "/", fetch: get, storage });

  // ⚠️ Витрина спрашивается БЕЗ токена: открытый проект открыт и для того, у
  // кого учётной записи нет вовсе.
  const page = await api.showcase("термореле", null);
  assert.equal(page.items[0].name, "Термореле");
  assert.ok(asked[0].includes("q=%D1%82"), `запрос без слова поиска: ${asked[0]}`);

  // Курсор уходит обратно КАК ЕСТЬ: своей постраничности у страницы нет.
  await api.showcase(null, "cursor-1");
  assert.ok(asked[1].endsWith("cursor=cursor-1"), asked[1]);

  // ⚠️ Архив забирается ЗАПРОСОМ, а не ссылкой: у закрытого проекта он требует
  // токена, а обычная ссылка заголовков не несёт.
  const bytes = await api.archive("p1", "c");
  assert.equal(new Uint8Array(bytes)[0], 80, "пришли не байты архива");
  assert.ok(asked[2].includes("/api/projects/p1/archive?target=c"), asked[2]);

  const created = await api.importArchive(new Uint8Array([80, 75]).buffer);
  assert.equal(created.name, "Копия");
  assert.ok(asked[3].startsWith("POST /api/projects/import"), asked[3]);
});

test("витрина: следующая страница просится курсором сервера и тем же словом", async () => {
  // ⚠️ Предмет — ЛЕНТА, а не разметка: где остановиться, знает `showcase.js`,
  // и это правило проверяется без браузера.
  const asked = [];
  const pages = {
    null: { items: [{ id: "p1" }, { id: "p2" }], next_cursor: "c1" },
    c1: { items: [{ id: "p3" }], next_cursor: null },
  };
  const ask = async (query, cursor) => {
    asked.push([query, cursor]);
    return pages[cursor ?? "null"];
  };
  const lane = feed(ask);

  assert.equal(lane.hasMore(), false, "до первого запроса продолжения нет");
  const first = await lane.first("термореле");
  assert.deepEqual(first.map((item) => item.id), ["p1", "p2"]);
  assert.ok(lane.hasMore(), "сервер дал курсор, а лента о нём забыла");

  const second = await lane.next();
  assert.deepEqual(second.map((item) => item.id), ["p3"]);
  // ⚠️ Слово поиска едет со СЛЕДУЮЩЕЙ страницей: курсор задаёт место, а не
  // отбор, и без слова читатель получил бы под своим поиском всю витрину.
  assert.deepEqual(asked[1], ["термореле", "c1"], `спрошено ${JSON.stringify(asked[1])}`);
  assert.equal(lane.hasMore(), false, "страница без курсора — последняя");

  // За последней страницей не ходят: курсора нет, и запрос был бы впустую.
  assert.deepEqual(await lane.next(), []);
  assert.equal(asked.length, 2, `лишний запрос: ${JSON.stringify(asked)}`);
});

test("витрина: новый поиск начинается с первой страницы", async () => {
  // ⚠️ Унесённый от прежнего поиска курсор отдал бы читателю чужую страницу:
  // место в одной выдаче ничего не значит в другой.
  const asked = [];
  const ask = async (query, cursor) => {
    asked.push([query, cursor]);
    return { items: [{ id: "p1" }], next_cursor: "c1" };
  };
  const lane = feed(ask);
  await lane.first("термореле");
  await lane.next();
  await lane.first("насос");
  assert.deepEqual(asked[2], ["насос", null], `спрошено ${JSON.stringify(asked[2])}`);

  // Пустое слово — не слово: список без отбора спрашивается без параметра.
  await lane.first("");
  assert.deepEqual(asked[3], [null, null], `спрошено ${JSON.stringify(asked[3])}`);
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
