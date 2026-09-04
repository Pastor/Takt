// Ширина оболочки: читатель задаёт её сам (фича 0531, задача 07b).
//
// # Зачем
//
// У референса содержимое стоит колонкой ограниченной ширины по центру
// (`main { max-width: 1100px; margin: 0 auto }`): на широком мониторе строка во
// весь экран читается плохо. У редактора две колонки кода, и «сколько нужно»
// зависит от монитора и от модели — поэтому ширину задаёт **читатель**, а не
// число в стилях (решение заказчика 2026-09-04).
//
// Шапка, рабочая область и нижняя полка делят ОДНУ ширину: разъедься они —
// вкладки перестали бы стоять над своей областью.
//
// # Границы
//
//   - **наибольшая — размер окна**: шире монитора оболочки не бывает;
//   - наименьшая — `MIN_WIDTH`: уже двух колонок кода раскладка всё равно
//     схлопывается в одну (порог 900 px), и сужать дальше нечего;
//   - на узком экране ручки нет вовсе: там область одна на экран.
//
// Значение помнится в `localStorage`: это настройка удобства читателя, и
// спрашивать её заново на каждом заходе незачем. В ссылку-снимок она не
// входит — ширина монитора у получателя своя.

/** Ключ хранилища. */
export const KEY = "takt.shell";

/** Наименьшая ширина оболочки: уже неё две колонки кода не имеют смысла. */
export const MIN_WIDTH = 640;

/**
 * Приводит запрошенную ширину к допустимой.
 *
 * ⚠️ Отдельной функцией, потому что проверяется в `node`: DOM там нет, а
 * правило границ есть, и ошибка в нём делает оболочку либо неуправляемо узкой,
 * либо шире окна.
 */
export function clamp(width, windowWidth) {
  const most = Math.max(MIN_WIDTH, windowWidth);
  return Math.round(Math.min(most, Math.max(MIN_WIDTH, width)));
}

/** Читает запомненную ширину; `null` — её нет либо запись испорчена. */
export function stored(storage) {
  try {
    const value = Number(storage.getItem(KEY));
    return Number.isFinite(value) && value > 0 ? value : null;
  } catch {
    return null;
  }
}

/**
 * Заводит ручку ширины.
 *
 * @param {HTMLElement} grip элемент-разделитель
 * @param {Storage} storage хранилище настройки
 */
export function attach(grip, storage) {
  const root = grip.ownerDocument.documentElement;
  let width = stored(storage) ?? window.innerWidth;

  const apply = (next) => {
    width = clamp(next, window.innerWidth);
    root.style.setProperty("--shell-w", `${width}px`);
    grip.setAttribute("aria-valuenow", String(width));
    grip.setAttribute("aria-valuemin", String(MIN_WIDTH));
    grip.setAttribute("aria-valuemax", String(Math.max(MIN_WIDTH, window.innerWidth)));
  };

  const remember = () => {
    try {
      storage.setItem(KEY, String(width));
    } catch {
      // Приватный режим либо запрет сайту: ширина действует до перезагрузки.
    }
  };

  // Тяга мышью и пальцем — одним обработчиком: указатель у браузера один.
  // ⚠️ Оболочка стоит ПО ЦЕНТРУ, поэтому сдвиг края меняет ширину ВДВОЕ:
  // считать её от одной стороны — значит уводить содержимое вбок.
  grip.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    grip.setPointerCapture(event.pointerId);
    const startX = event.clientX;
    const startWidth = width;
    const move = (moved) => apply(startWidth + (moved.clientX - startX) * 2);
    const stop = () => {
      grip.removeEventListener("pointermove", move);
      grip.removeEventListener("pointerup", stop);
      grip.removeEventListener("pointercancel", stop);
      remember();
    };
    grip.addEventListener("pointermove", move);
    grip.addEventListener("pointerup", stop);
    grip.addEventListener("pointercancel", stop);
  });

  // Клавиатура: разделитель без неё недоступен вовсе.
  grip.addEventListener("keydown", (event) => {
    const step = event.shiftKey ? 160 : 32;
    switch (event.key) {
      case "ArrowLeft": apply(width - step); remember(); break;
      case "ArrowRight": apply(width + step); remember(); break;
      case "Home": apply(MIN_WIDTH); remember(); break;
      case "End": apply(window.innerWidth); remember(); break;
      default: return;
    }
    event.preventDefault();
  });

  // Двойной щелчок возвращает во всю ширину: сузив оболочку случайно, вернуть
  // её надо одним движением.
  grip.addEventListener("dblclick", () => {
    apply(window.innerWidth);
    remember();
  });

  // Окно уменьшили — оболочка обязана поместиться; запомненное при этом не
  // портится: вернут окно, вернётся и ширина.
  window.addEventListener("resize", () => apply(width));

  apply(width);
}
