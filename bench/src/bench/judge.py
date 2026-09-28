import random
import time
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Literal, TypedDict, cast

from bench import codex, counts, dataset, labels, pool, runs, store
from bench.limits import LimitReachedError, StageError

type Reason = Literal["validation", "hit", "grep-any-sample", "unflagged-sample"]

HIT_THRESHOLD = 0.3
GREP_ANY_SAMPLE = 20
UNFLAGGED_SAMPLE = 10
BATCH = 40
GATE_F1 = 0.67
MIN_RATED_TWICE = 30
SHOWN_CHARACTERS = 20_000
_SELECTION_FILE = "selection.json"
_INSTRUCTIONS = """You are an expert annotator for the CodeSearchNet Challenge. A developer typed a
search query into a code search engine. For each item below, rate how relevant the code is to the
query, on the CodeSearchNet scale:
3 = Exact match: this seems exactly what the developer was looking for; they would copy-paste the
    code and make minor adaptations, or use this functionality of the library in their code.
2 = Strong match: this does more or less what they were looking for; they would use the code as a
    backbone for their purpose, but not necessarily copy-paste it or use this library.
1 = Weak match: not exactly what they were looking for, but there are useful elements or pointers
    (for example APIs or code structure) that could form the basis of a new query or exploration.
0 = Totally irrelevant: they would never want to see this for this query.
Judge each item on its own, from the query and the code shown. Do not run commands or read files;
everything you need is below. Return one judgment per item id."""
_SCHEMA: dict[str, object] = {
    "type": "object",
    "additionalProperties": False,
    "required": ["judgments"],
    "properties": {
        "judgments": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["id", "relevance"],
                "properties": {
                    "id": {"type": "string"},
                    "relevance": {"type": "integer", "enum": [0, 1, 2, 3]},
                },
            },
        }
    },
}


@dataclass(frozen=True)
class Flags:
    jevpipe: frozenset[int]
    deepseek: frozenset[int]
    grep_any: frozenset[int]
    grep_all: frozenset[int]
    grep_agent: frozenset[int]

    def hits(self) -> frozenset[int]:
        return self.jevpipe | self.deepseek | self.grep_all | self.grep_agent

    def any_flag(self) -> frozenset[int]:
        return self.hits() | self.grep_any


@dataclass(frozen=True)
class Item:
    id: str
    query: str
    snippet: int
    reasons: tuple[Reason, ...]


@dataclass(frozen=True)
class Agreement:
    judge: counts.Counts
    experts: counts.Counts
    judged: int
    twice: int

    @property
    def passed(self) -> bool:
        return self.judge.f1 >= GATE_F1


class _ItemRecord(TypedDict):
    id: str
    query: str
    snippet: str
    reasons: list[Reason]


class _SelectionFile(TypedDict):
    seed: int
    complete: bool
    items: list[_ItemRecord]


class Judgment(TypedDict):
    id: str
    relevance: int


class _Answer(TypedDict):
    judgments: list[Judgment]


class _BatchFile(TypedDict):
    batch: int
    model: str
    judged: str
    seconds: float
    ratings: dict[str, int]


def flags_of(run: runs.RunFile) -> Flags:
    jev = run["jevpipe"]["probability"]
    deepseek = run["deepseek"]
    return Flags(
        frozenset(i for i, p in enumerate(jev) if p is not None and p >= HIT_THRESHOLD),
        frozenset(
            i
            for i, (answer, p) in enumerate(
                zip(deepseek["answer"], deepseek["probability"], strict=True)
            )
            if answer is True or (p is not None and p >= HIT_THRESHOLD)
        ),
        frozenset(run["grep"]["any"]),
        frozenset(run["grep"]["all"]),
        frozenset(run["grep"]["agent"]),
    )


def select(rated: Iterable[labels.Pair], flags: Mapping[str, Flags], pool_size: int) -> list[Item]:
    validation = validation_items(rated)
    rated_pairs = {(item.query, item.snippet) for item in validation}
    reasons: dict[labels.Pair, set[Reason]] = {}
    unrated = _Unrated(rated_pairs, pool_size)
    for query_id, flagged in sorted(flags.items()):
        for reason, chosen in unrated.reasons(query_id, flagged):
            for index in chosen:
                reasons.setdefault((query_id, index), set()).add(reason)
    others = sorted(reasons)
    random.Random(dataset.SEED).shuffle(others)
    return validation + [
        Item(f"j{number:05d}", query_id, index, tuple(sorted(reasons[query_id, index])))
        for number, (query_id, index) in enumerate(others, start=len(validation) + 1)
    ]


