import re
import subprocess
import time
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from bench import pool, tools
from bench.limits import StageError

_FILLERS = frozenset(
    {
        "a",
        "an",
        "the",
        "to",
        "of",
        "from",
        "in",
        "on",
        "for",
        "and",
        "or",
        "is",
        "how",
        "with",
        "into",
        "by",
        "based",
        "another",
        "x",
    }
)
_REGEX_ERROR = 2


@dataclass(frozen=True)
class Hits:
    indexes: tuple[int, ...]
    seconds: float


def keywords(query: str) -> tuple[str, ...]:
    words = re.findall(r"[a-z0-9]+", query.lower())
    return tuple(dict.fromkeys(word for word in words if word not in _FILLERS))


def any_keyword(folder: Path, words: Sequence[str]) -> Hits:
    if not words:
        return Hits((), 0.0)
    return _search(folder, ["-F", *(argument for word in words for argument in ("-e", word))])


def all_keywords(folder: Path, words: Sequence[str]) -> Hits:
    if not words:
        return Hits((), 0.0)
    searches = [_search(folder, ["-F", "-e", word]) for word in words]
    common = set(searches[0].indexes).intersection(*(search.indexes for search in searches[1:]))
    return Hits(tuple(sorted(common)), sum(search.seconds for search in searches))


def pattern(folder: Path, regex: str) -> Hits:
    return _search(folder, ["-e", regex])


def compiles(regex: str) -> bool:
    result = subprocess.run(
        [_ripgrep(), "--no-config", "-e", regex, "-"],
        input="",
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    return result.returncode != _REGEX_ERROR


def version() -> str:
    return tools.version(_ripgrep())


def _search(folder: Path, arguments: list[str]) -> Hits:
    command = [
        _ripgrep(),
        "--no-config",
        "--no-ignore",
        "--files-with-matches",
        "--ignore-case",
        *arguments,
        ".",
    ]
    started = time.perf_counter()
    result = subprocess.run(
        command, cwd=folder, capture_output=True, text=True, encoding="utf-8", check=False
    )
    seconds = time.perf_counter() - started
    if result.returncode == _REGEX_ERROR:
        message = f"ripgrep failed: {result.stderr.strip()}"
        raise StageError(message)
    indexes = sorted(pool.index_of_name(Path(line).name) for line in result.stdout.splitlines())
    return Hits(tuple(indexes), seconds)


def _ripgrep() -> str:
    return tools.find("rg")
