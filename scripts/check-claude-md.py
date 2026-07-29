#!/usr/bin/env python3
"""check-claude-md.py — согласованность живого контекста `CLAUDE.md` (фича 0149).

`CLAUDE.md` читается **в начале каждой сессии**: его факты становятся
предпосылкой всей последующей работы ещё до первого взгляда в код. Поэтому
ошибка здесь стоит дороже ошибки в любом другом документе — а проверялся он до
сих пор ничем.

Замер при заведении гейта нашёл в файле: два места, объявлявших **закрытые**
фичи «в работе» (35 строк текста, включая действующую инструкцию «не используй
вывод C как эталон» — при том что цель `c` компилируется гейтом, а
`conformance_c_tests` служит основным эталоном сверки проекта), один
несуществующий путь и ссылку на номер строки, указывающую не туда.

Проверяются ТОЛЬКО машинно-проверяемые классы:

1. **статус фичи** — «фича NNNN … в работе / не закрыта» против
   `docs/features/README.md`;
2. **путь к файлу** — разрешается полностью либо по хвосту
   (`semantic/tree.rs` → `takt-lang/src/semantic/tree.rs`);
3. **версия крейта** — каноническая форма ``**сейчас `X.Y.Z`**`` против
   `Cargo.toml`;
4. **номер строки** — форма `файл.rs:NNN` ЗАПРЕЩЕНА (гниёт молча).

⚠️ **Версия ЯЗЫКА здесь не проверяется** — она живёт в
`scripts/check-language-version.sh`, который сверяет её сразу в трёх источниках
(константа, README, `CLAUDE.md`). Два гейта, проверяющие один предмет,
неизбежно разъезжаются — проект платил за это дважды.

⚠️ **Истинность утверждений о поведении кода вне объёма.** Гейт не скажет, что
«`Bit` → `int`» неверно: это проверка прозы на истинность. Он лишь не даёт
соседнему факту — статусу фичи, пути — врать молча.

Использование:

    python3 scripts/check-claude-md.py              # проверка
    python3 scripts/check-claude-md.py --self-test  # проверка самой ловушки
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLAUDE_MD = os.path.join(ROOT, "CLAUDE.md")
FEATURES_REGISTRY = os.path.join(ROOT, "docs", "features", "README.md")

# Расширения, по которым строка в обратных кавычках считается путём к файлу.
FILE_SUFFIXES = (".rs", ".toml", ".sh", ".takt", ".lalrpop", ".lua", ".xml", ".yml")

# Упоминания несуществующего, законные ПО СМЫСЛУ. Список короткий и с причиной:
# без него гейт требовал бы завести файлы, которых в проекте нет намеренно.
LEGITIMATELY_ABSENT = {
    "TASKS.md": "плоских списков задач в проекте нет — текст об этом и говорит",
    "STATUS.md": "то же: упоминается как отсутствующее по правилу 10",
    "concat.takt": "гипотетический пример ловушки имён IEC, файла нет и не должно быть",
    "lib/ieclib.txt": "файл внутри префикса установки MatIEC, вне репозитория",
}

# Слова, которыми текст заявляет незавершённость фичи.
UNFINISHED = ("в работе", "не закрыт", "не закрыта", "незакрыт")


def read_lines(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read().splitlines()


def feature_statuses():
    """Номер фичи → статус из реестра `docs/features/README.md`."""
    statuses = {}
    for line in read_lines(FEATURES_REGISTRY):
        match = re.match(r"\|\s*\[(\d{4})\]\([^)]*\)\s*\|[^|]*\|[^|]*\|([^|]*)\|", line)
        if match:
            statuses[match.group(1)] = re.sub(r"[*✅\s]", "", match.group(2))
    return statuses


def build_file_index():
    """Индекс `имя файла` → пути в дереве (для разрешения сокращённых форм)."""
    index = {}
    skip = ("/.git", "/target", "/book/book", "/node_modules")
    for directory, dirnames, names in os.walk(ROOT):
        rel = directory[len(ROOT):]
        if any(s in rel for s in skip):
            dirnames[:] = []
            continue
        for name in names:
            index.setdefault(name, []).append(
                os.path.join(directory, name)[len(ROOT) + 1:]
            )
    return index


def check_feature_status(lines, statuses, problems):
    """Класс 1: закрытая фича, объявленная незавершённой.

    Номер ищется в абзаце, а не в строке: запись «(фичи\\n[0026](…), [0029](…),
    обе в работе)» переносится, и построчный поиск её пропускал — именно так
    35 строк ложного текста и прожили до замера.
    """
    for number, line in enumerate(lines, start=1):
        if not any(word in line for word in UNFINISHED):
            continue
        window = " ".join(lines[max(0, number - 3): number + 2])
        for feature in sorted(set(re.findall(r"\b(0[01]\d\d)\b", window))):
            if statuses.get(feature) == "ГОТОВО":
                problems.append(
                    (number, f"фича {feature} названа незавершённой, а в реестре — ГОТОВО")
                )


def check_paths(lines, index, problems):
    """Класс 2: путь, который не разрешается ни полностью, ни по хвосту."""
    for number, line in enumerate(lines, start=1):
        for path in re.findall(r"`([A-Za-z0-9_\-./]+\.[a-z]+)`", line):
            if not path.endswith(FILE_SUFFIXES) and not path.endswith(".md"):
                continue
            if path in LEGITIMATELY_ABSENT:
                continue
            if os.path.exists(os.path.join(ROOT, path)):
                continue
            candidates = index.get(os.path.basename(path), [])
            if any(c.endswith(path) for c in candidates):
                continue
            problems.append((number, f"путь `{path}` не найден в дереве"))


def check_crate_versions(lines, problems):
    """Класс 3: каноническая форма версии крейта против манифеста."""
    for number, line in enumerate(lines, start=1):
        # ⚠️ Между именем крейта и версией законно стоят другие обратные кавычки
        # (`крейта `takt-lang` (`CARGO_PKG_VERSION`, сейчас `0.20.0`)`), поэтому
        # класс символов их НЕ исключает — первая редакция исключала, и мутация
        # версии проходила мимо гейта.
        for crate, version in re.findall(
            r"`(takt-lang|takt-sim)`[^\n]{0,80}?сейчас `(\d+\.\d+\.\d+)`", line
        ):
            manifest = os.path.join(ROOT, crate, "Cargo.toml")
            actual = None
            for manifest_line in read_lines(manifest):
                match = re.match(r'version\s*=\s*"(\d+\.\d+\.\d+)"', manifest_line)
                if match:
                    actual = match.group(1)
                    break
            if actual != version:
                problems.append(
                    (number, f"крейт {crate}: заявлено {version}, в манифесте {actual}")
                )


def check_line_references(lines, problems):
    """Класс 4: ссылка на номер строки — запрещённая форма.

    Проверять её бессмысленно: строка почти всегда существует, а указывает уже
    не туда (замер: `generator/c/mod.rs:150` — символ уехал на 291). Ссылаться
    надо именем символа: `generator/c/mod.rs::map_c_type`.
    """
    for number, line in enumerate(lines, start=1):
        for reference in re.findall(r"`([A-Za-z0-9_\-./]+\.rs:\d+)`", line):
            problems.append(
                (number, f"ссылка на номер строки `{reference}` — форма запрещена, "
                         "ссылайтесь именем символа (`файл.rs::symbol`)")
            )


def run_checks(lines):
    statuses = feature_statuses()
    index = build_file_index()
    problems = []
    check_feature_status(lines, statuses, problems)
    check_paths(lines, index, problems)
    check_crate_versions(lines, problems)
    check_line_references(lines, problems)
    return sorted(set(problems))


def self_test():
    """Ловушка обязана срабатывать на подложенном тексте каждого класса.

    Без этой проверки «находок нет» неотличимо от «гейт ничего не ищет».
    """
    statuses = feature_statuses()
    closed = next((f for f, s in statuses.items() if s == "ГОТОВО"), None)
    if closed is None:
        sys.exit("САМОПРОВЕРКА ПРОВАЛЕНА: в реестре нет ни одной закрытой фичи")

    cases = {
        "статус фичи": [f"- **Проба** (фича {closed}, в работе). Текст."],
        "путь": ["- Проба `takt-lang/src/nonexistent_zzz.rs` — файла нет."],
        # ⚠️ Образец повторяет РЕАЛЬНУЮ форму записи — с обратными кавычками
        # между именем крейта и версией. Первая редакция самопроверки брала
        # упрощённый образец («`takt-lang` (сейчас `0.0.1`)»), зеленела, и
        # мутация настоящей строки CLAUDE.md проходила мимо гейта: проверялось
        # упрощение, а не та форма, которая в файле встречается.
        "версия крейта": [
            "- Крейт `takt-lang` (`CARGO_PKG_VERSION`, сейчас `0.0.1`) — заведомо не тот."
        ],
        "номер строки": ["- Проба `semantic/tree.rs:999` — запрещённая форма."],
    }
    for name, planted in cases.items():
        found = run_checks(planted)
        if not found:
            sys.exit(f"САМОПРОВЕРКА ПРОВАЛЕНА: класс «{name}» не сработал на подложенном тексте")

    # Обратная сторона: законный текст ложных находок давать не должен.
    clean = [
        "- Проба `semantic/tree.rs` — сокращённая форма, обязана разрешаться.",
        "- Плоских списков задач (`TASKS.md`/`STATUS.md`) нет.",
    ]
    noise = run_checks(clean)
    if noise:
        sys.exit(f"САМОПРОВЕРКА ПРОВАЛЕНА: законный текст дал ложные находки: {noise}")
    print("  самопроверка гейта: ловушка взведена (4 класса ловятся, законный текст — нет)")


def main():
    if "--self-test" in sys.argv[1:]:
        self_test()
        return 0

    lines = read_lines(CLAUDE_MD)
    problems = run_checks(lines)
    if problems:
        print("Расхождения в живом контексте CLAUDE.md (фича 0149):", file=sys.stderr)
        for number, message in problems:
            print(f"  CLAUDE.md:{number}: {message}", file=sys.stderr)
        print(
            "\nЖивой контекст читается в начале каждой сессии — ложный факт здесь\n"
            "становится предпосылкой всей работы. Приведите запись в соответствие\n"
            "с реестром фич, деревом исходников и манифестами.",
            file=sys.stderr,
        )
        return 1
    print(f"Живой контекст CLAUDE.md: проверено {len(lines)} строк, расхождений нет.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
