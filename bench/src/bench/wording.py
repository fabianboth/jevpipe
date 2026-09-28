from dataclasses import dataclass
from typing import Literal, TypedDict, cast

from bench import counts, dataset, deepseek, jev, labels, pool, store

type Model = Literal["jevpipe", "deepseek"]

MODELS: tuple[Model, ...] = ("jevpipe", "deepseek")
WORDINGS = (
    'Does this code do what a developer searching for "{query}" is looking for?',
    'Is this code a good result for the code search "{query}"?',
    'Would a developer searching for "{query}" want to use this function?',
    'Does this function implement "{query}"?',
)
JEV_THRESHOLD = 0.5
_FOLDER = store.PYTHON.results / "wordings"
_FROZEN_FILE = store.PYTHON.results / "frozen.json"


class _Trial(TypedDict):
    wording: int
    f1: float
    cost: float
    decisions: dict[str, list[float | None]] | dict[str, list[bool | None]]


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


def question(wording: int, query: str) -> str:
    return WORDINGS[wording - 1].format(query=query)


def try_wordings() -> None:
    queries = dataset.load(store.PYTHON).of_split("dev")
    known = labels.Labels(labels.load_experts(store.PYTHON))
    dev = _dev(queries, known)
    for model in MODELS:
        path = _FOLDER / f"{model}.json"
        if store.exists(path):
            print(f"wordings: {model} already tried")
            continue
        trials = [_trial(model, wording, dev) for wording in range(1, len(WORDINGS) + 1)]
        data: _TrialsFile = {"model": _model_id(model), "trials": trials}
        store.write_json(path, data)
    _freeze()


def _dev(queries: tuple[dataset.Query, ...], known: labels.Labels) -> _Dev:
    snippets: dict[str, tuple[int, ...]] = {}
    relevant: dict[str, tuple[bool, ...]] = {}
    for query in queries:
        rated = sorted(index for query_id, index in known.expert if query_id == query.id)
        snippets[query.id] = tuple(rated)
        relevant[query.id] = tuple(known.relevant((query.id, index)) is True for index in rated)
    return _Dev(queries, snippets, relevant)


def _trial(model: Model, wording: int, dev: _Dev) -> _Trial:
    match model:
        case "jevpipe":
            trial = _jev_trial(wording, dev)
        case "deepseek":
            trial = _deepseek_trial(wording, dev)
    print(f"wordings: {model} wording {wording}: F1 {trial['f1']:.3f}, ${trial['cost']:.4f}")
    return trial


def _jev_trial(wording: int, dev: _Dev) -> _Trial:
    decisions: dict[str, list[float | None]] = {}
    cost = 0.0
    for query in dev.queries:
        names = [store.PYTHON.name_of(index) for index in dev.snippets[query.id]]
        result = jev.run(names, question(wording, query.text), store.PYTHON.pool)
        decisions[query.id] = list(result.probability)
        cost += result.cost
    flags = (
        (probability is not None and probability >= JEV_THRESHOLD, relevant)
        for query in dev.queries
        for probability, relevant in zip(decisions[query.id], dev.relevant[query.id], strict=True)
    )
    return {"wording": wording, "f1": counts.count(flags).f1, "cost": cost, "decisions": decisions}


def _deepseek_trial(wording: int, dev: _Dev) -> _Trial:
    items = [
        deepseek.Item(
            question(wording, query.text),
            store.PYTHON.name_of(index),
            pool.code(store.PYTHON, index),
        )
        for query in dev.queries
        for index in dev.snippets[query.id]
    ]
    result = deepseek.run(items)
    answers = iter(result.answer)
    decisions = {query.id: [next(answers) for _ in dev.snippets[query.id]] for query in dev.queries}
    flags = (
        (answer is True, relevant)
        for query in dev.queries
        for answer, relevant in zip(decisions[query.id], dev.relevant[query.id], strict=True)
    )
    return {
        "wording": wording,
        "f1": counts.count(flags).f1,
        "cost": result.cost,
        "decisions": decisions,
    }


def _model_id(model: Model) -> str:
    match model:
        case "jevpipe":
            return jev.MODEL
        case "deepseek":
            return deepseek.MODEL


def _freeze() -> None:
    frozen: dict[str, _Frozen] = {}
    for model in MODELS:
        data = cast("_TrialsFile", store.read_json(_FOLDER / f"{model}.json"))
        best = max(data["trials"], key=lambda trial: (trial["f1"], -trial["wording"]))
        frozen[model] = {"wording": best["wording"], "question": WORDINGS[best["wording"] - 1]}
        print(f"wordings: {model} uses wording {best['wording']} (dev F1 {best['f1']:.3f})")
    store.write_json(_FROZEN_FILE, frozen)


def frozen_templates() -> dict[Model, str]:
    data = cast("dict[Model, _Frozen]", store.read_json(_FROZEN_FILE))
    return {model: data[model]["question"] for model in MODELS}


def frozen_wordings() -> dict[Model, int]:
    data = cast("dict[Model, _Frozen]", store.read_json(_FROZEN_FILE))
    return {model: data[model]["wording"] for model in MODELS}


def trial_f1s(model: Model) -> tuple[float, ...]:
    data = cast("_TrialsFile", store.read_json(_FOLDER / f"{model}.json"))
    return tuple(
        trial["f1"] for trial in sorted(data["trials"], key=lambda trial: trial["wording"])
    )
