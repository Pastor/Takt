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
import { feed } from "./showcase.js";
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

/**
 * Лента витрины: чем спросить следующую страницу, знает она.
 *
 * ⚠️ Курсор здесь не считается и не хранится россыпью по обработчикам: правило
 * «спросить тем, что дал сервер, и остановиться, когда он молчит» живёт одним
 * носителем и проверяется без браузера.
 */
const showcase = feed((query, cursor) => api.showcase(query, cursor));

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
  // ⚠️ Кнопка ОДНА на вход и выход: пока не вошли — открывает панель со
  // формой, после входа — выходит. Двух кнопок, из которых всегда видна одна,
  // читателю не нужно (раскладка заказчика 2026-09-05).
  dom.session.addEventListener("click", () => (api.who() ? leave() : openSignin()));
  dom["signin-cancel"].addEventListener("click", closeSignin);
  // Окно закрывается щелчком по затемнению и клавишей Escape — как всякое
  // модальное: из разговора обязан быть выход, не требующий попадания в кнопку.
  dom["signin-modal"].addEventListener("click", (event) => {
    if (event.target === dom["signin-modal"]) closeSignin();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !dom["signin-modal"].hidden) closeSignin();
  });
  dom.signin.addEventListener("click", () => enter(api.signIn));
  dom.signup.addEventListener("click", () => enter(api.register));
  dom.signout.addEventListener("click", () => leave());
  dom.newproject.addEventListener("click", () => make());
  dom.save.addEventListener("click", () => save());
  dom.reread.addEventListener("click", () => resolveConflict("reread"));
  dom.overwrite.addEventListener("click", () => resolveConflict("overwrite"));
  dom.download.addEventListener("click", () => download());
  dom.upload.addEventListener("change", (event) => upload(event.target.files?.[0]));
  dom.setpass.addEventListener("click", () => setPassword());
  dom.showcase.addEventListener("click", () => toggleShowcase());
  dom.findbtn.addEventListener("click", () => search());
  dom.more.addEventListener("click", () => showMore());
  dom.query.addEventListener("keydown", (event) => {
    if (event.key === "Enter") search();
  });
  dom.found.addEventListener("click", (event) => {
    const row = event.target.closest("[data-open]");
    // ⚠️ Обычный переход, а не открытие «внутри»: у живой страницы проекта
    // свой адрес, и он обязан оказаться в адресной строке — иначе им нельзя
    // поделиться, а ради этого витрина и существует.
    if (row) location.assign(`${api.apiRoot()}p/${row.dataset.open}`);
  });
  dom.links.addEventListener("click", (event) => {
    const row = event.target.closest("[data-unlink]");
    if (row) unlinkProvider(row.dataset.unlink);
  });
  dom.pickok.addEventListener("click", () => {
    const login = dom.picklogin.value.trim();
    if (!login) {
      host.say(t("oauth.pickLogin"), "warning");
      return;
    }
    exchange(state.ticket, login);
  });
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
  fillProviders();
}

/**
 * Принимает проект, открытый по адресу `/p/<id>` (задача 09c).
 *
 * ⚠️ Нужно ради архива: читатель видит чужой открытый проект и вправе его
 * скачать — текст всё равно уже у него в браузере. Уровень при этом остаётся
 * читательским: сохранять такой проект нельзя, и кнопки сохранения не будет.
 */
