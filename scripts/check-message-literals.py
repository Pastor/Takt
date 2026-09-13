#!/usr/bin/env python3
"""check-message-literals.py — текст сообщения живёт в каталоге, а не в коде (фича 0532).

Сообщение, написанное строковым литералом мимо каталога `takt-lang/messages/`, под
`--lang en` приходит по-русски, и ни одна другая проверка этого не видит: паритет
каталогов (`check-messages.py`) судит каталог, а не места эмиссии.

Замер при заведении проверки нашёл класс, из-за которого она устроена так, а не
грепом: сканер задач 02–05 искал литерал в пределах одной строки исходника, и
многострочные сообщения с продолжением `\\` — два десятка с лишним — прошли мимо.
Здесь исходник разбирает мини-лексер: обычные и сырые строки, символьные литералы,
вложенные блочные комментарии. Комментарии маскируются, и слово `panic!` в
комментарии рядом с сообщением его не прячет.

Проверки (падают СПИСКОМ):

* `L1` - литерал с кириллицей вне тестов и вне реестра допустимых мест.
* `L2` - у файла из реестра число литералов разошлось с записью. Выросло — в файл
  добавлен русский текст, а реестр называет категорию, а не выдаёт индульгенцию.
  Уменьшилось — запись обязана быть опущена (ратчет: долг не растёт и не зависает).
* `L3` - запись реестра, чей файл не существует.

Отсеиваются: файлы тестов (`*tests.rs`, `tests*.rs`, каталоги `tests/`), код после
`#[cfg(test)] mod` и литералы в `assert!`, `expect`, `panic!`, `unreachable!`,
`todo!`, `must_use`, под `#[cfg(test)]` — они описывают нарушенный инвариант кода,
а не ошибку модели.

Реестр — `scripts/message-literals.txt`: `<файл> <число> # причина`. Причина —
категория текста, который сообщением не является: порождённый код, данные трассы,
недостижимый запасной ответ.

Контроль — `scripts/test-check-message-literals.sh`; корень переопределяет `ML_ROOT`.
"""

import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gatelib import require_input  # noqa: E402  (путь к помощнику известен только здесь)

