import itertools
import statistics
from collections import defaultdict
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from functools import cached_property

from bench import agreement, dataset, labels, selection, store
from bench.contenders import CONTENDERS, MODELS, Contender, Model
from bench.counts import Counts
from bench.decisions import THRESHOLDS, Setting
from bench.records import RepeatFile, RunFile

_BAND_EDGES = (0.5, 0.6, 0.7, 0.8, 0.9, 1.0)
_PERCENTILE = 90
_DECISION = 0.5
_EPSILON = 1e-9


@dataclass(frozen=True)
class Data:
    queries: dataset.Queries
    known: labels.Labels
    runs: Mapping[str, RunFile]
    repeats: Mapping[str, RepeatFile]
    selection: Sequence[selection.Item]
    pool_size: int
    suite: store.Suite

    def ids(self, split: dataset.Split) -> tuple[str, ...]:
        return tuple(query.id for query in self.queries.of_split(split) if query.id in self.runs)

    def relevant(self, query_id: str) -> frozenset[int]:
        return self._relevant.get(query_id, frozenset())

    @cached_property
    def _relevant(self) -> dict[str, frozenset[int]]:
        found: defaultdict[str, set[int]] = defaultdict(set)
        for pair in {*self.known.expert, *self.known.judge}:
            if self.known.relevant(pair):
                found[pair[0]].add(pair[1])
        return {query_id: frozenset(indexes) for query_id, indexes in found.items()}

    def sampled(self, reason: selection.Reason) -> dict[str, list[int]]:
        chosen: dict[str, list[int]] = {}
        for item in self.selection:
            if reason in item.reasons:
                chosen.setdefault(item.query, []).append(item.snippet)
        return chosen

    def seconds(self, query_id: str, contender: Contender) -> float:
        run = self.runs[query_id]
        match contender:
            case "jevpipe" | "deepseek":
                record = run[contender]
                retried = record.get("retried")
                return record["wall_seconds"] + (retried["wall_seconds"] if retried else 0.0)
            case "grep-any":
                return run["grep"]["seconds"]["any"]
            case "grep-all":
                return run["grep"]["seconds"]["all"]
            case "grep-agent":
                return run["grep"]["seconds"]["agent"]

    def cost(self, query_id: str, contender: Contender) -> float:
        match contender:
            case "jevpipe" | "deepseek":
                return self.runs[query_id][contender]["cost"]
            case "grep-any" | "grep-all" | "grep-agent":
                return 0.0


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
    right: int

    @property
    def accuracy(self) -> float | None:
        return self.right / self.decisions if self.decisions else None

    def __add__(self, other: Band) -> Band:
        return Band(self.low, self.high, self.decisions + other.decisions, self.right + other.right)


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


@dataclass(frozen=True)
class Scored:
    data: Data
    thresholds: dict[Model, float]
    results: dict[Contender, Result]
    agreement: agreement.Agreement

    def setting(self, contender: Contender) -> Setting:
        match contender:
            case "jevpipe" | "deepseek":
                return Setting(contender, self.thresholds[contender])
            case "grep-any" | "grep-all" | "grep-agent":
                return Setting(contender, 0.0)


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


def model_counts(data: Data, setting: Setting) -> Counts:
    return _counts_on(data, setting, data.ids("test"))


def _counts_on(data: Data, setting: Setting, query_ids: Iterable[str]) -> Counts:
    return counted(data, query_ids, lambda q: setting.flags(data.runs[q]))[0]


def choose_threshold(data: Data, model: Model) -> float:
    dev = data.ids("dev")
    return max(THRESHOLDS, key=lambda t: (_counts_on(data, Setting(model, t), dev).f1, -t))


def result(data: Data, setting: Setting) -> Result:
    test = data.ids("test")
    match setting.contender:
        case "grep-any":
            counts, unjudged = _grep_any_counts(data, test), 0
        case "grep-all" | "grep-agent" | "jevpipe" | "deepseek":
            counts, unjudged = counted(data, test, lambda q: setting.flags(data.runs[q]))
    seconds = [data.seconds(q, setting.contender) for q in test]
    cost = sum(data.cost(q, setting.contender) for q in test)
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
        hits = Setting("grep-any", 0.0).flags(data.runs[query_id])
        rated = {i for i in hits if data.known.rated_by_expert((query_id, i))}
        unrated = hits - rated
        share = _share(data, [(query_id, i) for i in sample.get(query_id, [])])
        estimated = len(unrated) * (share if share is not None else pooled or 0.0)
        relevant_unrated = max(estimated, len(unrated & relevant))
        found += len(rated & relevant) + relevant_unrated
        false_hits += len(rated - relevant) + len(unrated) - relevant_unrated
        known_relevant += len(relevant - unrated) + relevant_unrated
    return Counts(found, false_hits, known_relevant)