export function adopt(project) {
  state.project = { id: project.id, name: project.name };
  state.file = null;
  state.level = "view";
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

/**
 * Строит кнопки площадок по ответу сервера.
 *
 * ⚠️ Имён площадок в коде страницы нет: идентификатор и **ключ подписи**
 * приходят от сервера. Заведи здесь свой список — он разошёлся бы с настройкой
 * стенда молча, и кнопка вела бы в никуда.
 */
async function fillProviders() {
  let list = [];
  try {
    list = await api.oauthProviders();
  } catch {
    // Площадки не настроены либо сервера нет: вход паролем работает, и
    // сообщать тут не о чем.
    list = [];
  }
  dom.oauth.replaceChildren();
  dom.oauth.hidden = list.length === 0;
  for (const item of list) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "brand-btn";
    // ⚠️ Значок приходит ИМЕНЕМ ФАЙЛА от сервера, путь строит страница: имён
    // площадок в её коде нет — тем же приёмом, что и подписи (задача 09f-3).
    if (item.icon) {
      const mark = document.createElement("img");
      mark.className = "brand-mark";
      // ⚠️ Адрес считается от МОДУЛЯ (`import.meta.url`), а не от документа:
      // страница живёт и по адресу `/p/<id>`, и относительный путь увёл бы в
      // корень — файл приехал бы 404 при пустой на вид кнопке (класс 07b).
      mark.src = new URL(`brand/${encodeURIComponent(item.icon)}`, import.meta.url).href;
      mark.alt = "";
      button.appendChild(mark);
    }
    button.appendChild(document.createTextNode(t(item.label)));
    button.addEventListener("click", () => leaveTo(item));
    dom.oauth.appendChild(button);
  }
}

/**
 * Уходит на площадку.
 *
 * ⚠️ Черновик пишется НЕМЕДЛЕННО, до перехода: отложенная на 400 мс запись до
 * ухода со страницы не доживёт, а `beforeunload` встретил бы вопросом «уйти?»
 * каждого, кто нажал «Войти».
 */
function leaveTo(item) {
  host.keep();
  const path = `${api.apiRoot()}api/oauth/${encodeURIComponent(item.id)}/start`;
  const query = new URLSearchParams({ return_to: location.pathname });
  if (item.via) query.set("via", item.via);
  // Обычная навигация, а не `fetch`: cookie потока и перенаправление на
  // площадку бывают только у неё.
  location.assign(`${path}?${query}`);
}

/**
 * Разбирает возврат с площадки: `#login=…`, `#login_error=…`, `#linked=1`.
 *
 * ⚠️ Фрагмент стирается СРАЗУ и до первого запроса: ticket — секрет этого шага,
 * и оставлять его в адресной строке и в истории браузера незачем.
 */
