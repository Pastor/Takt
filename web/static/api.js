// Разговор страницы с сервером проектов (фича 0531, задача 09e).
//
// # Что здесь есть и чего нет
//
// Только транспорт: пара токенов, их обновление и вызовы ручек. Правил доступа
// здесь нет ни строки — кому что можно, решает сервер, и ответ он называет
// словом (`level`). Заведи проверку здесь — правило стало бы жить в двух
// местах и разошлось бы при первой правке (класс 0084).
//
// # Токены
//
// Access живёт час, refresh — непрозрачная строка, одноразовая. Оба лежат в
// `localStorage`: страница переживает перезагрузку, и заново входить после
// каждого обновления сборки автор не должен.
//
// ⚠️ Обновление пары идёт **один раз на несколько запросов сразу**: refresh
// одноразовый, и две параллельные попытки погасили бы семейство — то есть
// выкинули бы автора из сессии ровно в тот момент, когда он сохраняет работу.

/** Ключ хранилища пары. Версия в имени: форма записи ещё может измениться. */
const KEY = "takt.session.v1";

/** Кто вошёл и с какой парой. */
let session = null;

/** Обновление пары, если оно уже идёт: второго не заводим. */
let refreshing = null;

/** Корень API. Ставится страницей: за прокси он с префиксом. */
let root = "/";

/** Способ сходить на сервер. Подменяется проверками. */
let get = (...args) => fetch(...args);

/** Хранилище пары. Подменяется проверками. */
let store = null;

/**
 * Настраивает клиента.
 *
 * @param {{root?: string, fetch?: typeof fetch, storage?: Storage}} options
 */
export function configure(options = {}) {
  if (options.root) root = options.root;
  if (options.fetch) get = options.fetch;
  store = options.storage ?? null;
  session = read();
  refreshing = null;
}

/** Кто вошёл; `null` — никто. */
export function who() {
  return session ? { login: session.login, role: session.role } : null;
}

/** Вошли ли мы. */
export function signed() {
  return session !== null;
}

/**
 * Заводит учётную запись и входит ею же.
 *
 * @returns {Promise<{login: string, role: string}>}
 */
export async function register(login, password) {
  return await enter("register", { login, password });
}

/** Входит существующей записью. */
export async function signIn(login, password) {
  return await enter("token", { grant_type: "password", login, password });
}

/**
 * Выходит: гасит refresh на сервере и забывает пару здесь.
 *
 * ⚠️ Пара забывается **в любом случае**, даже если сервер не ответил: иначе
 * «выйти» на неработающей сети оставляло бы автора внутри.
 */
export async function signOut() {
  const token = session?.refresh;
  session = null;
  remember(null);
  if (!token) return;
  try {
    await call("revoke", { method: "POST", body: { refresh_token: token } }, false);
  } catch {
    // Сервер недоступен — пара всё равно забыта здесь.
  }
}

/** Мои проекты и выданные мне, каждый со своим уровнем. */
export async function projects() {
  return await call("projects");
}

/** Заводит проект. */
export async function create(name, description = "") {
  return await call("projects", { method: "POST", body: { name, description } });
}

/** Читает проект: метаданные, состав файлов, мой уровень. */
export async function project(id) {
  return await call(`projects/${encodeURIComponent(id)}`);
}

/** Правит метаданные проекта. */
export async function patch(id, fields) {
  return await call(`projects/${encodeURIComponent(id)}`, { method: "PATCH", body: fields });
}

/** Читает файл: текст и ревизию ПРОЕКТА на момент чтения. */
export async function file(id, name) {
  return await call(`projects/${encodeURIComponent(id)}/files/${encodeURIComponent(name)}`);
}

/**
 * Пишет файл.
 *
 * @param {number|null} revision ревизия, которую видел автор; `null` — файл новый
 */
export async function write(id, name, text, revision) {
  return await call(`projects/${encodeURIComponent(id)}/files/${encodeURIComponent(name)}`, {
    method: "PUT",
    body: revision === null || revision === undefined ? { text } : { text, revision },
  });
}