def _share(data: Data, pairs: Sequence[labels.Pair]) -> float | None:
    judged = [data.known.relevant(pair) for pair in pairs]
    known = [relevant for relevant in judged if relevant is not None]
    return sum(known) / len(known) if known else None


def sweep(data: Data, model: Model) -> list[tuple[float, Counts]]:
    return [(threshold, model_counts(data, Setting(model, threshold))) for threshold in THRESHOLDS]


def bands(data: Data, model: Model) -> list[Band]:
    edges = list(itertools.pairwise(_BAND_EDGES))
    decided = [0] * len(edges)
    right = [0] * len(edges)
    for query_id in data.ids("test"):
        for index, p in enumerate(data.runs[query_id][model]["probability"]):
            relevant = data.known.relevant((query_id, index))
            if p is None or relevant is None:
                continue
            band = min(int((max(p, 1 - p) - _BAND_EDGES[0]) * 10 + _EPSILON), len(edges) - 1)
            decided[band] += 1
            right[band] += (p >= _DECISION) == relevant
    return [
        Band(low, high, count, hits)
        for (low, high), count, hits in zip(edges, decided, right, strict=True)
    ]


def missed(data: Data) -> Missed:
    sample = data.sampled("unflagged-sample")
    test = data.ids("test")
    pairs = [(q, i) for q in test for i in sample.get(q, [])]
    judged = [data.known.relevant(pair) for pair in pairs]
    known = [relevant for relevant in judged if relevant is not None]
    unflagged = 0
    for query_id in test:
        hits = selection.flags_of(data.runs[query_id]).any_flag()
        unflagged += sum(
            1
            for index in range(data.pool_size)
            if index not in hits and not data.known.rated_by_expert((query_id, index))
        )
    share = sum(known) / len(known) if known else 0.0
    return Missed(len(known), sum(known), unflagged, unflagged * share)


def repeat_changes(scored: Scored) -> dict[Model, float] | None:
    data = scored.data
    if not data.repeats:
        return None
    changes: dict[Model, float] = {}
    for model in MODELS:
        setting = scored.setting(model)
        changed = sum(
            len(setting.flags(data.runs[query_id]) ^ setting.flags(again))
            for query_id, again in data.repeats.items()
        )
        changes[model] = changed / (data.pool_size * len(data.repeats))
    return changes


def tally(data: Data, model: Model) -> Tally:
    records = [run[model] for run in data.runs.values()]
    return Tally(
        sum(len(record["probability"]) for record in records),
        sum(record["failed"] for record in records),
        sum(run["jevpipe"]["skipped"] for run in data.runs.values()) if model == "jevpipe" else 0,
        sum(p is None for record in records for p in record["probability"]),
    )


def chosen_thresholds(data: Data) -> dict[Model, float]:
    return {model: choose_threshold(data, model) for model in MODELS}


def scored(data: Data, thresholds: dict[Model, float]) -> Scored:
    found = agreement.agreement(data.known.expert, data.known.judge)
    unscored = Scored(data, thresholds, {}, found)
    results: dict[Contender, Result] = {c: result(data, unscored.setting(c)) for c in CONTENDERS}
    return Scored(data, thresholds, results, found)


def dates(data: Data) -> dict[str, str]:
    started = sorted(run[model]["started"] for run in data.runs.values() for model in MODELS)
    return {"first": started[0][:10], "last": started[-1][:10]}


def requested(data: Data, model: Model) -> str:
    return ", ".join(sorted({run[model]["model"] for run in data.runs.values()}))


def resolved(data: Data, model: Model) -> list[str]:
    return sorted({run[model]["resolved_model"] for run in data.runs.values()})


def per_query(scored: Scored, contender: Contender) -> list[Counts]:
    data = scored.data
    setting = scored.setting(contender)
    return [counted(data, [q], lambda q: setting.flags(data.runs[q]))[0] for q in data.ids("test")]