export async function handleReturn() {
  const raw = location.hash.replace(/^#/, "");
  if (!raw) return false;
  const params = new URLSearchParams(raw);
  const ticket = params.get("login");
  const failure = params.get("login_error");
  const linked = params.get("linked");
  if (!ticket && !failure && !linked) return false;
  history.replaceState(null, "", location.pathname + location.search);

  if (failure) {
    // Причина приходит КЛЮЧОМ, а не текстом площадки: чужой текст не
    // переводится и не всегда предназначен читателю.
    host.say(oauthError(failure), "error");
    return true;
  }
  if (linked) {
    host.say(t("oauth.linked"), "ok");
    refresh();
    return true;
  }
  await exchange(ticket, null);
  return true;
}

/** Меняет ticket на пару; при нужде спрашивает логин. */
async function exchange(ticket, login) {
  try {
    const pair = await api.oauthComplete(ticket, login);
    hideLoginPrompt();
    host.say(t("account.hello", { login: pair.login }), "ok");
    refresh();
    await list();
  } catch (error) {
    if (error?.message_text === "login_required") {
      // Первый вход: логин выбирает человек — имени площадки мы не читаем.
      showLoginPrompt(ticket);
      return;
    }
    fail(error);
  }
}

/** Показывает окно «Выберите логин». */
function showLoginPrompt(ticket) {
  state.ticket = ticket;
  dom.pick.hidden = false;
  dom.picklogin.value = "";
  dom.picklogin.focus();
  host.say(t("oauth.pickLogin"), "warning");
}

function hideLoginPrompt() {
  state.ticket = null;
  dom.pick.hidden = true;
}

/** Текст отказа входа через площадку — по конечному словарю ключей. */
function oauthError(key) {
  if (key === "denied") return t("oauth.error.denied");
  if (key === "expired") return t("oauth.error.expired");
  if (key === "csrf") return t("oauth.error.csrf");
  if (key === "unavailable") return t("oauth.error.unavailable");
  if (key === "identity_taken") return t("oauth.error.identityTaken");
  return t("oauth.error.failed");
}

/** Показывает или прячет витрину. */
async function toggleShowcase() {
  const show = dom.finder.hidden;
  dom.finder.hidden = !show;
  if (show) await search();
}

/** Ищет по витрине открытых проектов — с первой страницы. */
async function search() {
  try {
    const items = await showcase.first(dom.query.value.trim());
    dom.found.replaceChildren();
    showRows(items);
    if (items.length === 0) {
      const empty = document.createElement("div");
      empty.className = "row row-ok";
      empty.textContent = t("showcase.nothing");
      dom.found.appendChild(empty);
    }
  } catch (error) {
    fail(error);
  }
}

/**
 * Досыпает следующую страницу витрины.
 *
 * ⚠️ Записи ДОБАВЛЯЮТСЯ, а не заменяют показанные: «ещё» — это продолжение
 * списка, и подмена содержимого выглядела бы потерей найденного.
 */
async function showMore() {
  try {
    showRows(await showcase.next());
  } catch (error) {
    fail(error);
  }
}

/**
 * Дописывает записи витрины и показывает «ещё» ровно тогда, когда сервер
 * сказал, что продолжение есть.
 */
function showRows(items) {
  for (const item of items) {
    const node = document.createElement("div");
    node.className = "row";
    node.dataset.open = item.id;
    node.textContent = t("showcase.row", { name: item.name, owner: item.owner });
    dom.found.appendChild(node);
  }
  dom.more.hidden = !showcase.hasMore();
}

/**
 * Скачивает открытый проект архивом.
 *
 * ⚠️ Байты приходят запросом, а файл отдаётся временной ссылкой: у закрытого
 * проекта архив требует токена, а обычная ссылка заголовков не несёт.
 */
async function download() {
  if (!state.project) {
    host.say(t("account.nothingToSave"), "warning");
    return;
  }
  try {
    const bytes = await api.archive(state.project.id, host.target());
    const url = URL.createObjectURL(new Blob([bytes], { type: "application/zip" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = `${state.project.name || "takt-project"}.zip`;
    link.click();
    // Ссылка живёт до конца загрузки: снятая сразу, она отменила бы её.
    setTimeout(() => URL.revokeObjectURL(url), 10_000);
    host.say(t("archive.downloaded"), "ok");
  } catch (error) {
    fail(error);
  }
}

/** Загружает проект из архива. */
async function upload(file) {
  if (!file) return;
  try {
    const bytes = await file.arrayBuffer();
    const created = await api.importArchive(bytes);
    host.say(t("archive.imported", { name: created.name }), "ok");
    await list();
    await openProject(created.id);
  } catch (error) {
    fail(error);
  } finally {
    // Тот же файл выбирают дважды: без сброса второе «выбрать» молчит.
    dom.upload.value = "";
  }
}

/** Задаёт пароль записи, у которой его не было. */
async function setPassword() {
  const password = dom.newpass.value;
  if (!password) {
    host.say(t("account.needBoth"), "warning");
    return;
  }
  try {
    await api.setPassword(password);
    dom.newpass.value = "";
    // ⚠️ Пароль гасит живые сеансы: вход придётся повторить, и сказать об
    // этом надо здесь, а не оставить человека гадать, почему всё отказывает.
    host.say(t("profile.passwordSet"), "ok");
    await api.signOut();
    refresh();
    fillProviders();
  } catch (error) {
    fail(error);
  }
}

/** Отвязывает площадку. */
async function unlinkProvider(provider) {
  try {
    await api.oauthUnlink(provider);
    host.say(t("profile.unlinked"), "ok");
    await fillProfile();
  } catch (error) {
    fail(error);
  }
}

/** Наполняет раздел профиля: связанные площадки и «задать пароль». */
async function fillProfile() {
  if (!api.signed()) {
    dom.profile.hidden = true;
    return;
  }
  dom.profile.hidden = false;
  try {
    // ⚠️ «Задать пароль» предлагается только тому, у кого пароля НЕТ: иначе
    // человек узнавал бы об отказе нажатием, а не видом страницы.
    const me = await api.refreshMe();
    const needs = me !== null && me.has_password === false;
    dom.newpass.hidden = !needs;
    dom.setpass.hidden = !needs;
    const list = await api.oauthIdentities();
    dom.links.replaceChildren();
    for (const item of list) {
      const node = document.createElement("div");
      node.className = "row";
      node.dataset.unlink = item.provider;
      node.textContent = t("profile.linkedRow", { provider: item.provider });
      dom.links.appendChild(node);
    }
    if (list.length === 0) {
      const empty = document.createElement("div");
      empty.className = "row row-ok";
      empty.textContent = t("profile.noLinks");
      dom.links.appendChild(empty);
    }
  } catch (error) {
    fail(error);
  }
}

/** Показывает или прячет панель. */
/**
 * Открывает окно входа.
 *
 * ⚠️ Фокус уходит в поле логина: модальное окно, забирающее внимание, но не
 * фокус, для клавиатуры не открылось вовсе.
 */
function openSignin() {
  dom["signin-modal"].hidden = false;
  dom.login.focus();
}

/** Закрывает окно входа, не трогая набранное: вернуться в него — обычное дело. */
function closeSignin() {
  dom["signin-modal"].hidden = true;
  dom.session.focus();
}

function toggle(force) {
  const show = force ?? dom.panel.hidden;
  dom.panel.hidden = !show;
  if (show && api.signed()) {
    list();
    fillProfile();
  }
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
    // Разговор окончен ответом «вошёл» — окно закрывается само.
    closeSignin();
    host.say(t("account.hello", { login: me.login }), "ok");
    refresh();
    await list();
    await fillProfile();
  } catch (error) {
    fail(error);
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
    fail(error);
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
    fail(error);
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
    fail(error);
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
    fail(error);
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
    fail(error);
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
    fail(error);
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
  // ⚠️ Кнопки шапки — ЗНАЧКИ, и текста не несут: логин показывается подписью
  // рядом, а в саму кнопку уходит подпись для диктора и всплывающая.
  const label = me ? t("account.signOut") : t("account.enter");
  dom.session.dataset.tip = label;
  dom.session.setAttribute("aria-label", label);
  // Значок следует за действием: не вошёл — стрелка внутрь, вошёл — наружу.
  dom["icon-enter"].hidden = me !== null;
  dom["icon-leave"].hidden = me === null;
  dom["whoami-bar"].textContent = me ? me.login : "";
  dom["whoami-bar"].hidden = me === null;
  dom.download.hidden = state.project === null;
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

/**
 * Показывает отказ и приводит панель в согласие с сессией.
 *
 * ⚠️ Отказ `401` означает, что сохранённая пара больше не годится (сервер
 * перезапущен с чистой базой, токен отозван, семейство погашено). Без этого шага
 * панель продолжала бы показывать «вошли как …» и кнопку «Выйти» у сессии,
 * которой нет — нашлось прогоном страницы (задача 09f-3).
 */
function fail(error) {
  host.say(text(error), "error");
  if (error?.status === 401) {
    state.project = null;
    state.file = null;
    state.level = "none";
    refresh();
    fillProviders();
  }
}

/** Текст отказа: свой — из словаря, чужой — как прислал сервер. */
function text(error) {
  return t(error?.key ?? "api.failed", error?.params ?? { message: String(error) });
}
