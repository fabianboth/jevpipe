from dataclasses import dataclass, field

from bench import dataset, labels, metrics, selection, store
from bench.records import GrepRecord, RepeatFile, RunFile

POOL_SIZE = 6


@dataclass(frozen=True)
class Run:
    jev: list[float | None]
    answers: list[bool | None] = field(default_factory=lambda: [False] * POOL_SIZE)
    probabilities: list[float | None] = field(default_factory=lambda: [0.0] * POOL_SIZE)
    grep_any: list[int] = field(default_factory=list[int])
    seconds: float = 10.0
    cost: float = 0.02


def models(query_id: str, run: Run) -> RepeatFile:
    return {
        "query": query_id,
        "jevpipe": {
            "model": "typesafe/jev-1.13",
            "resolved_model": "typesafe/jev-1.13-20260917",
            "started": "2026-09-27T21:00:00+00:00",
            "first": True,
            "wall_seconds": run.seconds,
            "cost": run.cost,
            "failed": sum(p is None for p in run.jev),
            "skipped": 0,
            "probability": run.jev,
        },
        "deepseek": {
            "model": "deepseek/deepseek-v4.1-flash",
            "resolved_model": "deepseek/deepseek-v4.1-flash",
            "started": "2026-09-27T21:00:10+00:00",
            "first": False,
            "wall_seconds": 30.0,
            "cost": 0.03,
            "failed": 0,
            "providers": {"DekaLLM": POOL_SIZE},
            "answer": run.answers,
            "probability": run.probabilities,
        },
    }


def run_file(query_id: str, run: Run) -> RunFile:
    grep: GrepRecord = {
        "version": "ripgrep 15.2.0",
        "any": run.grep_any,
        "all": [],
        "agent": [],
        "seconds": {"any": 0.01, "all": 0.02, "agent": 0.03},
    }
    return {**models(query_id, run), "grep": grep}


QUERIES = dataset.Queries(
    (
        dataset.Query("q00", "aes encryption", "dev"),
        dataset.Query("q01", "sort a list", "test"),
        dataset.Query("q02", "parse json", "test"),
    ),
    ("q01",),
)
EXPERT: dict[labels.Pair, tuple[int, ...]] = {
    ("q00", 0): (3,),
    ("q00", 1): (0,),
    ("q00", 2): (2,),
    ("q01", 0): (3, 3),
    ("q01", 1): (1,),
    ("q02", 4): (2,),
}
JUDGE: dict[labels.Pair, int] = {
    ("q01", 2): 2,
    ("q01", 3): 0,
    ("q02", 0): 3,
    ("q02", 1): 0,
    ("q02", 5): 1,
}
RUNS = {
    "q00": run_file(
        "q00",
        Run(
            [0.9, 0.6, 0.4, 0.1, 0.1, 0.1],
            [True, False, True, False, False, False],
            [0.95, None, 0.7, 0.2, 0.1, 0.1],
        ),
    ),
    "q01": run_file("q01", Run([0.8, 0.7, 0.2, 0.5, 0.0, None], grep_any=[0, 1, 2, 3, 4])),
    "q02": run_file("q02", Run([0.1, 0.1, 0.1, 0.1, 0.9, 0.9], seconds=20.0, cost=0.04)),
}
REPEATS = {"q01": models("q01", Run([0.8, 0.2, 0.2, 0.5, 0.0, None]))}
SELECTION = [
    selection.Item("j00001", "q01", 2, ("grep-any-sample",)),
    selection.Item("j00002", "q01", 3, ("grep-any-sample",)),
    selection.Item("j00003", "q02", 0, ("unflagged-sample",)),
    selection.Item("j00004", "q02", 1, ("unflagged-sample",)),
]


def data(suite: store.Suite = store.PYTHON) -> metrics.Data:
    return metrics.Data(
        QUERIES,
        labels.Labels(EXPERT, JUDGE),
        RUNS,
        REPEATS if suite.primary else {},
        SELECTION,
        POOL_SIZE,
        suite,
    )


def scored(suite: store.Suite = store.PYTHON) -> metrics.Scored:
    return metrics.scored(data(suite), {"jevpipe": 0.3, "deepseek": 0.3})
