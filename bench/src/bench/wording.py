from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import NotRequired, TypedDict, cast

from bench import counts, dataset, decisions, deepseek, jev, labels, limits, pool, store
from bench.contenders import MODELS, Model
from bench.limits import StageError

WORDINGS = (
    'Does this code do what a developer searching for "{query}" is looking for?',
    'Is this code a good result for the code search "{query}"?',
    'Would a developer searching for "{query}" want to use this function?',
    'Does this function implement "{query}"?',
)


class _Trial(TypedDict):
    wording: int
    f1: float
    cost: float
    decisions: NotRequired[dict[str, list[float | None]] | dict[str, list[bool | None]]]
    probability: NotRequired[dict[str, list[float | None]]]
    answer: NotRequired[dict[str, list[bool | None]]]


class _TrialsFile(TypedDict):
    model: str
    trials: list[_Trial]


class _Frozen(TypedDict):
    wording: int
    question: str


@dataclass(frozen=True)
class _Dev:
    queries: tuple[dataset.Query, ...]
    snippets: dict[str, tuple[int, ...]]
    relevant: dict[str, tuple[bool, ...]]


@dataclass(frozen=True)
class _Answers:
    probability: dict[str, list[float | None]]
    answer: dict[str, list[bool | None]]
    cost: float


def try_wordings() -> None:
    queries = dataset.load(store.PYTHON).of_split("dev")
    known = labels.Labels(labels.load_experts(store.PYTHON))
    dev = _dev(queries, known)
    for model in MODELS:
        stored = _trials(model)
        done = {trial["wording"] for trial in stored}
        for number in range(1, len(WORDINGS) + 1):
            if number in done:
                continue
            stored.append(_trial(model, number, dev))
            data: _TrialsFile = {"model": _model_id(model), "trials": stored}
            store.write_json(_trials_file(model), data)
    _freeze()


def _trials_file(model: Model) -> Path:
    return store.PYTHON.results / "wordings" / f"{model}.json"


def _frozen_file() -> Path:
    return store.PYTHON.results / "frozen.json"


def _trials(model: Model) -> list[_Trial]:
    path = _trials_file(model)
    return cast("_TrialsFile", store.read_json(path))["trials"] if path.is_file() else []


def _dev(queries: tuple[dataset.Query, ...], known: labels.Labels) -> _Dev:
    snippets: dict[str, tuple[int, ...]] = {}
    relevant: dict[str, tuple[bool, ...]] = {}
    for query in queries:
        rated = sorted(index for query_id, index in known.expert if query_id == query.id)
        snippets[query.id] = tuple(rated)
        relevant[query.id] = tuple(known.relevant((query.id, index)) is True for index in rated)
    return _Dev(queries, snippets, relevant)


def _trial(model: Model, wording: int, dev: _Dev) -> _Trial:
    template = WORDINGS[wording - 1]
    match model:
        case "jevpipe":
            answers = _jev_answers(template, dev)
        case "deepseek":
            answers = _deepseek_answers(template, dev)
    flags = (
        (decisions.says_yes(p, decisions.DEFAULT_THRESHOLD, answer=answer), relevant)
        for query in dev.queries
        for p, answer, relevant in zip(
            answers.probability[query.id],
            answers.answer[query.id],
            dev.relevant[query.id],
            strict=True,
        )
    )
    f1 = counts.count(flags).f1
    print(f"wordings: {model} wording {wording}: F1 {f1:.3f}, ${answers.cost:.4f}")
    return {
        "wording": wording,
        "f1": f1,
        "cost": answers.cost,
        "probability": answers.probability,
        "answer": answers.answer,
    }


def _jev_answers(template: str, dev: _Dev) -> _Answers:
    probability: dict[str, list[float | None]] = {}
    cost = 0.0
    for query in dev.queries:
        names = [store.PYTHON.name_of(index) for index in dev.snippets[query.id]]
        result = jev.run(names, template.format(query=query.text), store.PYTHON.pool)
        _check_failures(result.parsed.failed, len(names))
        probability[query.id] = list(result.parsed.probability)
        cost += result.cost
    answer: dict[str, list[bool | None]] = {q: [None] * len(p) for q, p in probability.items()}
    return _Answers(probability, answer, cost)


def _deepseek_answers(template: str, dev: _Dev) -> _Answers:
    items = [
        deepseek.Item(
            template.format(query=query.text),
            store.PYTHON.name_of(index),
            pool.code(store.PYTHON, index),
        )
        for query in dev.queries
        for index in dev.snippets[query.id]
    ]
    result = deepseek.run(items)
    _check_failures(result.failed, len(items))
    probability = _per_query(result.probability, dev)
    answer = _per_query(result.answer, dev)
    return _Answers(probability, answer, result.cost)


def _per_query[T](values: Sequence[T], dev: _Dev) -> dict[str, list[T]]:
    flat = iter(values)
    return {query.id: [next(flat) for _ in dev.snippets[query.id]] for query in dev.queries}


def _check_failures(failed: int, decided: int) -> None:
    if failed > limits.MAX_FAILED_SHARE * decided:
        message = f"{failed} of {decided} decisions failed; the trial is not stored, run again"
        raise StageError(message)


def _model_id(model: Model) -> str:
    match model:
        case "jevpipe":
            return jev.MODEL
        case "deepseek":
            return deepseek.MODEL


def _freeze() -> None:
    frozen: dict[str, _Frozen] = {}
    for model in MODELS:
        scores = {trial["wording"]: trial["f1"] for trial in _trials(model)}
        best = best_wording(scores)
        frozen[model] = {"wording": best, "question": WORDINGS[best - 1]}
        print(f"wordings: {model} uses wording {best} (dev F1 {scores[best]:.3f})")
    store.write_json(_frozen_file(), frozen)


def best_wording(scores: Mapping[int, float]) -> int:
    return max(scores, key=lambda number: (scores[number], -number))


def _frozen() -> dict[Model, _Frozen]:
    return cast("dict[Model, _Frozen]", store.read_json(_frozen_file()))


def frozen_wordings() -> dict[Model, int]:
    frozen = _frozen()
    return {model: frozen[model]["wording"] for model in MODELS}


def frozen_templates() -> dict[Model, str]:
    frozen = _frozen()
    return {model: frozen[model]["question"] for model in MODELS}


def trial_f1s(model: Model) -> tuple[float, ...]:
    return tuple(
        trial["f1"] for trial in sorted(_trials(model), key=lambda trial: trial["wording"])
    )
