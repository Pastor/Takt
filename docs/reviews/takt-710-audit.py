#!/usr/bin/env python3
"""Без записи: TSV на stdout, сводка на stderr. Вход — два дерева Git."""

import argparse
import csv
import json
import posixpath
import re
import runpy
import subprocess
import sys
import unicodedata
from tempfile import TemporaryDirectory
from collections import Counter
from functools import lru_cache
from pathlib import Path
from urllib.parse import quote, unquote, urlsplit

BASE = "89ba01950875cfed51c99dad3c27fc7578eaf4d5"
ARCHIVE = "3c154a13d979b18312a1250ebfbe3db2fa5247d3"
GITHUB = "https://github.com/Pastor/Takt"
SOURCE = re.compile(r"Источник: \[([^\]]+)\]\(([^)]+)\)\n\n")
LINK = re.compile(r"(?<!\\)(?P<image>!)?\[(?:[^\[\]\n]|\\.)*\]\(\s*(?P<url>[^()\s]+)\s*\)")
GATE = re.compile(r"\]\(\s*([^)\s#][^)\s]*?)\s*\)")
FENCE = re.compile(r"^\s*(`{3,}|~{3,})")
CODE = re.compile(r"(`+)[^`]*?\1")
HEADING = re.compile(r"^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$")
SAMPLE = {10, 18, 34, 598}


def git(*args):
    return subprocess.check_output(["git", *args])


def tree(revision):
    entries = {}
    for row in git("ls-tree", "-rtz", revision).decode().split("\0"):
        if row:
            details, path = row.split("\t", 1)
            mode, kind, _ = details.split()
            entries[path] = (mode, kind)
    return entries


@lru_cache(maxsize=None)
def blob(revision, path):
    return git("show", f"{revision}:{path}").decode()


def prose(body):
    offset = 0
    fence = None
    for number, line in enumerate(body.splitlines(keepends=True), 1):
        marker = FENCE.match(line)
        if marker and fence is None:
            fence = marker[1]
        elif marker and marker[1][0] == fence[0] and len(marker[1]) >= len(fence):
            if not line[marker.end():].strip():
                fence = None
        elif fence is None:
            masked = CODE.sub(lambda match: " " * len(match[0]), line)
            yield (number, offset, line, masked)
        offset += len(line)


@lru_cache(maxsize=None)
def headings(path):
    # Приближение GitHub для обычных ATX-заголовков. Сложную разметку
    # и повторы проверяет человек на архивной странице.
    slugs = Counter()
    ids = set()
    for _, _, line, _ in prose(blob(ARCHIVE, path)):
        match = HEADING.match(line)
        if match:
            name = match[1].replace("`", "").lower()
            slug = "".join(
                char for char in name
                if char in " -_" or unicodedata.category(char)[0] in "LMN"
            ).replace(" ", "-")
            suffix = slugs[slug]
            candidate = f"{slug}-{suffix}" if suffix else slug
            while candidate in ids:
                suffix += 1
                candidate = f"{slug}-{suffix}"
            slugs[slug] += 1
            ids.add(candidate)
    return slugs, ids


def destination(source, old, archive, current):
    parts = urlsplit(old)
    path, separator, fragment = old.partition("#")
    if not path or parts.scheme or parts.netloc or path.startswith("/"):
        return "", "", fragment, "special-address"
    if parts.query or "%" in path or "\\" in path:
        return "", "", fragment, "encoded-or-query-path"
    resolved = posixpath.normpath(posixpath.join(posixpath.dirname(source), path))
    entry = archive.get(resolved)
    if entry is None:
        return resolved, "", fragment, "archive-path-missing"
    mode, kind = entry
    if mode == "120000" or kind not in {"blob", "tree"}:
        return resolved, "", fragment, "special-git-object"
    url = f"{GITHUB}/{kind}/{ARCHIVE}/{quote(resolved, safe='/')}"
    if separator:
        url += "#" + fragment
    if current:
        return resolved, url, fragment, "also-resolves-in-export"
    if not separator:
        return resolved, url, fragment, "path"
    if kind != "blob" or not resolved.endswith(".md"):
        return resolved, url, fragment, "anchor-needs-review"
    slugs, ids = headings(resolved)
    decoded = unquote(fragment)
    if decoded not in ids:
        return resolved, url, fragment, "anchor-unverified"
    if slugs[decoded] > 1 or (decoded not in slugs and decoded.rsplit("-", 1)[0] in slugs):
        return resolved, url, fragment, "anchor-duplicate"
    return resolved, url, fragment, "anchor-heading"


