from dataclasses import dataclass, field

import pytest

from bench import dataset, judge, labels, metrics, runs, store

_POOL_SIZE = 6


@dataclass(frozen=True)
class _Run:
    jev: list[float | None]
    answers: list[bool | None] = field(default_factory=lambda: [False] * _POOL_SIZE)
    probabilities: list[float | None] = field(default_factory=lambda: [0.0] * _POOL_SIZE)
    grep_any: list[int] = field(default_factory=list[int])
    seconds: float = 10.0
    cost: float = 0.02


def _models(query_id: str, run: _Run) -> runs.RepeatFile:
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
            "providers": {"DekaLLM": _POOL_SIZE},
            "answer": run.answers,
            "probability": run.probabilities,
        },
    }


def _run(query_id: str, run: _Run) -> runs.RunFile:
    grep: runs.GrepRecord = {
        "version": "ripgrep 15.2.0",
        "any": run.grep_any,
        "all": [],
        "agent": [],
        "seconds": {"any": 0.01, "all": 0.02, "agent": 0.03},
    }
    return {**_models(query_id, run), "grep": grep}


_QUERIES = dataset.Queries(
    (
        dataset.Query("q00", "aes encryption", "dev"),
        dataset.Query("q01", "sort a list", "test"),
        dataset.Query("q02", "parse json", "test"),
    ),
    ("q01",),
)
_EXPERT: dict[labels.Pair, tuple[int, ...]] = {
    ("q00", 0): (3,),
    ("q00", 1): (0,),
    ("q00", 2): (2,),
    ("q01", 0): (3, 3),
    ("q01", 1): (1,),
    ("q02", 4): (2,),
}
_JUDGE: dict[labels.Pair, int] = {
    ("q01", 2): 2,
    ("q01", 3): 0,
    ("q02", 0): 3,
    ("q02", 1): 0,
    ("q02", 5): 1,
}
_RUNS = {
    "q00": _run(
        "q00",
        _Run(
            [0.9, 0.6, 0.4, 0.1, 0.1, 0.1],
            [True, False, True, False, False, False],
            [0.95, None, 0.7, 0.2, 0.1, 0.1],
        ),
    ),
    "q01": _run("q01", _Run([0.8, 0.7, 0.2, 0.5, 0.0, None], grep_any=[0, 1, 2, 3, 4])),
    "q02": _run("q02", _Run([0.1, 0.1, 0.1, 0.1, 0.9, 0.9], seconds=20.0, cost=0.04)),
}
_REPEATS = {"q01": _models("q01", _Run([0.8, 0.2, 0.2, 0.5, 0.0, None]))}
_SELECTION = [
    judge.Item("j00001", "q01", 2, ("grep-any-sample",)),
    judge.Item("j00002", "q01", 3, ("grep-any-sample",)),
    judge.Item("j00003", "q02", 0, ("unflagged-sample",)),
    judge.Item("j00004", "q02", 1, ("unflagged-sample",)),
]
_DATA = metrics.Data(
    _QUERIES,
    labels.Labels(_EXPERT, _JUDGE),
    _RUNS,
    _REPEATS,
    _SELECTION,
    _POOL_SIZE,
    store.PYTHON,
)


def test_thresholds_are_chosen_on_dev_by_f1() -> None:
    assert metrics.choose_threshold(_DATA, "jevpipe") == 0.3
    assert metrics.choose_threshold(_DATA, "deepseek") == 0.3


def test_model_result_counts_labelled_hits_on_test_queries() -> None:
    result = metrics.result(_DATA, "jevpipe", 0.3)

    counts = result.counts
    assert (counts.found, counts.false_hits, counts.known_relevant) == (2, 3, 4)
    assert counts.precision == pytest.approx(0.4)
    assert counts.recall == pytest.approx(0.5)
    assert counts.f1 == pytest.approx(4 / 9)
    assert result.unjudged == 0
    assert result.seconds_median == 15.0
    assert result.cost_per_1000 == pytest.approx(5.0)


def test_a_higher_threshold_flags_fewer() -> None:
    counts = metrics.model_counts(_DATA, "jevpipe", 0.75)

    assert (counts.found, counts.false_hits) == (2, 1)


def test_grep_any_estimates_its_unrated_hits_from_its_sample() -> None:
    counts = metrics.result(_DATA, "grep-any", 0.0).counts

    assert counts.found == pytest.approx(2.5)
    assert counts.false_hits == pytest.approx(2.5)
    assert counts.known_relevant == pytest.approx(4.5)


def test_bands_group_labelled_decisions_by_confidence() -> None:
    bands = metrics.bands(_DATA, "jevpipe")

    assert [(band.low, band.decisions) for band in bands] == [
        (0.5, 1),
        (0.6, 0),
        (0.7, 1),
        (0.8, 2),
        (0.9, 4),
    ]
    assert [band.accuracy for band in bands] == [0.0, 0.0, 0.0, 0.5, 0.5]


def test_missed_estimate_scales_the_unflagged_sample() -> None:
    missed = metrics.missed(_DATA)

    assert (missed.sampled, missed.relevant_in_sample, missed.unflagged) == (2, 1, 5)
    assert missed.estimate == pytest.approx(2.5)


def test_repeat_changes_are_the_share_of_flips() -> None:
    changes = metrics.repeat_changes(_DATA, {"jevpipe": 0.3, "deepseek": 0.3})

    assert changes["jevpipe"] == pytest.approx(1 / _POOL_SIZE)
    assert changes["deepseek"] == 0.0


def test_sweep_covers_point_three_to_point_nine() -> None:
    thresholds = [threshold for threshold, _ in metrics.sweep(_DATA, "jevpipe")]

    assert thresholds[0] == 0.3
    assert thresholds[-1] == 0.9
    assert 0.5 in thresholds
    assert 0.7 in thresholds


def test_per_query_counts_found_false_hits_and_relevant() -> None:
    assert metrics.per_query(_DATA, "jevpipe", 0.3) == [(1, 2, 2), (1, 1, 2)]
