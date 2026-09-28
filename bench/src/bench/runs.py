from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from bench import dataset, deepseek, grep, jev, limits, patterns, pool, store, wording
from bench.contenders import Model
from bench.limits import StageError
from bench.records import (
    DeepSeekRecord,
    GrepRecord,
    JevRecord,
    RepeatFile,
    Retried,
    RunFile,
)


@dataclass(frozen=True)
class _Setup:
    names: tuple[str, ...]
    codes: tuple[str, ...]
    templates: dict[Model, str]
    resolved_model: str
    jevpipe_version: str
    rg_version: str
    folder: Path


@dataclass(frozen=True)
class _Models:
    record: RepeatFile
    complete: bool


@dataclass(frozen=True)
class _Plan:
    suite: store.Suite
    queries: tuple[dataset.Query, ...]
    folder: Path


@dataclass(frozen=True)
class _Asked:
    suite: store.Suite
    names: list[str]
    question: str


type _Stored = Callable[[dataset.Query, _Setup, RepeatFile], object]


def run_queries(suite: store.Suite, only: tuple[str, ...] | None) -> None:
    agent_patterns = patterns.load(suite)

    def with_grep(query: dataset.Query, setup: _Setup, record: RepeatFile) -> object:
        return {**record, "grep": _grep(query, agent_patterns[query.id], setup)}

    plan = _Plan(suite, dataset.load(suite).chosen(only), suite.runs)
    _run(plan, with_grep)


def repeat_queries(only: tuple[str, ...] | None) -> None:
    queries = dataset.load(store.PYTHON)
    chosen = queries.chosen(only or queries.repeat)
    _run(_Plan(store.PYTHON, chosen, store.PYTHON.repeats), lambda _q, _s, record: record)


def _run(plan: _Plan, stored: _Stored) -> None:
    todo = [query for query in plan.queries if not (plan.folder / f"{query.id}.json").is_file()]
    if not todo:
        print("every query is already run")
        return
    setup = _setup(plan.suite)
    spent = 0.0
    incomplete: list[str] = []
    for query in todo:
        models = _models(query, setup)
        spent += models.record["jevpipe"]["cost"] + models.record["deepseek"]["cost"]
        print(_progress(models.record, spent))
        if models.complete:
            store.write_json(plan.folder / f"{query.id}.json", stored(query, setup, models.record))
        else:
            incomplete.append(query.id)
    print(f"{len(todo)} queries run, ${spent:.3f} spent")
    if incomplete:
        message = f"too many failures, not stored: {', '.join(incomplete)}; run again"
        raise StageError(message)


