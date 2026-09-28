from collections.abc import Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import NotRequired, TypedDict, cast

from bench import dataset, deepseek, grep, jev, patterns, pool, store, wording
from bench.limits import StageError

MAX_FAILED_SHARE = 0.01


class GrepRecord(TypedDict):
    version: str
    any: list[int]
    all: list[int]
    agent: list[int]
    seconds: dict[str, float]


class Retried(TypedDict):
    records: int
    answered: int
    version: str
    cost: float
    wall_seconds: float


class JevRecord(TypedDict):
    version: NotRequired[str]
    retried: NotRequired[Retried]
    model: str
    resolved_model: str
    started: str
    first: bool
    wall_seconds: float
    cost: float
    failed: int
    skipped: int
    probability: list[float | None]


class DeepSeekRecord(TypedDict):
    model: str
    resolved_model: str
    started: str
    first: bool
    wall_seconds: float
    cost: float
    failed: int
    providers: dict[str, int]
    answer: list[bool | None]
    probability: list[float | None]


class RepeatFile(TypedDict):
    query: str
    jevpipe: JevRecord
    deepseek: DeepSeekRecord


class RunFile(RepeatFile):
    grep: GrepRecord


@dataclass(frozen=True)
class _Setup:
    names: tuple[str, ...]
    codes: tuple[str, ...]
    templates: dict[wording.Model, str]
    jev_version: str
    jevpipe: str
    folder: Path
    rg_version: str


@dataclass(frozen=True)
class _Models:
    record: RepeatFile
    complete: bool


def runs_folder(suite: store.Suite) -> Path:
    return suite.results / "runs"


def repeats_folder() -> Path:
    return store.PYTHON.results / "repeat"


def run_queries(suite: store.Suite, only: Sequence[str] | None) -> None:
    queries = dataset.load(suite)
    agent_patterns = patterns.load(suite)
    folder = runs_folder(suite)
    todo = [q for q in queries.queries if (only is None or q.id in only) and not _done(folder, q)]
    setup = _setup(suite)
    spent = 0.0
    incomplete: list[str] = []
    for query in todo:
        searched = _grep(query, agent_patterns[query.id], setup)
        models = _models(query, setup)
        spent += models.record["jevpipe"]["cost"] + models.record["deepseek"]["cost"]
        print(_progress(models.record, spent))
        if models.complete:
            store.write_json(folder / f"{query.id}.json", {**models.record, "grep": searched})
        else:
            incomplete.append(query.id)
    _finish(len(todo), spent, incomplete)


def repeat_queries(only: Sequence[str] | None) -> None:
    queries = dataset.load(store.PYTHON)
    chosen = [queries.by_id(query_id) for query_id in (only or queries.repeat)]
    todo = [query for query in chosen if not _done(repeats_folder(), query)]
    setup = _setup(store.PYTHON)
    spent = 0.0
    incomplete: list[str] = []
    for query in todo:
        models = _models(query, setup)
        spent += models.record["jevpipe"]["cost"] + models.record["deepseek"]["cost"]
        print(_progress(models.record, spent))
        if models.complete:
            store.write_json(repeats_folder() / f"{query.id}.json", models.record)
        else:
            incomplete.append(query.id)
    _finish(len(todo), spent, incomplete)


def _done(folder: Path, query: dataset.Query) -> bool:
    return store.exists(folder / f"{query.id}.json")


def _setup(suite: store.Suite) -> _Setup:
    snippets = pool.load(suite).snippets
    codes = tuple(pool.code(suite, index) for index in range(len(snippets)))
    return _Setup(
        tuple(snippet.name for snippet in snippets),
        codes,
        wording.frozen_templates(),
        jev.resolved_version(),
        jev.version(),
        suite.pool,
        grep.version(),
    )


def _grep(query: dataset.Query, agent_pattern: str, setup: _Setup) -> GrepRecord:
    words = grep.keywords(query.text)
    found_any = grep.any_keyword(setup.folder, words)
    found_all = grep.all_keywords(setup.folder, words)
    found_agent = grep.pattern(setup.folder, agent_pattern)
    return {
        "version": setup.rg_version,
        "any": list(found_any.indexes),
        "all": list(found_all.indexes),
        "agent": list(found_agent.indexes),
        "seconds": {
            "any": found_any.seconds,
            "all": found_all.seconds,
            "agent": found_agent.seconds,
        },
    }


def jev_first(query_id: str) -> bool:
    return int(query_id.removeprefix("q")) % 2 == 0


