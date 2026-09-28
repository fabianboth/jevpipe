import itertools
import statistics
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from typing import Literal

from bench import dataset, judge, labels, runs, store
from bench.counts import Counts

type Contender = Literal["grep-any", "grep-all", "grep-agent", "jevpipe", "deepseek"]
type Model = Literal["jevpipe", "deepseek"]
type Decisions = runs.RepeatFile | runs.RunFile

CONTENDERS: tuple[Contender, ...] = ("grep-any", "grep-all", "grep-agent", "jevpipe", "deepseek")
MODELS: tuple[Model, ...] = ("jevpipe", "deepseek")
THRESHOLDS = tuple(round(0.30 + 0.05 * step, 2) for step in range(13))
_BAND_EDGES = (0.5, 0.6, 0.7, 0.8, 0.9, 1.0)
_PERCENTILE = 90
_DECISION = 0.5
_EPSILON = 1e-9


@dataclass(frozen=True)
class Data:
    queries: dataset.Queries
    known: labels.Labels
    runs: Mapping[str, runs.RunFile]
    repeats: Mapping[str, runs.RepeatFile]
    selection: Sequence[judge.Item]
    pool_size: int
    suite: store.Suite

    def ids(self, split: dataset.Split) -> tuple[str, ...]:
        return tuple(query.id for query in self.queries.of_split(split) if query.id in self.runs)

    def relevant(self, query_id: str) -> frozenset[int]:
        labelled = set(self.known.expert) | set(self.known.judge)
        return frozenset(
            index
            for query, index in labelled
            if query == query_id and self.known.relevant((query, index))
        )

    def sampled(self, reason: judge.Reason) -> dict[str, list[int]]:
        chosen: dict[str, list[int]] = {}
        for item in self.selection:
            if reason in item.reasons:
                chosen.setdefault(item.query, []).append(item.snippet)
        return chosen


@dataclass(frozen=True)
class Result:
    counts: Counts
    unjudged: int
    seconds_median: float
    seconds_p90: float
    cost_per_1000: float


@dataclass(frozen=True)
class Band:
    low: float
    high: float
    decisions: int
    accuracy: float


@dataclass(frozen=True)
class Missed:
    sampled: int
    relevant_in_sample: int
    unflagged: int
    estimate: float


@dataclass(frozen=True)
class Tally:
    decisions: int
    failed: int
    skipped: int
    without_probability: int


def flagged(run: Decisions, contender: Contender, threshold: float) -> frozenset[int]:
    match contender:
        case "jevpipe":
            probability = run["jevpipe"]["probability"]
            return frozenset(
                i for i, p in enumerate(probability) if p is not None and p >= threshold
            )
        case "deepseek":
            deepseek = run["deepseek"]
            answers = zip(deepseek["answer"], deepseek["probability"], strict=True)
            return frozenset(
                i
                for i, (answer, p) in enumerate(answers)
                if (p >= threshold if p is not None else answer is True)
            )
        case "grep-any":
            return frozenset(_grep(run)["any"])
        case "grep-all":
            return frozenset(_grep(run)["all"])
        case "grep-agent":
            return frozenset(_grep(run)["agent"])


def _grep(run: Decisions) -> runs.GrepRecord:
    if "grep" not in run:
        message = f"the run of {run['query']} holds no grep results"
        raise KeyError(message)
    return run["grep"]


def counted(
    data: Data, query_ids: Iterable[str], flags: Callable[[str], frozenset[int]]
) -> tuple[Counts, int]:
    found = false_hits = known_relevant = unjudged = 0
    for query_id in query_ids:
        known_relevant += len(data.relevant(query_id))
        for index in flags(query_id):
            match data.known.relevant((query_id, index)):
                case True:
                    found += 1
                case False:
                    false_hits += 1
                case None:
                    unjudged += 1
    return Counts(found, false_hits, known_relevant), unjudged


def model_counts(data: Data, model: Model, threshold: float) -> Counts:
    return _model_counts(data, (model, threshold), data.ids("test"))


def _model_counts(data: Data, setting: tuple[Model, float], query_ids: Iterable[str]) -> Counts:
    model, threshold = setting
    return counted(data, query_ids, lambda q: flagged(data.runs[q], model, threshold))[0]


def choose_threshold(data: Data, model: Model) -> float:
    dev = data.ids("dev")
    return max(THRESHOLDS, key=lambda t: (_model_counts(data, (model, t), dev).f1, -t))


def result(data: Data, contender: Contender, threshold: float) -> Result:
    test = data.ids("test")
    if contender == "grep-any":
        counts, unjudged = _grep_any_counts(data, test), 0
    else:
        counts, unjudged = counted(
            data, test, lambda q: flagged(data.runs[q], contender, threshold)
        )
    seconds = [_seconds(data.runs[q], contender) for q in test]
    cost = sum(_cost(data.runs[q], contender) for q in test)
    return Result(
        counts,
        unjudged,
        statistics.median(seconds),
        _percentile(seconds),
        cost / (data.pool_size * len(test)) * 1000,
    )


def _percentile(values: Sequence[float]) -> float:
    if len(values) == 1:
        return values[0]
    return statistics.quantiles(values, n=100, method="inclusive")[_PERCENTILE - 1]


