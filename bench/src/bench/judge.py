import time
from collections.abc import Sequence
from dataclasses import dataclass
from typing import TypedDict, cast

from bench import agreement, codex, dataset, labels, pool, selection, store
from bench.limits import LimitReachedError, StageError

SHOWN_CHARACTERS = 20_000
_TRIES = 2
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


class Judgment(TypedDict):
    id: str
    relevance: int


class _Answer(TypedDict):
    judgments: list[Judgment]


def judge_pairs(suite: store.Suite, parallel: int, *, continue_judging: bool) -> None:
    chosen = selection.current(suite)
    all_batches = chosen.batches()
    texts = {query.id: query.text for query in dataset.load(suite).queries}
    judging = _Judging(suite, all_batches, texts, parallel)
    validation = [n for n, batch in enumerate(all_batches) if batch[0].validation]
    judging.judge(validation)
    found = agreement.agreement(labels.load_experts(suite), selection.judged_ratings(suite))
    print(f"{suite.label} {agreement.describe(found)}")
    if not found.passed and not continue_judging:
        message = (
            f"the judge's F1 against the experts is {found.judge.counts.f1:.3f}, below "
            f"{agreement.GATE_F1}; rerun with --continue-judging to judge the rest anyway"
        )
        raise LimitReachedError(message)
    if not chosen.complete:
        message = "validation is judged; run every query, then rerun judge for the rest"
        raise StageError(message)
    judging.judge([n for n in range(len(all_batches)) if n not in validation])
    rated = len(selection.judged_ratings(suite))
    print(f"judge: {suite.label}: all {len(all_batches)} batches judged, {rated} pairs rated")


@dataclass(frozen=True)
class _Judging:
    suite: store.Suite
    batches: list[list[selection.Item]]
    texts: dict[str, str]
    parallel: int

    def judge(self, numbers: list[int]) -> None:
        pending = [n for n in numbers if not selection.batch_file(self.suite, n).is_file()]
        tasks = [lambda number=number: self._judge_batch(number) for number in pending]
        codex.run_all(tasks, self.parallel)

    def _judge_batch(self, number: int) -> None:
        batch = self.batches[number]
        started = time.perf_counter()
        ratings = self._rate(batch)
        data: selection.BatchFile = {
            "batch": number,
            "model": codex.MODEL,
            "judged": store.now(),
            "seconds": round(time.perf_counter() - started, 1),
            "ratings": ratings,
        }
        store.write_json(selection.batch_file(self.suite, number), data)
        print(f"judge: {self.suite.label} batch {number} rated {len(ratings)} of {len(batch)}")

    def _rate(self, batch: Sequence[selection.Item]) -> dict[str, int]:
        body = "\n\n".join(self._shown(item) for item in batch)
        expected = {item.id for item in batch}
        for _ in range(_TRIES):
            answer = cast("_Answer", codex.ask(f"{_INSTRUCTIONS}\n\n{body}", _SCHEMA))
            ratings = usable_ratings(answer["judgments"], expected)
            if len(ratings) == len(expected) and len(answer["judgments"]) == len(expected):
                return ratings
        message = f"the judge did not rate every item of a batch in {_TRIES} tries; run again"
        raise StageError(message)

    def _shown(self, item: selection.Item) -> str:
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