def _models(query: dataset.Query, setup: _Setup) -> _Models:
    first = jev_first(query.id)
    if first:
        jev_record = _jev(query, setup)
        deepseek_record = _deepseek(query, setup)
    else:
        deepseek_record = _deepseek(query, setup)
        jev_record = _jev(query, setup)
    jev_record["first"] = first
    deepseek_record["first"] = not first
    allowed = MAX_FAILED_SHARE * len(setup.names)
    complete = jev_record["failed"] <= allowed and deepseek_record["failed"] <= allowed
    record: RepeatFile = {"query": query.id, "jevpipe": jev_record, "deepseek": deepseek_record}
    return _Models(record, complete)


def _jev(query: dataset.Query, setup: _Setup) -> JevRecord:
    started = _now()
    question = setup.templates["jevpipe"].format(query=query.text)
    result = jev.run(setup.names, question, setup.folder)
    return {
        "version": setup.jevpipe,
        "model": jev.MODEL,
        "resolved_model": setup.jev_version,
        "started": started,
        "first": False,
        "wall_seconds": round(result.wall_seconds, 2),
        "cost": result.cost,
        "failed": result.failed,
        "skipped": result.skipped,
        "probability": list(result.probability),
    }


def _deepseek(query: dataset.Query, setup: _Setup) -> DeepSeekRecord:
    question = setup.templates["deepseek"].format(query=query.text)
    items = [
        deepseek.Item(question, name, code)
        for name, code in zip(setup.names, setup.codes, strict=True)
    ]
    started = _now()
    result = deepseek.run(items)
    return {
        "model": deepseek.MODEL,
        "resolved_model": result.resolved_model,
        "started": started,
        "first": False,
        "wall_seconds": round(result.wall_seconds, 2),
        "cost": result.cost,
        "failed": result.failed,
        "providers": result.providers,
        "answer": list(result.answer),
        "probability": list(result.probability),
    }


def _now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def _progress(record: RepeatFile, spent: float) -> str:
    jev_record = record["jevpipe"]
    deepseek_record = record["deepseek"]
    return (
        f"{record['query']}: jevpipe {jev_record['wall_seconds']:.1f}s "
        f"${jev_record['cost']:.4f} {jev_record['failed']} failed, "
        f"deepseek {deepseek_record['wall_seconds']:.1f}s ${deepseek_record['cost']:.4f} "
        f"{deepseek_record['failed']} failed "
        f"({sum(p is None for p in deepseek_record['probability'])} without probability); "
        f"spent ${spent:.3f}"
    )


def _finish(ran: int, spent: float, incomplete: list[str]) -> None:
    print(f"{ran} queries run, ${spent:.3f} spent")
    if incomplete:
        message = f"too many failures, not stored: {', '.join(incomplete)}; run again"
        raise StageError(message)


def load_runs(suite: store.Suite) -> dict[str, RunFile]:
    return {
        path.stem: cast("RunFile", store.read_json(path))
        for path in sorted(runs_folder(suite).glob("q*.json"))
    }


def load_repeats() -> dict[str, RepeatFile]:
    return {
        path.stem: cast("RepeatFile", store.read_json(path))
        for path in sorted(repeats_folder().glob("q*.json"))
    }


def retry_failed(suite: store.Suite) -> None:
    folder = runs_folder(suite)
    names = [snippet.name for snippet in pool.load(suite).snippets]
    templates = wording.frozen_templates()
    queries = dataset.load(suite)
    retried = spent = 0
    for query_id, stored in load_runs(suite).items():
        record = stored["jevpipe"]
        missing = [index for index, p in enumerate(record["probability"]) if p is None]
        if not missing or "retried" in record:
            continue
        question = templates["jevpipe"].format(query=queries.by_id(query_id).text)
        again = jev.run([names[index] for index in missing], question, suite.pool)
        probability = list(record["probability"])
        for index, answer in zip(missing, again.probability, strict=True):
            probability[index] = answer
        answered = sum(answer is not None for answer in again.probability)
        record["retried"] = {
            "records": len(missing),
            "answered": answered,
            "version": jev.version(),
            "cost": again.cost,
            "wall_seconds": round(again.wall_seconds, 2),
        }
        record["probability"] = probability
        record["failed"] = sum(p is None for p in probability) - record["skipped"]
        record["cost"] += again.cost
        store.write_json(folder / f"{query_id}.json", stored)
        retried += 1
        spent += again.cost
        print(f"{query_id}: {answered} of {len(missing)} failed records answered on retry")
    print(f"retry: {retried} runs retried, ${spent:.4f} spent")
