// Черновик: работа автора переживает перезагрузку (фича 0531, R13).
//
// # Зачем
//
// В редакторе лежит НЕСОХРАНЁННЫЙ исходник. Его теряет всё: случайно закрытая
// вкладка, перезагрузка ради новой сборки, паника модуля (в WebAssembly она
// есть `abort`, и страница остаётся без модуля). Проработка задачи 0531-07
// назвала это первым требованием: сохранность идёт впереди уведомлений.
//
// ⚠️ Черновик живёт в браузере и только в нём: ни синхронизации между
// устройствами, ни отправки на сервер. Обещание A6 — не хранить чужой код без
// спроса — выполняется тем, что хранить нечего.

/** Ключ хранилища. Версия в имени: форма записи ещё может измениться. */
const KEY = "takt.draft.v1";

/**
 * Предел черновика — тот же, что у публикации (64 КиБ, решение A6).
 *
 * ⚠️ Превышение сообщается, а не режется молча: усечённый исходник выглядит
 * целым и перестаёт компилироваться в месте, которого автор не писал.
 */
export const LIMIT_BYTES = 64 * 1024;

/**
 * Сохраняет черновик; возвращает `null` либо причину отказа.
 *
 * ⚠️ Причина возвращается **ключом словаря и подстановками**, а не строкой:
 * текст оболочки строит одна точка — главный поток страницы (задача 0531-10a).
 * Здесь строкой была бы вторая копия словаря.
 */
export function save(storage, draft) {
  const text = JSON.stringify(draft);
  const size = new TextEncoder().encode(text).length;
  if (size > LIMIT_BYTES) {
    return { key: "draft.tooBig", params: { limit: Math.floor(LIMIT_BYTES / 1024), size } };
  }
  try {
    storage.setItem(KEY, text);
    return null;
  } catch (error) {
    // Приватный режим, переполненное хранилище, запрет сайту — записи нет, и
    // автор обязан об этом узнать: он рассчитывает, что текст переживёт
    // перезагрузку.
    return { key: "draft.notSaved", params: { error: error?.message ?? error } };
  }
}

/** Читает черновик; `null` — его нет либо он испорчен. */
export function load(storage) {
  let text;
  try {
    text = storage.getItem(KEY);
  } catch {
    return null;
  }
  if (!text) return null;
  try {
    const parsed = JSON.parse(text);
    return {
      source: parsed.source ?? "",
      scenario: parsed.scenario ?? "",
      target: parsed.target ?? "",
      args: parsed.args ?? "",
    };
  } catch {
    // Испорченная запись — не повод падать: страница открывается с
    // умолчаниями, как при первом заходе.
    return null;
  }
}

/** Забывает черновик (после публикации либо по просьбе автора). */
export function clear(storage) {
  try {
    storage.removeItem(KEY);
  } catch {
    // Нечего забывать — не ошибка.
  }
}

/**
 * Откладывает вызов: черновик пишется не на каждую букву.
 *
 * ⚠️ Задержка — не оптимизация ради красоты: `localStorage` синхронен, и
 * запись на каждое нажатие подтормаживала бы набор на длинной модели.
 */
export function debounce(fn, delayMs) {
  let timer = null;
  let pending = null;
  const wrapped = (...args) => {
    pending = args;
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      fn(...pending);
    }, delayMs);
  };
  /**
   * Записывает немедленно, если запись отложена.
   *
   * ⚠️ Нужно там, где страницу вот-вот перезагрузят (выход на новую сборку):
   * отложенная на 400 мс запись до перезагрузки не доживёт, и автор потеряет
   * последние набранные строки.
   */
  wrapped.now = () => {
    if (timer === null) return;
    clearTimeout(timer);
    timer = null;
    fn(...pending);
  };
  return wrapped;
}
