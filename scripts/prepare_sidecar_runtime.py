#!/usr/bin/env python3
"""Prepare sidecar runtime resources for Tauri bundling.

Copies Data-Analysis-Agent into src-tauri/resources/Data-Analysis-Agent,
including .venv, while excluding runtime outputs and cache files.
"""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path

EXCLUDED_DIR_NAMES = {"outputs", "uploads", "__pycache__"}
EXCLUDED_FILE_SUFFIXES = {".pyc"}
EXCLUDED_FILE_NAMES = {".DS_Store"}


def should_skip(path: Path) -> bool:
    if any(part in EXCLUDED_DIR_NAMES for part in path.parts):
        return True
    if path.name in EXCLUDED_FILE_NAMES:
        return True
    if path.suffix in EXCLUDED_FILE_SUFFIXES:
        return True
    return False


def copy_tree(src: Path, dst: Path) -> None:
    if dst.exists():
        shutil.rmtree(dst)
    dst.mkdir(parents=True, exist_ok=True)

    for item in src.rglob("*"):
        rel = item.relative_to(src)
        if should_skip(rel):
            continue

        target = dst / rel
        if item.is_dir():
            target.mkdir(parents=True, exist_ok=True)
            continue

        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(item, target)


def verify_venv(dst: Path) -> None:
    unix_py = dst / ".venv" / "bin" / "python"
    win_py = dst / ".venv" / "Scripts" / "python.exe"
    if unix_py.exists():
        print(f"[prepare-sidecar-runtime] detected interpreter: {unix_py}")
        return
    if win_py.exists():
        print(f"[prepare-sidecar-runtime] detected interpreter: {win_py}")
        return
    raise SystemExit("[prepare-sidecar-runtime] ERROR: embedded python interpreter not found in .venv")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", default="Data-Analysis-Agent")
    parser.add_argument("--target", default="src-tauri/resources/Data-Analysis-Agent")
    parser.add_argument("--require-venv", action="store_true")
    args = parser.parse_args()

    src = Path(args.source).resolve()
    dst = Path(args.target).resolve()

    app_py = src / "app.py"
    if not app_py.exists():
        raise SystemExit(f"[prepare-sidecar-runtime] ERROR: missing app.py at {app_py}")

    print(f"[prepare-sidecar-runtime] source: {src}")
    print(f"[prepare-sidecar-runtime] target: {dst}")
    copy_tree(src, dst)

    if args.require_venv:
        verify_venv(dst)

    print("[prepare-sidecar-runtime] done")


if __name__ == "__main__":
    main()
