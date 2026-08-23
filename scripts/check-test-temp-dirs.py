#!/usr/bin/env python3
"""Гейт уникальности временных каталогов тестов (фикс 0190-01).

Инвариант 0190: тесты идут ПАРАЛЛЕЛЬНО, а каждый помощник начинает работу с
`remove_dir_all` своего каталога. Значит каталог обязан быть уникален по тесту
— иначе один тест сносит рабочий каталог другого прямо во время сборки, и
падение выглядит как сбой чужого инструмента («Undefined symbols _main»), а не
как гонка.

Замер 2026-08-23: из 234 мест построения каталога 132 обходятся без имени
потока — и это НЕ дефект сам по себе: уникальным именем каталог тоже уникален.
Настоящий признак — СОВПАДЕНИЕ, и его было ровно одно: шаблон
`takt_conformance_sv_{tag}` в двух файлах сразу.

Классы:
  D1 — один и тот же литеральный каталог в разных файлах;
  D2 — один и тот же шаблон `format!` без имени потока в разных файлах;
  D3 — один и тот же тег `build_dir("…")` дважды в файле, чей помощник не
       берёт имя потока.

⚠️ Гейт проверяет ИМЕНА, а не поведение: тест, строящий имя вычислением, ему
невидим. Граница названа — предмет в том, чтобы совпадение не заводилось
незаметно.

Корень переопределяется переменной `TD_ROOT` (фича 0315) — для сторожа гейта.
"""

import collections
import os
import re
import sys

ROOT = os.environ.get("TD_ROOT") or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TEST_DIRS = ("takt-sim/tests", "takt-lang/tests")

LITERAL = re.compile(r'temp_dir\(\)\s*\.join\(\s*"([^"]+)"')
TEMPLATE = re.compile(r'temp_dir\(\)\s*\.join\(format!\(\s*"([^"]+)"')
TAG = re.compile(r'build_dir\(\s*"([^"]+)"\s*\)')


def sources():
    for base in TEST_DIRS:
        root = os.path.join(ROOT, base)
        if not os.path.isdir(root):
            continue
        for directory, _, files in os.walk(root):
            for name in files:
                if name.endswith(".rs"):
                    yield os.path.join(directory, name)


def main() -> int:
    literals = collections.defaultdict(set)
    templates = collections.defaultdict(set)
    problems = []
    files = 0

    for path in sources():
        files += 1
        text = open(path, encoding="utf-8").read()
        flat = " ".join(text.split())
        rel = os.path.relpath(path, ROOT)
        for match in LITERAL.finditer(flat):
            literals[match.group(1)].add(rel)
        for match in TEMPLATE.finditer(flat):
            # Имя потока ищется рядом с построением — там, где строится ключ.
            window = flat[max(0, match.start() - 300) : match.start() + 300]
            if "thread" in window:
                continue
            templates[match.group(1)].add(rel)
        if "thread" not in text:
            counts = collections.Counter(TAG.findall(text))
            for tag, count in sorted(counts.items()):
                if count > 1:
                    problems.append(
                        f"D3: {rel} — тег '{tag}' использован {count} раза, "
                        "а имя потока в ключ не входит"
                    )

    for name, where in sorted(literals.items()):
        if len(where) > 1:
            problems.append(f"D1: каталог '{name}' общий у файлов: {', '.join(sorted(where))}")
    for name, where in sorted(templates.items()):
        if len(where) > 1:
            problems.append(f"D2: шаблон '{name}' общий у файлов: {', '.join(sorted(where))}")

    if problems:
        print("ОТКАЗ: временные каталоги тестов пересекаются:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        print(
            "\nТесты идут параллельно (фича 0190), и помощник начинает с\n"
            "remove_dir_all: совпавший каталог значит снос чужой сборки.\n"
            "Ключ уникальности — имя потока: харнесс Rust называет поток\n"
            "именем теста (двоеточия вычищать).",
            file=sys.stderr,
        )
        return 1

    print(f"Временные каталоги тестов: проверено {files} файлов, пересечений нет.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
