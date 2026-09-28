import json
import re
import subprocess
import time
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import NotRequired, TypedDict, cast

import httpx

from bench import openrouter, tools
from bench.limits import LimitReachedError, StageError

MODEL = "typesafe/jev-1.13"
LABEL = "Jev 1.13"
_BINARY_VARIABLE = "BENCH_JEVPIPE"
_SYSTEM_ONE = f"{openrouter.API}/systemone"
_SOME_FAILED_EXIT = 2
_LIMIT_EXIT = 3
_TIMEOUT_SECONDS = 1800
_SUMMARY = " records, "
_COST = re.compile(r"\$(\d+(?:\.\d+)?)")


@dataclass(frozen=True)
class Parsed:
    probability: tuple[float | None, ...]
    skipped: int

    @property
    def failed(self) -> int:
        return failed_of(self.probability, self.skipped)


@dataclass(frozen=True)
class JevRun:
    parsed: Parsed
    cost: float
    wall_seconds: float


class _Answer(TypedDict):
    noul: float


class _Line(TypedDict):
    record: str
    answers: NotRequired[dict[str, _Answer]]
    outcome: NotRequired[str]


class _SystemOneReply(TypedDict):
    model: str


def _questions(question: str) -> str:
    return json.dumps({"match": {"type": "noul", "instructions": question}})


def binary() -> str:
    return tools.find("jevpipe", _BINARY_VARIABLE)


def version() -> str:
    return tools.version(binary())


def run(names: Sequence[str], question: str, folder: Path) -> JevRun:
    command = [
        binary(),
        "map",
        "--read-files",
        *("--model", MODEL),
        *("--concurrency", str(openrouter.IN_FLIGHT)),
        *("-q", _questions(question)),
    ]
    started = time.perf_counter()
    try:
        result = subprocess.run(
            command,
            input="".join(f"{name}\n" for name in names),
            cwd=folder,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=_TIMEOUT_SECONDS,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        message = f"jevpipe did not finish within {_TIMEOUT_SECONDS} s"
        raise StageError(message) from error
    wall_seconds = time.perf_counter() - started
    _check(result)
    return JevRun(parse(names, result.stdout), cost_of(result.stderr), wall_seconds)


def _check(result: subprocess.CompletedProcess[str]) -> None:
    stderr = result.stderr.strip()
    status = result.returncode
    if status == 0 or (status == _SOME_FAILED_EXIT and _SUMMARY in stderr):
        return
    if status == _LIMIT_EXIT:
        raise LimitReachedError(stderr)
    message = f"jevpipe exited with {status}: {stderr}"
    raise StageError(message)


def parse(names: Sequence[str], stdout: str) -> Parsed:
    answered: dict[str, float] = {}
    skipped = 0
    for text in stdout.splitlines():
        line = cast("_Line", json.loads(text))
        if "answers" in line:
            answered[line["record"]] = line["answers"]["match"]["noul"]
        elif line.get("outcome") == "skipped":
            skipped += 1
    return Parsed(tuple(answered.get(name) for name in names), skipped)


def failed_of(probability: Sequence[float | None], skipped: int) -> int:
    return sum(value is None for value in probability) - skipped


def cost_of(stderr: str) -> float:
    summaries = [line for line in stderr.splitlines() if _SUMMARY in line]
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