def audit(writer, sample):
    archive, current = tree(ARCHIVE), tree(BASE)
    totals = Counter()
    gate = runpy.run_path("scripts/check-links.py")["check"]
    for number in range(1, 699):
        key = f"TAKT-{number}"
        export = f"tasks/{key}.md"
        original = blob(BASE, export)
        _, front, body = original.split("---\n", 2)
        source_match = SOURCE.match(body.lstrip("\n"))
        if source_match is None:
            raise ValueError(f"Нет источника для {key}")
        source, source_url = source_match.groups()
        if source_url != f"{GITHUB}/blob/{ARCHIVE}/{source}":
            raise ValueError(f"Неожиданный источник для {key}")
        historical = blob(ARCHIVE, source)
        totals["cards"] += 1
        totals["distinct_archive_bytes"] += body.lstrip("\n")[source_match.end():] != historical
        offset = len(original) - len(body)
        originals = Counter((line, match[1]) for _, _, line, masked in prose(historical)
                            for match in GATE.finditer(masked))
        changes = []
        files_broken = 0
        reported = []
        for line_no, line_offset, line, masked in prose(body):
            simple = {match.start("url"): match for match in LINK.finditer(line)}
            for match in GATE.finditer(masked):
                start, end = match.span(1)
                old = line[start:end]
                if old.startswith(("http://", "https://", "mailto:", "ftp://")):
                    totals["external_in_gate"] += 1
                    continue
                resolved_here = posixpath.normpath(posixpath.join("tasks", old.split("#", 1)[0]))
                present = resolved_here in current
                files_broken += not present
                target, url, fragment, reason = destination(source, old, archive, present)
                totals["directory_targets"] += bool(target and archive.get(target, ("", ""))[1] == "tree")
                totals["fragments"] += "#" in old
                link = simple.get(start)
                if not link or link["url"] != old:
                    reason = "syntax-needs-review"
                elif link["image"]:
                    reason = "image-needs-review"
                evidence = (line, old)
                if not originals[evidence]:
                    totals["no_exact_archive_line"] += 1
                    reason = "archive-line-needs-review"
                else:
                    originals[evidence] -= 1
                totals[reason] += 1
                reported.append((key, re.search(r"^  id: (\d+)$", front, re.M)[1],
                                 re.search(r"^  version: (\d+)$", front, re.M)[1],
                                 export, line_no + original[:offset].count("\n"),
                                 start + 1, source, old, target, url, fragment, reason))
                if reason in {"path", "anchor-heading"}:
                    changes.append((line_offset + start, line_offset + end, old, url))
        totals["files_with_broken"] += bool(files_broken)
        totals["broken"] += files_broken
        # В памяти проверяем замену и обратную замену по паре смещений.
        pieces, undo, cursor, rendered_offset = [], [], 0, 0
        for start, end, old, url in changes:
            unchanged = body[cursor:start]
            pieces.extend((unchanged, url))
            rendered_offset += len(unchanged)
            undo.append((rendered_offset, url, old))
            rendered_offset += len(url)
            cursor = end
        proposed = "".join(pieces) + body[cursor:]
        restored = proposed
        for start, url, old in reversed(undo):
            assert restored[start:start + len(url)] == url
            restored = restored[:start] + old + restored[start + len(url):]
        assert restored == body
        old_length = len(body.lstrip("\n"))
        new_length = len(proposed.lstrip("\n"))
        totals["already_over_50000"] += old_length > 50000
        totals["proposed_over_50000"] += new_length > 50000
        if new_length > 50000:
            totals["ready_links_in_oversize_cards"] += len(changes)
        for row in reported:
            writer.writerow((*row, old_length, new_length))
        if number in SAMPLE:
            with TemporaryDirectory() as tmp:
                page = Path(tmp) / "tasks" / f"{key}.md"
                page.parent.mkdir()
                page.write_text(original, encoding="utf-8")
                before = len(gate(page))
                page.write_text(original[:offset] + proposed, encoding="utf-8")
                after = len(gate(page))
                assert before == files_broken and after == before - len(changes)
                totals["sample_gate_before"] += before
                totals["sample_gate_after"] += after
            if sample:
                from difflib import unified_diff
                sys.stderr.writelines(unified_diff(
                    original.splitlines(keepends=True),
                    (original[:offset] + proposed).splitlines(keepends=True),
                    fromfile=export, tofile=export + " (локальная проекция)",
                ))
            totals["sample_changes"] += len(changes)
            totals["sample_held"] += len(reported) - len(changes)
            totals["sample_cards"] += 1
            print(f"{key}: {len(changes)} замен, {len(reported) - len(changes)} на сверку", file=sys.stderr)

    print(json.dumps(totals, ensure_ascii=False, sort_keys=True), file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sample-diff", action="store_true")
    args = parser.parse_args()
    if not Path("tasks").is_dir():
        parser.error("Запустите из корня Takt")
    writer = csv.writer(sys.stdout, delimiter="\t", lineterminator="\n")
    writer.writerow(("card", "anal_id", "export_version", "export_path", "line", "column",
                     "source", "old", "archive_path", "proposal", "fragment", "reason",
                     "old_chars", "projected_chars"))
    audit(writer, args.sample_diff)


if __name__ == "__main__":
    main()
