#!/usr/bin/env python3
"""check-links.py — проверка целостности относительных ссылок в Markdown (правило 14).

Ловит именно тот класс дефекта, ради которого правило 14 существует: ссылку,
которая **разрешается не туда** после переезда файлов (инцидент `docs/docs/...`).
Проверка существования цели по пути, вычисленному **относительно файла-источника**,
а не по имени — поэтому ловит и случай, когда файл с таким именем есть в другой
папке (например, `[ADR 0019](0019-*.md)` из `docs/analyze/`, ведущий в анализ).

Использование:
    scripts/check-links.py            # проверить репозиторий
    scripts/check-links.py --quiet    # только код возврата

Код возврата: 0 — битых ссылок нет; 1 — найдены битые.
Внешние ссылки (http/https/mailto) и якоря (#…) не проверяются.
Каталог docs/templates/ пропускается: содержит плейсхолдеры (правило 17).
"""

import os
import re
import sys

# Ссылка вида [текст](цель). Исключаем цели, начинающиеся с '#' (чистые якоря).
LINK_RE = re.compile(r"\]\(\s*([^)\s#][^)\s]*?)\s*\)")
# Инлайновый код-спан: `...` / ``...``. Внутри него `](…)` — не ссылка, а текст
# (так правило 14 цитирует само себя, так же документируются примеры ссылок).
CODE_SPAN_RE = re.compile(r"(`+)[^`]*?\1")
FENCE_RE = re.compile(r"^\s*(```|~~~)")
SKIP_DIRS = {".git", "target", "node_modules", "build", ".gradle"}
SKIP_PATHS = {os.path.join("docs", "templates")}
EXTERNAL = ("http://", "https://", "mailto:", "ftp://")


def markdown_files(root="."):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        rel = os.path.relpath(dirpath, root)
        if any(rel == p or rel.startswith(p + os.sep) for p in SKIP_PATHS):
            continue
        for name in filenames:
            if name.endswith(".md"):
                yield os.path.join(dirpath, name)


def check(path):
    broken = []
    in_fence = False
    with open(path, encoding="utf-8") as handle:
        for lineno, line in enumerate(handle, 1):
            if FENCE_RE.match(line):
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            # Код-спаны вырезаем: внутри них `](…)` — иллюстрация, а не ссылка.
            line = CODE_SPAN_RE.sub("", line)
            for match in LINK_RE.finditer(line):
                target = match.group(1).split("#", 1)[0]
                if not target or target.startswith(EXTERNAL):
                    continue
                resolved = os.path.normpath(os.path.join(os.path.dirname(path), target))
                if not os.path.exists(resolved):
                    broken.append((lineno, target, resolved))
    return broken


def main():
    quiet = "--quiet" in sys.argv
    total = 0
    for path in sorted(markdown_files()):
        for lineno, target, resolved in check(path):
            total += 1
            if not quiet:
                print(f"{path}:{lineno}: битая ссылка -> {target} (ищется как {resolved})")
    if total:
        if not quiet:
            print(f"\nИтого битых ссылок: {total} (правило 14)")
        return 1
    if not quiet:
        print("Битых ссылок нет (правило 14)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