/** Вход: обе ручки отвечают парой. */
async function enter(path, body) {
  const pair = await call(path, { method: "POST", body }, false);
  session = {
    access: pair.access_token,
    refresh: pair.refresh_token,
    login: "",
    role: "user",
  };
  const me = await call("me");
  session.login = me.login;
  session.role = me.role;
  remember(session);
  return { login: session.login, role: session.role };
}

/**
 * Зовёт ручку API.
 *
 * ⚠️ Один просроченный access-токен обновляет пару и **повторяет запрос** — но
 * ровно один раз: второй отказ означает, что вошли заново, и молчаливый цикл
 * повторов скрыл бы это от автора.
 */
async function call(path, options = {}, retry = true) {
  const headers = { accept: "application/json" };
  if (options.body !== undefined) headers["content-type"] = "application/json";
  if (session?.access) headers.authorization = `Bearer ${session.access}`;
  let response;
  try {
    response = await get(`${root}api/${path}`, {
      method: options.method ?? "GET",
      headers,
      body: options.body === undefined ? undefined : JSON.stringify(options.body),
    });
  } catch (error) {
    throw named({ key: "api.offline", params: { error: error?.message ?? String(error) } });
  }
  if (response.status === 401 && retry && session?.refresh) {
    if (await renew()) return await call(path, options, false);
  }
  if (response.status === 204) return null;
  let body = null;
  try {
    body = await response.json();
  } catch {
    body = null;
  }
  if (!response.ok) throw failure(response.status, body);
  return body;
}

/**
 * Обновляет пару. `false` — не вышло, и войти придётся заново.
 *
 * ⚠️ Обновление одно на все запросы сразу: refresh одноразовый, и две
 * параллельные попытки гасят семейство — автора выкинуло бы из сессии ровно
 * тогда, когда он сохраняет работу.
 */
async function renew() {
  if (!refreshing) {
    refreshing = (async () => {
      try {
        const pair = await call(
          "token",
          {
            method: "POST",
            body: { grant_type: "refresh_token", refresh_token: session.refresh },
          },
          false,
        );
        session = { ...session, access: pair.access_token, refresh: pair.refresh_token };
        remember(session);
        return true;
      } catch {
        session = null;
        remember(null);
        return false;
      } finally {
        refreshing = null;
      }
    })();
  }
  return await refreshing;
}

/**
 * Превращает отказ сервера в отказ страницы.
 *
 * ⚠️ Машинный код (`error`) — для страницы, текст — для человека, а числа
 * ревизии едут **полями**: разбирать их из сообщения значило бы сделать текст
 * отказа частью протокола.
 */
function failure(status, body) {
  // ⚠️ Ключ ОДИН, а не `api.<код>`: текст отказа сервер пишет сам, и он, как
  // диагностики инструментов, пока только по-русски (граница названа задачей
  // 10a, снимает её фича 0532). Заведи здесь ключ на каждый код — половина
  // словаря повторяла бы сообщения сервера и расходилась бы с ними молча.
  const error = named({
    key: "api.failed",
    params: { message: body?.message ?? String(status) },
  });
  error.status = status;
  error.code = body?.error ?? "failed";
  error.message_text = body?.message ?? "";
  if (typeof body?.revision === "number") error.revision = body.revision;
  if (typeof body?.seen === "number") error.seen = body.seen;
  return error;
}

/** Отказ с ключом словаря: текст строит страница. */
function named(problem) {
  const error = new Error(problem.key);
  error.key = problem.key;
  error.params = problem.params;
  return error;
}

function read() {
  try {
    const text = store?.getItem(KEY);
    if (!text) return null;
    const parsed = JSON.parse(text);
    return parsed?.access && parsed?.refresh ? parsed : null;
  } catch {
    return null;
  }
}

function remember(value) {
  try {
    if (value) store?.setItem(KEY, JSON.stringify(value));
    else store?.removeItem(KEY);
  } catch {
    // Приватный режим либо переполненное хранилище: сессия живёт до
    // перезагрузки, и это лучше, чем отказ входа.
  }
}
