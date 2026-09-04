// Вход, список проектов и явное сохранение (фича 0531, задача 09e).
//
// # Чего здесь нет
//
// Правил доступа: что автор может с проектом, говорит сервер словом (`level`),
// и страница только показывает. Второй список правил разошёлся бы с первым
// молча — и разошёлся бы в сторону «кнопка есть, а сохранить нельзя».
//
// # Почему сохранение ЯВНОЕ
//
// Не на каждую букву (проработка §7): сервер не должен видеть недописанное, а
// ревизия конфликта имеет смысл только у осмысленной записи. Несохранённое при
// этом не теряется — его держит черновик `v2`, ключуемый проектом и файлом.
//
// # Конфликт
//
// Расхождение ревизий показывается **обеими датами и обоими числами**, выбор —
// за автором: «перечитать» либо «перезаписать». Молчаливого выбора нет ни в
// одну сторону: перезаписать чужую работу и потерять свою — одинаково плохо.

import * as api from "./api.js";
import * as draft from "./draft.js";
import { t } from "./i18n.js";

/**
 * Как называется первый файл нового проекта.
 *
 * Имя судится сервером как имя модели (0195): латиница, цифры, `_`, `-` и
 * расширение. Переименование файла — задача не этой страницы.
 */
const DEFAULT_FILE = "model.takt";

/** Что открыто и чем это можно править. */
const state = {
  /** Метаданные открытого проекта либо `null`. */
  project: null,
  /** Имя открытого файла. */
  file: null,
  /** Ревизия проекта на момент чтения файла. */
  revision: null,
  /** Мой уровень доступа к открытому проекту. */
  level: "none",
  /** Ждущий решения конфликт: `{seen, actual, text}`. */
  conflict: null,
};

/** Узлы страницы и обратные вызовы, которые даёт `app.js`. */
let dom = null;
let host = null;

/**
 * Подключает панель.
 *
 * @param {object} nodes узлы страницы
 * @param {{source: () => string, scenario: () => string,
 *          open: (state: object) => void, say: (text: string, kind: string) => void}} callbacks
 */
export function attach(nodes, callbacks) {
  dom = nodes;
  host = callbacks;
  dom.account.addEventListener("click", () => toggle());
  dom.signin.addEventListener("click", () => enter(api.signIn));
  dom.signup.addEventListener("click", () => enter(api.register));
  dom.signout.addEventListener("click", () => leave());
  dom.newproject.addEventListener("click", () => make());
  dom.save.addEventListener("click", () => save());
  dom.reread.addEventListener("click", () => resolveConflict("reread"));
  dom.overwrite.addEventListener("click", () => resolveConflict("overwrite"));
  dom.projects.addEventListener("click", (event) => {
    const row = event.target.closest("[data-project]");
    if (row) openProject(row.dataset.project);
  });
  dom.files.addEventListener("click", (event) => {
    const row = event.target.closest("[data-file]");
    if (row) openFile(state.project?.id, row.dataset.file);
  });
  // Ctrl+S / Cmd+S — как в редакторе на машине; браузерное «сохранить
  // страницу» здесь заведомо не то, чего хочет автор.
  window.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      save();
    }
  });
  refresh();
}

/** Открыт ли файл проекта (а не безымянный буфер). */
export function editing() {
  return state.project !== null && state.file !== null;
}

/** Записывает черновик открытого файла. */
export function keepDraft(source, scenario) {
  if (!editing()) return null;
  return draft.saveFile(localStorage, {
    project: state.project.id,
    file: state.file,
    revision: state.revision,
    source,
    scenario,
  });
}

/** Показывает или прячет панель. */
function toggle(force) {
  const show = force ?? dom.panel.hidden;
  dom.panel.hidden = !show;
  if (show && api.signed()) list();
}

