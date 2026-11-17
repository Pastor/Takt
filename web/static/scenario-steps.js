// Срезы JSON оставляют числа и неизвестные поля такими, какими их записал автор.

/** Делит уже проверенный JSON по разделителям одного уровня. */
function split(text, separator) {
  const parts = [];
  let depth = 0;
  let start = 0;
  for (const token of text.matchAll(/\x22(?:[^\x22\\]|\\[\s\S])*\x22|[{}\[\],:]/g)) {
    const [char] = token;
    if (char === "[" || char === "{") depth += 1;
    if (char === "]" || char === "}") depth -= 1;
    if (char === separator && depth === 0) {
      parts.push(text.slice(start, token.index).trim());
      start = token.index + 1;
    }
  }
  const last = text.slice(start).trim();
  if (last) parts.push(last);
  return parts;
}

/** Возвращает тексты шагов после проверки формы массива. */
export function readSteps(text) {
  text = text.trim();
  const steps = JSON.parse(text);
  if (!Array.isArray(steps) || steps.some((step) => !step || Array.isArray(step) || typeof step !== "object")) {
    throw new Error("scenarioEdit.badSteps");
  }
  return split(text.slice(1, -1), ",");
}

/** Читает поля одного шага, сохраняя исходный текст каждого значения. */
export function readFields(step) {
  return new Map(split(step.slice(1, -1), ",").map((entry) => split(entry, ":")).map(
    ([name, value]) => [JSON.parse(name), value],
  ));
}

/** Печатает шаг из полей в исходном порядке. */
export function writeFields(fields) {
  return "{" + Array.from(fields, ([name, value]) => `${JSON.stringify(name)}: ${value}`).join(", ") + "}";
}

/** Печатает один шаг на строку и оставляет пустые такты в массиве. */
export function writeSteps(steps) {
  return steps.length ? `[\n  ${steps.join(",\n  ")}\n]\n` : "[]\n";
}
