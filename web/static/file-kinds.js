// Роды файлов проекта на странице: расширение, подпись и правило имени.
//
// # Один список на страницу
//
// Род файла выводится из расширения, и правило живёт в крейте проекта
// (`takt-project/src/kind.rs`): его же применяют сервер и командная строка. У
// страницы свой список неизбежен - подпись рода нужна до рейса, - но он один:
// окно "Новый файл", загрузка с диска и фильтр поля выбора файла берут его
// отсюда. Совпадение со списком крейта сверяет проверка страницы.
//
// # Отказ до рейса
//
// Сервер судит имя и размер сам, и его отказ остаётся последним рубежом. Страница
// ловит то, что может назвать заранее: чужое расширение, имя вне алфавита, файл
// больше предела. Отказ, который известен до отправки, не стоит рейса и не должен
// приходить от сервера текстом, не переведённым на язык страницы.

import { EXTENSION as LAYOUT_EXTENSION } from "./layout.js";
import { LIMIT_BYTES } from "./draft.js";

/** Роды файлов, которые автор вправе завести, и расширение каждого. */
export const FILE_KINDS = [
  { kind: "takt", label: "file.kind.takt", extension: ".takt" },
  { kind: "layout", label: "file.kind.layout", extension: LAYOUT_EXTENSION },
  { kind: "scenario", label: "file.kind.scenario", extension: ".json" },
  { kind: "markdown", label: "file.kind.markdown", extension: ".md" },
  { kind: "address_map", label: "file.kind.addressMap", extension: ".takt-map" },
];

/** Наибольшая длина имени файла, символов - та же, что у сервера. */
export const NAME_CHARS = 64;

/** Основа имени: латиница, цифры, `_` и `-`. */
const STEM = /^[A-Za-z0-9_-]+$/;

/**
 * Род файла по имени; `null` - расширение не рода проекта.
 *
 * Длинное расширение проверяется раньше короткого: `.takt-ui` и `.takt-map` не
 * кончаются на `.takt`, но порядок делает правило независимым от формы расширений.
 *
 * @param {string} name имя файла
 * @returns {{kind: string, label: string, extension: string}|null} род
 */
export function kindOf(name) {
  const longest = [...FILE_KINDS].sort((a, b) => b.extension.length - a.extension.length);
  return longest.find((item) => String(name).endsWith(item.extension)) ?? null;
}

/**
 * Значение атрибута `accept` поля выбора файла.
 *
 * @returns {string} расширения через запятую
 */
export function accept() {
  return FILE_KINDS.map((item) => item.extension).join(",");
}

/**
 * Отказ загрузки файла с диска, известный до рейса; `null` - файл годен.
 *
 * @param {string} name имя файла
 * @param {number} size размер в байтах
 * @returns {{key: string, params: object}|null} ключ словаря и подстановки
 */
export function uploadRefusal(name, size) {
  const kind = kindOf(name);
  if (!kind) {
    return { key: "file.badKind", params: { name } };
  }
  const stem = name.slice(0, -kind.extension.length);
  if (!STEM.test(stem) || name.length > NAME_CHARS) {
    return { key: "file.badUploadName", params: { name } };
  }
  if (size > LIMIT_BYTES) {
    return { key: "file.tooBig", params: { name, limit: Math.floor(LIMIT_BYTES / 1024) } };
  }
  return null;
}