def validation_items(rated: Iterable[labels.Pair]) -> list[Item]:
    pairs = sorted(set(rated))
    random.Random(dataset.SEED).shuffle(pairs)
    return [
        Item(f"j{number:05d}", query_id, index, ("validation",))
        for number, (query_id, index) in enumerate(pairs, start=1)
    ]


@dataclass(frozen=True)
class _Unrated:
    rated: set[labels.Pair]
    pool_size: int

    def reasons(self, query_id: str, flagged: Flags) -> list[tuple[Reason, list[int]]]:
        any_hits = self._of(query_id, flagged.grep_any)
        unflagged = self._of(query_id, set(range(self.pool_size)) - flagged.any_flag())
        return [
            ("hit", self._of(query_id, flagged.hits())),
            ("grep-any-sample", _sample(any_hits, GREP_ANY_SAMPLE, f"{query_id}:grep-any")),
            ("unflagged-sample", _sample(unflagged, UNFLAGGED_SAMPLE, f"{query_id}:unflagged")),
        ]

    def _of(self, query_id: str, indexes: Iterable[int]) -> list[int]:
        return sorted(index for index in indexes if (query_id, index) not in self.rated)


def _sample(population: list[int], size: int, name: str) -> list[int]:
    rng = random.Random(f"{dataset.SEED}:{name}")
    return sorted(rng.sample(population, min(size, len(population))))


def batches(items: Sequence[Item]) -> list[list[Item]]:
    validation = [item for item in items if _is_validation(item)]
    others = [item for item in items if not _is_validation(item)]
    return _chunks(validation) + _chunks(others)


def _chunks(items: Sequence[Item]) -> list[list[Item]]:
    return [list(items[start : start + BATCH]) for start in range(0, len(items), BATCH)]


def _is_validation(item: Item) -> bool:
    return "validation" in item.reasons


def agreement(
    expert: Mapping[labels.Pair, tuple[int, ...]], judged: Mapping[labels.Pair, int]
) -> Agreement:
    judge_counts = counts.count(
        (dataset.is_relevant(rating), dataset.is_relevant(dataset.mean(expert[pair])))
        for pair, rating in judged.items()
        if pair in expert
    )
    twice = [ratings for ratings in expert.values() if len(ratings) > 1]
    expert_counts = counts.count(
        (dataset.is_relevant(ratings[0]), dataset.is_relevant(dataset.mean(ratings[1:])))
        for ratings in twice
    )
    return Agreement(
        judge_counts, expert_counts, sum(pair in expert for pair in judged), len(twice)
    )


def folder(suite: store.Suite) -> Path:
    return suite.results / "judge"


def judge_pairs(suite: store.Suite, parallel: int, *, continue_judging: bool) -> None:
    items, complete = _selection(suite)
    all_batches = batches(items)
    texts = {query.id: query.text for query in dataset.load(suite).queries}
    judging = _Judging(suite, all_batches, texts, parallel)
    validation = [n for n, batch in enumerate(all_batches) if _is_validation(batch[0])]
    judging.judge(validation)
    found = agreement(labels.load_experts(suite), judged_ratings(suite))
    print(f"{suite.label} {_gate_line(found)}")
    if found.judge.f1 < GATE_F1 and not continue_judging:
        message = (
            f"the judge's F1 against the experts is {found.judge.f1:.3f}, below {GATE_F1}; "
            "rerun with --continue-judging to judge the rest anyway"
        )
        raise LimitReachedError(message)
    if not complete:
        message = "validation is judged; run every query, then rerun judge for the rest"
        raise StageError(message)
    rest = [n for n in range(len(all_batches)) if n not in validation]
    judging.judge(rest)
    rated = len(judged_ratings(suite))
    print(f"judge: {suite.label}: all {len(all_batches)} batches judged, {rated} pairs rated")


def _gate_line(found: Agreement) -> str:
    judge = found.judge
    experts = found.experts
    between = (
        f"F1 {experts.f1:.3f}"
        if found.twice >= MIN_RATED_TWICE
        else f"only {found.twice} pairs rated twice"
    )
    return (
        f"judge: against the experts on {found.judged} rated pairs: "
        f"precision {judge.precision:.3f}, "
        f"recall {judge.recall:.3f}, F1 {judge.f1:.3f}; one expert against the others: "
        f"{between} (gate {GATE_F1})"
    )