def _setup(suite: store.Suite) -> _Setup:
    snippets = pool.load(suite).snippets
    return _Setup(
        tuple(snippet.name for snippet in snippets),
        tuple(pool.code(suite, index) for index in range(len(snippets))),
        wording.frozen_templates(),
        jev.resolved_version(),
        jev.version(),
        grep.version(),
        suite.pool,
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


def _jev_first(query_id: str) -> bool:
    return int(query_id.removeprefix("q")) % 2 == 0


def _models(query: dataset.Query, setup: _Setup) -> _Models:
    first = _jev_first(query.id)
    if first:
        jev_record = _jev(query, setup)
        deepseek_record = _deepseek(query, setup)
    else:
        deepseek_record = _deepseek(query, setup)
        jev_record = _jev(query, setup)
    jev_record["first"] = first
    deepseek_record["first"] = not first
    allowed = limits.MAX_FAILED_SHARE * len(setup.names)
    complete = jev_record["failed"] <= allowed and deepseek_record["failed"] <= allowed
    record: RepeatFile = {"query": query.id, "jevpipe": jev_record, "deepseek": deepseek_record}
    return _Models(record, complete)


def _jev(query: dataset.Query, setup: _Setup) -> JevRecord:
    started = store.now()
    result = jev.run(setup.names, setup.templates["jevpipe"].format(query=query.text), setup.folder)
    return {
        "version": setup.jevpipe_version,
        "model": jev.MODEL,
        "resolved_model": setup.resolved_model,
        "started": started,
        "first": False,
        "wall_seconds": round(result.wall_seconds, 2),
        "cost": result.cost,
        "failed": result.parsed.failed,
        "skipped": result.parsed.skipped,
        "probability": list(result.parsed.probability),
    }


def _deepseek(query: dataset.Query, setup: _Setup) -> DeepSeekRecord:
    question = setup.templates["deepseek"].format(query=query.text)
    items = [
        deepseek.Item(question, name, code)
        for name, code in zip(setup.names, setup.codes, strict=True)
    ]
    started = store.now()
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


def load_runs(suite: store.Suite) -> dict[str, RunFile]:
    return cast("dict[str, RunFile]", _load(suite.runs))


def load_repeats() -> dict[str, RepeatFile]:
    return _load(store.PYTHON.repeats)


def _load(folder: Path) -> dict[str, RepeatFile]:
    return {
        path.stem: cast("RepeatFile", store.read_json(path))
        for path in sorted(folder.glob("q*.json"))
    }


def retry_failed(suite: store.Suite, only: tuple[str, ...] | None) -> None:
    queries = dataset.load(suite)
    wanted = {query.id for query in queries.chosen(only)}
    names = [snippet.name for snippet in pool.load(suite).snippets]
    templates = wording.frozen_templates()
    spent = 0.0
    for query_id, stored in load_runs(suite).items():
        if query_id not in wanted:
            continue
        text = queries.text_of(query_id)
        retried = [
            _retry_jev(
                stored["jevpipe"], _Asked(suite, names, templates["jevpipe"].format(query=text))
            ),
            _retry_deepseek(
                stored["deepseek"], _Asked(suite, names, templates["deepseek"].format(query=text))
            ),
        ]
        done = [again for again in retried if again is not None]
        if done:
            store.write_json(suite.runs / f"{query_id}.json", stored)
            spent += sum(again["cost"] for again in done)
            print(f"{query_id}: " + "; ".join(_retry_line(again) for again in done))
    print(f"retry: ${spent:.4f} spent")


def _retry_line(again: Retried) -> str:
    return f"{again['answered']} of {again['records']} failed records answered on retry"


def _retry_jev(record: JevRecord, asked: _Asked) -> Retried | None:
    missing = [index for index, p in enumerate(record["probability"]) if p is None]
    if len(missing) <= record["skipped"] or "retried" in record:
        return None
    again = jev.run([asked.names[index] for index in missing], asked.question, asked.suite.pool)
    probability = list(record["probability"])
    for index, answer in zip(missing, again.parsed.probability, strict=True):
        probability[index] = answer
    retried: Retried = {
        "records": len(missing),
        "answered": sum(answer is not None for answer in again.parsed.probability),
        "version": jev.version(),
        "cost": again.cost,
        "wall_seconds": round(again.wall_seconds, 2),
    }
    record["retried"] = retried
    record["probability"] = probability
    record["failed"] = jev.failed_of(probability, record["skipped"])
    record["cost"] += again.cost
    return retried


def _retry_deepseek(record: DeepSeekRecord, asked: _Asked) -> Retried | None:
    pairs = zip(record["answer"], record["probability"], strict=True)
    missing = [index for index, (a, p) in enumerate(pairs) if a is None and p is None]
    if not missing or "retried" in record:
        return None
    items = [
        deepseek.Item(asked.question, asked.names[i], pool.code(asked.suite, i)) for i in missing
    ]
    again = deepseek.run(items)
    for position, index in enumerate(missing):
        record["answer"][index] = again.answer[position]
        record["probability"][index] = again.probability[position]
    retried: Retried = {
        "records": len(missing),
        "answered": len(missing) - again.failed,
        "cost": again.cost,
        "wall_seconds": round(again.wall_seconds, 2),
    }
    record["retried"] = retried
    record["failed"] = again.failed
    record["cost"] += again.cost
    return retried
