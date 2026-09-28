import os
import stat
import sys
from pathlib import Path

import pytest

from bench import jev
from bench.limits import LimitReachedError, StageError

_STAND_IN = """import json, os, sys
if sys.argv[1:] == ["--version"]:
    print("jevpipe 9.9.9")
    sys.exit(0)
mode = os.environ["STAND_IN"]
names = sys.stdin.read().split()
if mode == "fatal":
    print("jevpipe: the model typesafe/jev-1.13 does not exist", file=sys.stderr)
    sys.exit(2)
if mode == "limit":
    print("jevpipe: stopped after $0.50; resume from line 1", file=sys.stderr)
    sys.exit(3)
failed = 1 if mode == "some-failed" else 0
for number, name in enumerate(names):
    if number < failed:
        print(json.dumps({"record": name, "outcome": "failed", "reason": "timeout"}))
    else:
        print(json.dumps({"record": name, "answers": {"match": {"type": "noul", "noul": 0.75}}}))
answered = len(names) - failed
summary = f"{len(names)} records, {answered} answered, 0 skipped, {failed} failed"
print(f"jevpipe: {summary}, $0.0012, 0.1s", file=sys.stderr)
sys.exit(2 if failed else 0)
"""
_NAMES = ["0000.py", "0001.py"]


@pytest.fixture(autouse=True)
def stand_in(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    script = tmp_path / "stand_in.py"
    script.write_text(_STAND_IN, encoding="utf-8")
    if os.name == "nt":
        launcher = tmp_path / "jevpipe.cmd"
        launcher.write_text(f'@"{sys.executable}" "{script}" %*\n', encoding="utf-8")
    else:
        launcher = tmp_path / "jevpipe"
        launcher.write_text(f'#!/bin/sh\nexec "{sys.executable}" "{script}" "$@"\n')
        launcher.chmod(launcher.stat().st_mode | stat.S_IEXEC)
    monkeypatch.setenv("BENCH_JEVPIPE", str(launcher))


def _run(mode: str, monkeypatch: pytest.MonkeyPatch, folder: Path) -> jev.JevRun:
    monkeypatch.setenv("STAND_IN", mode)
    return jev.run(_NAMES, "Does this function sort a list?", folder)


def test_every_record_answered(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    found = _run("answered", monkeypatch, tmp_path)

    assert found.parsed.probability == (0.75, 0.75)
    assert (found.parsed.failed, found.cost) == (0, 0.0012)


def test_some_failed_records_still_count(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    found = _run("some-failed", monkeypatch, tmp_path)

    assert found.parsed.probability == (None, 0.75)
    assert found.parsed.failed == 1


def test_a_fatal_error_stops_the_stage(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    with pytest.raises(StageError, match="does not exist"):
        _run("fatal", monkeypatch, tmp_path)


def test_a_spend_limit_is_a_limit(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    with pytest.raises(LimitReachedError):
        _run("limit", monkeypatch, tmp_path)


def test_the_version_comes_from_the_chosen_binary() -> None:
    assert jev.version() == "jevpipe 9.9.9"
