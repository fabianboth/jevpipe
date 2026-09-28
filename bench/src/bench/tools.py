import os
import shutil
import subprocess

from bench.limits import StageError


def find(name: str, override: str = "") -> str:
    chosen = (os.environ.get(override) if override else None) or shutil.which(name)
    if chosen is None:
        also = f" and {override} is not set" if override else ""
        message = f"{name} is not on the path{also}"
        raise StageError(message)
    return chosen


def version(binary: str) -> str:
    result = subprocess.run(
        [binary, "--version"], capture_output=True, text=True, encoding="utf-8", check=True
    )
    return result.stdout.splitlines()[0].strip()