ROOT = os.environ.get("ML_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
REGISTRY = os.path.join(ROOT, "scripts", "message-literals.txt")
CRATES = ("takt-lang", "takt-sim", "takt-wasm", "takt-wasm-io", "takt-wasm-export")
CYRILLIC = re.compile("[А-Яа-яЁё]")
SKIP = re.compile(
    r"assert\w*!|\.expect\(|panic!|unreachable!|todo!|must_use|cfg\(test\)"
)
TEST_MODULE = re.compile(r"#\[cfg\(test\)\]\s*mod\s+\w+")


def lex(src):
    """Литералы `(строка, начало, текст)` и исходник с замаскированными комментариями."""
    out, masked = [], list(src)
    i, n, line = 0, len(src), 1

    def blank(a, b):
        for k in range(a, b):
            if masked[k] != "\n":
                masked[k] = " "

    while i < n:
        c = src[i]
        if c == "\n":
            line += 1
            i += 1
        elif src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
        elif src.startswith("/*", i):
            start, depth, i = i, 1, i + 2
            while i < n and depth:
                if src.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif src.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    line += src[i] == "\n"
                    i += 1
            blank(start, i)
        elif (m := re.match(r'b?r(#*)"', src[i:i + 12])) and not (
            i and (src[i - 1].isalnum() or src[i - 1] == "_")
        ):
            end = '"' + m.group(1)
            j = src.find(end, i + m.end())
            j = n if j < 0 else j
            body = src[i + m.end():j]
            out.append((line, i, body))
            line += body.count("\n")
            i = j + len(end)
        elif c == '"':
            start, first, i = i, line, i + 1
            buf = []
            while i < n and src[i] != '"':
                if src[i] == "\\":
                    buf.append(src[i:i + 2])
                    line += src[i + 1:i + 2] == "\n"
                    i += 2
                else:
                    buf.append(src[i])
                    line += src[i] == "\n"
                    i += 1
            out.append((first, start, "".join(buf)))
            i += 1
        elif c == "'" and (
            m := re.match(r"'(?:\\u\{[0-9a-fA-F]+\}|\\.|[^\\'])'", src[i:i + 12])
        ):
            i += m.end()
        else:
            i += 1
    return out, "".join(masked)


def context(masked, start):
    """Код оператора до литерала: от ближайших `;`, `{`, `}` назад."""
    k = max(masked.rfind(";", 0, start), masked.rfind("{", 0, start), masked.rfind("}", 0, start))
    return masked[k + 1:start]


def is_test_file(rel):
    name = os.path.basename(rel)
    return (
        f"{os.sep}tests{os.sep}" in rel
        or name.endswith("tests.rs")
        or name.startswith("tests")
    )


def scan():
    """Файл → строки литералов с кириллицей; второе значение — число просмотренных файлов."""
    found, seen = {}, 0
    for crate in CRATES:
        src_dir = os.path.join(ROOT, crate, "src")
        for base, _, files in sorted(os.walk(src_dir)):
            for name in sorted(files):
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(base, name)
                rel = os.path.relpath(path, ROOT)
                if is_test_file(rel):
                    continue
                seen += 1
                with open(path, encoding="utf-8") as handle:
                    src = handle.read()
                cut = TEST_MODULE.search(src)
                if cut:
                    src = src[:cut.start()]
                literals, masked = lex(src)
                lines = [
                    ln for ln, start, body in literals
                    if CYRILLIC.search(body) and not SKIP.search(context(masked, start))
                ]
                if lines:
                    found[rel] = lines
    return found, seen


def read_registry(errors):
    allowed = {}
    if not os.path.exists(REGISTRY):
        return allowed
    with open(REGISTRY, encoding="utf-8") as handle:
        for no, raw in enumerate(handle, 1):
            line = raw.split("#", 1)[0].strip()
            if not line:
                continue
            parts = line.split()
            if len(parts) != 2 or not parts[1].isdigit():
                errors.append(f"L3 реестр:{no}: ожидалось '<файл> <число> # причина'")
                continue
            allowed[parts[0]] = int(parts[1])
    return allowed


def main():
    if "--list" in sys.argv:
        found, _ = scan()
        for rel, lines in sorted(found.items()):
            print(f"{rel} {len(lines)}")
        return 0

    errors = []
    allowed = read_registry(errors)
    found, seen = scan()
    for rel, lines in sorted(found.items()):
        want = allowed.get(rel)
        if want is None:
            for ln in lines:
                errors.append(f"L1 {rel}:{ln}: литерал с кириллицей мимо каталога сообщений")
        elif len(lines) != want:
            errors.append(
                f"L2 {rel}: литералов {len(lines)}, реестр — {want} "
                f"(строки {', '.join(map(str, lines))})"
            )
    for rel, want in sorted(allowed.items()):
        if not os.path.exists(os.path.join(ROOT, rel)):
            errors.append(f"L3 {rel}: запись реестра без файла")
        elif rel not in found:
            errors.append(f"L2 {rel}: литералов 0, реестр — {want}: опустите запись")

    if errors:
        print("Проверка литералов сообщений (0532): нарушения", file=sys.stderr)
        for line in errors:
            print(f"  {line}", file=sys.stderr)
        print(
            "\nТекст, который видит автор модели, строится каталогом "
            "(`takt-lang/messages/`, макрос `msg!`); реестр `scripts/message-literals.txt` "
            "называет лишь то, что сообщением не является.",
            file=sys.stderr,
        )
        return 1

    note = require_input("исходники крейтов с каталогом сообщений", seen, minimum=50)
    print(f"  литералы сообщений: {note}; мимо каталога — ни одного, реестр {len(allowed)} файлов")
    return 0


if __name__ == "__main__":
    sys.exit(main())