def _selection(suite: store.Suite) -> tuple[list[Item], bool]:
    stored = _stored_selection(suite)
    if stored is not None and stored[1]:
        return stored
    run_files = runs.load_runs(suite)
    complete = all(query.id in run_files for query in dataset.load(suite).queries)
    experts = labels.load_experts(suite)
    if complete:
        flags = {query_id: flags_of(run) for query_id, run in run_files.items()}
        chosen = select(experts, flags, len(pool.load(suite).snippets))
    else:
        chosen = validation_items(experts)
    if stored is not None and chosen[: len(stored[0])] != stored[0]:
        message = "the stored selection differs from a fresh one; remove judge/ to start over"
        raise StageError(message)
    data: _SelectionFile = {
        "seed": dataset.SEED,
        "complete": complete,
        "items": [
            {
                "id": item.id,
                "query": item.query,
                "snippet": suite.name_of(item.snippet),
                "reasons": list(item.reasons),
            }
            for item in chosen
        ],
    }
    store.write_json(folder(suite) / _SELECTION_FILE, data)
    return chosen, complete


def _stored_selection(suite: store.Suite) -> tuple[list[Item], bool] | None:
    path = folder(suite) / _SELECTION_FILE
    if not store.exists(path):
        return None
    data = cast("_SelectionFile", store.read_json(path))
    return load_selection(suite), data["complete"]


def selected(suite: store.Suite) -> list[Item]:
    return load_selection(suite) if store.exists(folder(suite) / _SELECTION_FILE) else []


def load_selection(suite: store.Suite) -> list[Item]:
    data = cast("_SelectionFile", store.read_json(folder(suite) / _SELECTION_FILE))
    return [
        Item(item["id"], item["query"], pool.index_of_name(item["snippet"]), tuple(item["reasons"]))
        for item in data["items"]
    ]


def _batch_file(number: int) -> str:
    return f"{number:03d}.json"


@dataclass(frozen=True)
class _Judging:
    suite: store.Suite
    batches: list[list[Item]]
    texts: dict[str, str]
    parallel: int

    def judge(self, numbers: list[int]) -> None:
        pending = [n for n in numbers if not store.exists(folder(self.suite) / _batch_file(n))]
        tasks = [lambda number=number: self._judge_batch(number) for number in pending]
        codex.run_all(tasks, self.parallel)

    def _judge_batch(self, number: int) -> None:
        batch = self.batches[number]
        started = time.perf_counter()
        ratings = self._rate(batch)
        data: _BatchFile = {
            "batch": number,
            "model": codex.MODEL,
            "judged": datetime.now(UTC).isoformat(timespec="seconds"),
            "seconds": round(time.perf_counter() - started, 1),
            "ratings": ratings,
        }
        store.write_json(folder(self.suite) / _batch_file(number), data)
        print(f"judge: {self.suite.label} batch {number} rated {len(ratings)} of {len(batch)}")

    def _rate(self, batch: Sequence[Item]) -> dict[str, int]:
        body = "\n\n".join(self._shown(item) for item in batch)
        expected = {item.id for item in batch}
        ratings: dict[str, int] = {}
        for _ in range(2):
            answer = cast("_Answer", codex.ask(f"{_INSTRUCTIONS}\n\n{body}", _SCHEMA))
            ratings = usable_ratings(answer["judgments"], expected)
            if len(ratings) == len(expected) and len(answer["judgments"]) == len(expected):
                break
        return ratings

    def _shown(self, item: Item) -> str:
        code = shown_code(pool.code(self.suite, item.snippet))
        fence = f"```{self.suite.language}\n{code}\n```"
        return f"### id: {item.id}\nQuery: {self.texts[item.query]}\n{fence}"


def shown_code(code: str) -> str:
    if len(code) <= SHOWN_CHARACTERS:
        return code
    return f"{code[:SHOWN_CHARACTERS]}\n[cut: {len(code) - SHOWN_CHARACTERS:,} more characters]"


def usable_ratings(judgments: Sequence[Judgment], expected: set[str]) -> dict[str, int]:
    seen: dict[str, int] = {}
    repeated: set[str] = set()
    for judgment in judgments:
        if judgment["id"] in seen:
            repeated.add(judgment["id"])
        seen[judgment["id"]] = judgment["relevance"]
    return {
        item_id: rating
        for item_id, rating in sorted(seen.items())
        if item_id in expected and item_id not in repeated
    }


def judged_ratings(suite: store.Suite) -> dict[labels.Pair, int]:
    if not store.exists(folder(suite) / _SELECTION_FILE):
        return {}
    pairs = {item.id: (item.query, item.snippet) for item in load_selection(suite)}
    ratings: dict[labels.Pair, int] = {}
    for path in sorted(folder(suite).glob("[0-9][0-9][0-9].json")):
        data = cast("_BatchFile", store.read_json(path))
        for item_id, rating in data["ratings"].items():
            ratings[pairs[item_id]] = rating
    return ratings


def finished(suite: store.Suite) -> bool:
    stored = _stored_selection(suite)
    if stored is None or not stored[1]:
        return False
    numbers = range(len(batches(stored[0])))
    return all(store.exists(folder(suite) / _batch_file(number)) for number in numbers)