/** Вход или регистрация: обе ручки отвечают одинаково. */
async function enter(how) {
  const login = dom.login.value.trim();
  const password = dom.password.value;
  if (!login || !password) {
    host.say(t("account.needBoth"), "warning");
    return;
  }
  try {
    const me = await how(login, password);
    dom.password.value = "";
    host.say(t("account.hello", { login: me.login }), "ok");
    refresh();
    await list();
  } catch (error) {
    host.say(text(error), "error");
  }
}

/** Выход: сессия забывается, открытый проект закрывается. */
async function leave() {
  await api.signOut();
  state.project = null;
  state.file = null;
  state.revision = null;
  state.level = "none";
  refresh();
}

/** Заводит проект с именем из поля. */
async function make() {
  const name = dom.newname.value.trim();
  if (!name) {
    host.say(t("account.needName"), "warning");
    return;
  }
  try {
    const created = await api.create(name);
    dom.newname.value = "";
    await list();
    await openProject(created.id);
  } catch (error) {
    host.say(text(error), "error");
  }
}

/** Наполняет список проектов. */
async function list() {
  try {
    const rows = await api.projects();
    dom.projects.replaceChildren();
    for (const row of rows) {
      const node = document.createElement("div");
      node.className = "row";
      node.dataset.project = row.id;
      node.textContent = t("account.projectRow", {
        name: row.name,
        level: levelName(row.level),
      });
      dom.projects.appendChild(node);
    }
    if (rows.length === 0) {
      const empty = document.createElement("div");
      empty.className = "row row-ok";
      empty.textContent = t("account.noProjects");
      dom.projects.appendChild(empty);
    }
  } catch (error) {
    host.say(text(error), "error");
  }
}

/** Перечитывает состав файлов открытого проекта. */
async function openProjectFiles(id) {
  const opened = await api.project(id);
  state.project = opened;
  state.level = opened.level;
  dom.files.replaceChildren();
  for (const file of opened.files) {
    const node = document.createElement("div");
    node.className = "row";
    node.dataset.file = file.name;
    node.textContent = file.name;
    dom.files.appendChild(node);
  }
}

/** Открывает проект: состав файлов и активный файл. */
async function openProject(id) {
  try {
    await openProjectFiles(id);
    const opened = state.project;
    const first = opened.main_file ?? opened.files[0]?.name ?? null;
    if (first) {
      await openFile(id, first);
    } else {
      // ⚠️ У нового проекта файлов ещё нет, но писать автор начинает СРАЗУ.
      // Не назови мы файл здесь — кнопки сохранения не было бы вовсе, и первый
      // же набранный текст оставался бы только в черновике (нашлось прогоном
      // страницы).
      state.file = DEFAULT_FILE;
      state.revision = null;
      hideConflict();
      refresh();
    }
  } catch (error) {
    host.say(text(error), "error");
  }
}

/**
 * Открывает файл проекта.
 *
 * ⚠️ Черновик сильнее сервера **не молча**: разошлись — показываются обе даты
 * и обе ревизии, и выбор делает автор. Сам подставить черновик нельзя (он
 * может быть вчерашним), сам выбросить — тем более.
 */
async function openFile(id, name) {
  if (!id || !name) return;
  try {
    const body = await api.file(id, name);
    state.file = name;
    state.revision = body.revision;
    const kept = draft.loadFile(localStorage, id, name);
    if (kept && kept.source !== body.text) {
      state.conflict = {
        kind: "draft",
        seen: kept.revision,
        actual: body.revision,
        text: body.text,
        draft: kept,
      };
      showConflict(
        t("account.draftDiffers", {
          saved: when(kept.savedAt),
          revision: body.revision,
        }),
      );
      host.open({ source: kept.source, scenario: kept.scenario ?? "" });
    } else {
      hideConflict();
      host.open({ source: body.text, scenario: state.project?.scenario ?? "" });
    }
    refresh();
  } catch (error) {
    host.say(text(error), "error");
  }
}

