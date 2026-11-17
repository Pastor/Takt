#!/usr/bin/env python3
"""Объединяет архивы WASM в каталоге новой сборки после проверки файлов."""
import argparse
import hashlib
import json
import re
import shutil
from pathlib import Path

MODULES = ("takt.wasm", "takt-export.wasm")
VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")


def read_json(path):
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: ожидался объект JSON")
    return value


def read_archive(root, *, allow_core_only=False):
    # Снимок контейнера должен содержать обычные файлы и каталоги.
    if root.is_symlink():
        raise ValueError(f"{root}: архив не может быть символьной ссылкой")
    for path in root.rglob("*"):
        if path.is_symlink() or not (path.is_file() or path.is_dir()):
            raise ValueError(f"{path}: недопустимый тип файла в архиве")
    index = read_json(root / "index.json")
    versions = index["versions"]
    if (not isinstance(versions, list)
            or any(not isinstance(v, str) or not VERSION.fullmatch(v) for v in versions)
            or len(versions) != len(set(versions))
            or index["latest"] not in versions
            or set(versions) != {p.name for p in root.iterdir() if p.is_dir()}):
        raise ValueError(f"{root}: индекс не соответствует каталогам версий")

    sums = {}
    for version in versions:
        directory = root / version
        manifest = read_json(directory / "manifest.json")
        export = manifest.get("export")
        if manifest["takt_lang"] != version:
            raise ValueError(f"{directory}: неверная версия в описи WASM")
        descriptions = [(MODULES[0], manifest)]
        if isinstance(export, dict) and export.get("file") == MODULES[1]:
            descriptions.append((MODULES[1], export))
        elif not (allow_core_only and "export" not in manifest and not (directory / MODULES[1]).exists()):
            raise ValueError(f"{directory}: неверная опись экспорта WASM")
        for module, description in descriptions:
            path = directory / module
            sha = hashlib.sha256(path.read_bytes()).hexdigest()
            if sha != description["sha256"] or path.stat().st_size != description["size"]:
                raise ValueError(f"{path}: SHA-256 или размер не совпадает с описью")
            sums[version, module] = sha
    return index, sums


def merge(candidate, published, *, index_only):
    index, new_sums = read_archive(candidate)
    old_index, old_sums = read_archive(published, allow_core_only=index_only)
    for version, module in sorted(new_sums):
        if version in old_index["versions"] and old_sums.get((version, module)) != new_sums[version, module]:
            raise ValueError(
                f"wasm/{version}/{module}: уже опубликован другой модуль "
                f"({old_sums.get((version, module), 'нет файла')} → {new_sums[version, module]}). "
                "Поднимите версию takt-lang."
            )

    # Сохраняем прежнюю опись (включая built_at) и предсжатые файлы.
    if not index_only:
        for version in old_index["versions"]:
            destination = candidate / version
            if destination.exists():
                shutil.rmtree(destination)
            shutil.copytree(published / version, destination)
    index["versions"] = sorted(
        set(index["versions"]) | set(old_index["versions"]),
        key=lambda version: tuple(map(int, version.split("."))),
    )
    (candidate / "index.json").write_text(json.dumps(index, indent=2) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--index-only", action="store_true",
        help="обновить индекс без копирования уже опубликованных файлов",
    )
    parser.add_argument("candidate", type=Path)
    parser.add_argument("published", type=Path, nargs="?")
    args = parser.parse_args()
    try:
        if args.published is not None and args.published.exists():
            merge(args.candidate, args.published, index_only=args.index_only)
        else:
            read_archive(args.candidate)
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(1, f"Ошибка подготовки WASM: {error}\n")


if __name__ == "__main__":
    main()
