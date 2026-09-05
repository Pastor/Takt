// Выпадающий список в стиле страницы (фича 0531, задача 07b).
//
// # Зачем свой, если есть системный
//
// Решение заказчика 2026-09-04 (приём референса): системный `<select>` рисуется
// операционной системой и выпадает из оформления — чужие шрифт, высота,
// скругление и цвета посреди чертёжной страницы.
//
// # Чем платим и как платим
//
// Доступность. У системного списка она даром: клавиатура, экранный диктор,
// список на телефоне. Здесь всё это делается заново, и потому:
//
//   - **источник истины остаётся `<select>`** — значение, порядок и подписи
//     живут в нём, а не в разметке меню. Страница по-прежнему слушает его
//     `change` и ничего не знает про эту надстройку;
//   - кнопка объявлена `combobox`, меню — `listbox`, вариант — `option` с
//     `aria-selected`; сам `<select>` убран из дерева доступности
//     (`aria-hidden`, `tabindex="-1"`), иначе диктор нашёл бы ДВА списка;
//   - клавиатура: Enter, Пробел и стрелки открывают; стрелки, Home и End
//     двигают; Enter и Пробел выбирают; Escape закрывает и возвращает фокус на
//     кнопку; Tab закрывает; набор буквы прыгает к варианту.
//
// ⚠️ Указатель НАРИСОВАН рамкой в CSS, а не набран символом: треугольника
// (`▾`, `▼`) Fira Code не знает, и браузер подставил бы
// системный глиф на чужой базовой линии.

/**
 * Надстраивает список над `<select>`.
 *
 * @param {HTMLSelectElement} select источник значений и подписей
 * @returns {{refresh: () => void}} обновление меню после смены состава
 */
export function enhance(select) {
  const document_ = select.ownerDocument;
  const box = document_.createElement("div");
  box.className = "pick" + (select.classList.contains("lang") ? " lang-pick" : "");
  const button = document_.createElement("button");
  button.type = "button";
  button.className = "pick-btn";
  button.setAttribute("role", "combobox");
  button.setAttribute("aria-haspopup", "listbox");
  button.setAttribute("aria-expanded", "false");
  const label = document_.createElement("span");
  button.appendChild(label);
  const menu = document_.createElement("div");
  menu.className = "pick-menu";
  menu.setAttribute("role", "listbox");
  menu.hidden = true;

  select.parentNode.insertBefore(box, select);
  box.append(select, button, menu);
  // Список остаётся носителем значения, но с экрана и из дерева доступности
  // уходит: два списка на одно значение — это два разных ответа диктору.
  select.setAttribute("aria-hidden", "true");
  select.tabIndex = -1;
  if (select.id) {
    button.setAttribute("aria-label", labelOf(select) || select.id);
  }

  function labelOf(node) {
    const tag = node.ownerDocument.querySelector(`label[for="${node.id}"]`);
    return tag ? tag.textContent.trim() : "";
  }

  function refresh() {
    // ⚠️ Кнопка показывает КОРОТКУЮ метку, если вариант её несёт
    // (`data-short`): переключатель языка стоит рядом со значками шапки и
    // обязан быть им под стать. В списке при этом остаётся полное название.
    const picked = select.selectedOptions[0];
    label.textContent = picked?.dataset.short ?? picked?.textContent ?? "";
    menu.replaceChildren();
    for (const option of select.options) {
      const item = document_.createElement("button");
      item.type = "button";
      item.className = "pick-opt";
      item.setAttribute("role", "option");
      item.setAttribute("aria-selected", String(option.selected));
      item.textContent = option.textContent;
      item.dataset.value = option.value;
      item.addEventListener("click", () => choose(option.value));
      menu.appendChild(item);
    }
  }

  function open() {
    if (!menu.hidden) return;
    menu.hidden = false;
    button.setAttribute("aria-expanded", "true");
    const current = menu.querySelector('[aria-selected="true"]') ?? menu.firstElementChild;
    current?.focus();
  }

  function close(returnFocus) {
    if (menu.hidden) return;
    menu.hidden = true;
    button.setAttribute("aria-expanded", "false");
    if (returnFocus) button.focus();
  }

  function choose(value) {
    select.value = value;
    // Событие рождается на `<select>`: страница слушает ЕГО, и надстройка
    // остаётся невидимой для всего остального кода.
    select.dispatchEvent(new Event("change", { bubbles: true }));
    refresh();
    close(true);
  }

  function move(from, step) {
    const items = [...menu.children];
    if (items.length === 0) return;
    const at = items.indexOf(from);
    const next = step === "home" ? 0
      : step === "end" ? items.length - 1
      : (at + step + items.length) % items.length;
    items[next].focus();
  }

  button.addEventListener("click", () => (menu.hidden ? open() : close(true)));
  button.addEventListener("keydown", (event) => {
    if (["ArrowDown", "ArrowUp", "Enter", " "].includes(event.key)) {
      event.preventDefault();
      open();
    }
  });

  menu.addEventListener("keydown", (event) => {
    const item = event.target.closest(".pick-opt");
    switch (event.key) {
      case "ArrowDown": event.preventDefault(); move(item, 1); break;
      case "ArrowUp": event.preventDefault(); move(item, -1); break;
      case "Home": event.preventDefault(); move(item, "home"); break;
      case "End": event.preventDefault(); move(item, "end"); break;
      case "Escape": event.preventDefault(); close(true); break;
      case "Tab": close(false); break;
      case "Enter":
      case " ":
        event.preventDefault();
        if (item) choose(item.dataset.value);
        break;
      default:
        // Набор буквы прыгает к варианту: у системного списка так, и без
        // этого длинный список целей перебирают стрелками по одному.
        if (event.key.length === 1) jump(event.key);
    }
  });

  function jump(letter) {
    const lower = letter.toLowerCase();
    const item = [...menu.children].find((node) =>
      node.textContent.trim().toLowerCase().startsWith(lower)
    );
    item?.focus();
  }

  // Клик мимо закрывает: меню поверх страницы, и уйти от него надо уметь без
  // клавиатуры.
  document_.addEventListener("pointerdown", (event) => {
    if (!box.contains(event.target)) close(false);
  });
  // Значение могли поменять извне (ссылка, черновик, открытый проект) —
  // надстройка обязана догнать: иначе кнопка показывает одно, а собирается
  // другое. ⚠️ Догоняет ЦЕЛИКОМ (`refresh`), а не одной подписью: прежде
  // обновлялась только она, и в меню оставался прежний `aria-selected` —
  // диктору список отвечал вчерашним выбором.
  select.addEventListener("change", refresh);
  refresh();
  return { refresh };
}