/** Сохраняет открытый файл на сервер. */
async function save() {
  if (!editing()) {
    host.say(t("account.nothingToSave"), "warning");
    return;
  }
  if (state.level !== "edit" && state.level !== "owner") {
    host.say(t("account.readOnly"), "warning");
    return;
  }
  try {
    const written = await api.write(
      state.project.id,
      state.file,
      host.source(),
      state.revision,
    );
    state.revision = written.revision;
    draft.clearFile(localStorage, state.project.id, state.file);
    hideConflict();
    host.say(t("account.saved", { revision: written.revision }), "ok");
    // Состав файлов мог измениться (первое сохранение заводит файл): список
    // перечитывается, иначе он остался бы вчерашним.
    await openProjectFiles(state.project.id);
    refresh();
  } catch (error) {
    if (error?.code === "revision_conflict" && typeof error.revision === "number") {
      // ⚠️ Числа взяты ПОЛЯМИ ответа, а не разобраны из текста: текст отказа
      // переводится и правится, а протокол — нет.
      state.conflict = { kind: "server", seen: state.revision, actual: error.revision };
      showConflict(
        t("account.conflict", { mine: state.revision, theirs: error.revision }),
      );
      return;
    }
    host.say(text(error), "error");
  }
}

/** Разрешает конфликт выбором автора. */
async function resolveConflict(choice) {
  if (!state.conflict || !editing()) return;
  try {
    if (choice === "reread") {
      const body = await api.file(state.project.id, state.file);
      state.revision = body.revision;
      draft.clearFile(localStorage, state.project.id, state.file);
      host.open({ source: body.text, scenario: "" });
      host.say(t("account.rereadDone", { revision: body.revision }), "ok");
    } else {
      // Перезаписать — это записать поверх ТОЙ ревизии, что у сервера сейчас:
      // автор увидел оба числа и решил.
      state.revision = state.conflict.actual;
      hideConflict();
      await save();
      return;
    }
    hideConflict();
    refresh();
  } catch (error) {
    host.say(text(error), "error");
  }
}

function showConflict(message) {
  dom.conflict.hidden = false;
  dom.conflicttext.textContent = message;
}

function hideConflict() {
  state.conflict = null;
  dom.conflict.hidden = true;
  dom.conflicttext.textContent = "";
}

/** Приводит панель и шапку в согласие с тем, что открыто. */
function refresh() {
  const me = api.who();
  dom.signedout.hidden = me !== null;
  dom.signedin.hidden = me === null;
  dom.whoami.textContent = me ? me.login : "";
  dom.account.textContent = me ? me.login : t("account.enter");
  const writable = editing() && (state.level === "edit" || state.level === "owner");
  dom.save.hidden = !writable;
  dom.openfile.hidden = !editing();
  if (editing()) {
    // ⚠️ У НОВОГО файла ревизии нет вовсе, и печатать «ревизия null» нельзя:
    // подпись читает человек, а `null` в ней — сообщение об ошибке, которой не
    // было (нашлось прогоном страницы).
    dom.openfile.textContent =
      state.revision === null
        ? t("account.openNew", { project: state.project.name, file: state.file })
        : t("account.openFile", {
            project: state.project.name,
            file: state.file,
            revision: state.revision,
          });
  }
}

/**
 * Имя уровня словом.
 *
 * ⚠️ Ключи перечислены **буквально**, а не собраны из строки уровня: ключ,
 * собранный на ходу, невидим сверке словаря — и пропавший перевод обнаружился
 * бы у читателя, а не у гейта.
 */
function levelName(level) {
  if (level === "owner") return t("account.level.owner");
  if (level === "edit") return t("account.level.edit");
  if (level === "fork") return t("account.level.fork");
  if (level === "view") return t("account.level.view");
  return t("account.level.none");
}

/** Человеческая дата черновика: без неё «разошлось» ничего не говорит. */
function when(savedAt) {
  if (!savedAt) return "—";
  return new Date(savedAt).toLocaleString();
}

/** Текст отказа: свой — из словаря, чужой — как прислал сервер. */
function text(error) {
  return t(error?.key ?? "api.failed", error?.params ?? { message: String(error) });
}