def _grep_any_counts(data: Data, test: Sequence[str]) -> Counts:
    sample = data.sampled("grep-any-sample")
    pooled = _share(data, [(q, i) for q in test for i in sample.get(q, [])])
    found = false_hits = known_relevant = 0.0
    for query_id in test:
        relevant = data.relevant(query_id)
        hits = flagged(data.runs[query_id], "grep-any", 0.0)
        rated = {i for i in hits if data.known.rated_by_expert((query_id, i))}
        unrated = hits - rated
        share = _share(data, [(query_id, i) for i in sample.get(query_id, [])])
        estimated = len(unrated) * (share if share is not None else pooled or 0.0)
        found += len(rated & relevant) + estimated
        false_hits += len(rated - relevant) + len(unrated) - estimated
        known_relevant += len(relevant) + max(0.0, estimated - len(unrated & relevant))
    return Counts(found, false_hits, known_relevant)


def _share(data: Data, pairs: Sequence[labels.Pair]) -> float | None:
    judged = [data.known.relevant(pair) for pair in pairs]
    known = [relevant for relevant in judged if relevant is not None]
    return sum(known) / len(known) if known else None


def _seconds(run: runs.RunFile, contender: Contender) -> float:
    match contender:
        case "jevpipe" | "deepseek":
            return run[contender]["wall_seconds"]
        case "grep-any":
            return run["grep"]["seconds"]["any"]
        case "grep-all":
            return run["grep"]["seconds"]["all"]
        case "grep-agent":
            return run["grep"]["seconds"]["agent"]


def _cost(run: runs.RunFile, contender: Contender) -> float:
    match contender:
        case "jevpipe" | "deepseek":
            return run[contender]["cost"]
        case "grep-any" | "grep-all" | "grep-agent":
            return 0.0


def sweep(data: Data, model: Model) -> list[tuple[float, Counts]]:
    return [(threshold, model_counts(data, model, threshold)) for threshold in THRESHOLDS]


def bands(data: Data, model: Model) -> list[Band]:
    decided: list[tuple[float, bool]] = []
    for query_id in data.ids("test"):
        probability = data.runs[query_id][model]["probability"]
        for index, p in enumerate(probability):
            relevant = data.known.relevant((query_id, index))
            if p is not None and relevant is not None:
                decided.append((p, relevant))
    edges = list(itertools.pairwise(_BAND_EDGES))
    right: list[list[bool]] = [[] for _ in edges]
    for p, relevant in decided:
        confidence = max(p, 1 - p)
        band = min(int((confidence - _BAND_EDGES[0]) * 10 + _EPSILON), len(edges) - 1)
        right[band].append((p >= _DECISION) == relevant)
    return [
        Band(low, high, len(inside), sum(inside) / len(inside) if inside else 0.0)
        for (low, high), inside in zip(edges, right, strict=True)
    ]


def missed(data: Data) -> Missed:
    sample = data.sampled("unflagged-sample")
    test = data.ids("test")
    pairs = [(q, i) for q in test for i in sample.get(q, [])]
    judged = [data.known.relevant(pair) for pair in pairs]
    known = [relevant for relevant in judged if relevant is not None]
    unflagged = 0
    for query_id in test:
        hits = judge.flags_of(data.runs[query_id]).any_flag()
        unflagged += sum(
            1
            for index in range(data.pool_size)
            if index not in hits and not data.known.rated_by_expert((query_id, index))
        )
    share = sum(known) / len(known) if known else 0.0
    return Missed(len(known), sum(known), unflagged, unflagged * share)


def repeat_changes(data: Data, thresholds: Mapping[Model, float]) -> dict[Model, float]:
    changes: dict[Model, float] = {}
    for model in MODELS:
        changed = total = 0
        for query_id, again in data.repeats.items():
            first = flagged(data.runs[query_id], model, thresholds[model])
            second = flagged(again, model, thresholds[model])
            changed += len(first ^ second)
            total += data.pool_size
        changes[model] = changed / total if total else 0.0
    return changes


def tally(data: Data, model: Model) -> Tally:
    records = [run[model] for run in data.runs.values()]
    return Tally(
        sum(len(record["probability"]) for record in records),
        sum(record["failed"] for record in records),
        sum(run["jevpipe"]["skipped"] for run in data.runs.values()) if model == "jevpipe" else 0,
        sum(p is None for record in records for p in record["probability"]),
    )


@dataclass(frozen=True)
class Scored:
    data: Data
    thresholds: dict[Model, float]
    results: dict[Contender, Result]


def chosen_thresholds(data: Data) -> dict[Model, float]:
    return {model: choose_threshold(data, model) for model in MODELS}


def scored(data: Data, thresholds: dict[Model, float]) -> Scored:
    results: dict[Contender, Result] = {
        contender: result(data, contender, threshold_for(thresholds, contender))
        for contender in CONTENDERS
    }
    return Scored(data, thresholds, results)


def threshold_for(thresholds: dict[Model, float], contender: Contender) -> float:
    match contender:
        case "jevpipe" | "deepseek":
            return thresholds[contender]
        case "grep-any" | "grep-all" | "grep-agent":
            return 0.0


def dates(data: Data) -> dict[str, str]:
    started = sorted(run[model]["started"] for run in data.runs.values() for model in MODELS)
    return {"first": started[0][:10], "last": started[-1][:10]}


def requested(data: Data, model: Model) -> str:
    return ", ".join(sorted({run[model]["model"] for run in data.runs.values()}))


def resolved(data: Data, model: Model) -> list[str]:
    return sorted({run[model]["resolved_model"] for run in data.runs.values()})


def per_query(data: Data, contender: Contender, threshold: float) -> list[tuple[int, int, int]]:
    counts: list[tuple[int, int, int]] = []
    for query_id in data.ids("test"):
        relevant = data.relevant(query_id)
        hits = flagged(data.runs[query_id], contender, threshold)
        false_hits = sum(1 for index in hits if data.known.relevant((query_id, index)) is False)
        counts.append((len(hits & relevant), false_hits, len(relevant)))
    return counts
