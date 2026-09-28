import json
import os
import re
import shutil
import subprocess
import time
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import NotRequired, TypedDict, cast

import httpx

from bench import openrouter
from bench.limits import LimitReachedError, StageError

MODEL = "typesafe/jev-1.13"
BINARY_VARIABLE = "BENCH_JEVPIPE"
CONCURRENCY = 100
_SYSTEM_ONE = f"{openrouter.API}/systemone"
_LIMIT_EXIT = 3
_ACCEPTED_EXITS = (0, 2)
_COST = re.compile(r"\$(\d+(?:\.\d+)?)")


@dataclass(frozen=True)
class JevRun:
    probability: tuple[float | None, ...]
    failed: int
    skipped: int
    cost: float
    wall_seconds: float


@dataclass(frozen=True)
class Parsed:
    probability: tuple[float | None, ...]
    failed: int
    skipped: int


class _Answer(TypedDict):
    noul: float


class _Line(TypedDict):
    record: str
    answers: NotRequired[dict[str, _Answer]]
    outcome: NotRequired[str]


class _SystemOneReply(TypedDict):
    model: str


def questions(question: str) -> str:
    return json.dumps({"match": {"type": "noul", "instructions": question}})


def binary() -> str:
    chosen = os.environ.get(BINARY_VARIABLE) or shutil.which("jevpipe")
    if chosen is None:
        message = f"jevpipe is not on the path and {BINARY_VARIABLE} is not set"
        raise StageError(message)
    return chosen


def version() -> str:
    result = subprocess.run(
        [binary(), "--version"], capture_output=True, text=True, encoding="utf-8", check=True
    )
    return result.stdout.strip()


def run(names: Sequence[str], question: str, folder: Path) -> JevRun:
    command = [
        binary(),
        "map",
        "--read-files",
        *("--model", MODEL),
        *("--concurrency", str(CONCURRENCY)),
        *("-q", questions(question)),
    ]
    started = time.perf_counter()
    result = subprocess.run(
        command,
        input="".join(f"{name}\n" for name in names),
        cwd=folder,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    wall_seconds = time.perf_counter() - started
    if result.returncode == _LIMIT_EXIT:
        raise LimitReachedError(result.stderr.strip())
    if result.returncode not in _ACCEPTED_EXITS:
        message = f"jevpipe exited with {result.returncode}: {result.stderr.strip()}"
        raise StageError(message)
    parsed = parse(names, result.stdout)
    return JevRun(
        parsed.probability, parsed.failed, parsed.skipped, cost_of(result.stderr), wall_seconds
    )


def parse(names: Sequence[str], stdout: str) -> Parsed:
    answered: dict[str, float] = {}
    skipped = 0
    for text in stdout.splitlines():
        line = cast("_Line", json.loads(text))
        if "answers" in line:
            answered[line["record"]] = line["answers"]["match"]["noul"]
        elif line.get("outcome") == "skipped":
            skipped += 1
    probability = tuple(answered.get(name) for name in names)
    failed = sum(value is None for value in probability) - skipped
    return Parsed(probability, failed, skipped)


def cost_of(stderr: str) -> float:
    summaries = [line for line in stderr.splitlines() if " records, " in line]
    if not summaries:
        return 0.0
    match = _COST.search(summaries[-1])
    return float(match.group(1)) if match else 0.0


def resolved_version() -> str:
    body = {
        "model": MODEL,
        "state": "def add(a, b):\n    return a + b",
        "questions": {"match": {"type": "noul", "instructions": "Is this Python code?"}},
    }
    response = httpx.post(_SYSTEM_ONE, json=body, headers=openrouter.authorization(), timeout=60)
    if not response.is_success:
        message = f"the Jev version call failed with {response.status_code}: {response.text}"
        raise StageError(message)
    return cast("_SystemOneReply", response.json())["model"]
