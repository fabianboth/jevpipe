import pytest
import sample

from bench import decisions, labels, metrics, selection, store
from bench.counts import Counts
from bench.decisions import Setting

_DATA = sample.data()


def test_thresholds_are_chosen_on_dev_by_f1() -> None:
    assert metrics.choose_threshold(_DATA, "jevpipe") == 0.3
    assert metrics.choose_threshold(_DATA, "deepseek") == 0.3


def test_model_result_counts_labelled_hits_on_test_queries() -> None:
    result = metrics.result(_DATA, Setting("jevpipe", 0.3))

    counts = result.counts
    assert (counts.found, counts.false_hits, counts.known_relevant) == (2, 3, 4)
    assert counts.precision == pytest.approx(0.4)
    assert counts.recall == pytest.approx(0.5)
    assert counts.f1 == pytest.approx(4 / 9)
    assert result.unjudged == 0
    assert result.seconds_median == 15.0
    assert result.cost_per_1000 == pytest.approx(5.0)


def test_a_higher_threshold_flags_fewer() -> None:
    counts = metrics.model_counts(_DATA, Setting("jevpipe", 0.75))

    assert (counts.found, counts.false_hits) == (2, 1)


def test_grep_any_estimates_its_unrated_hits_from_its_sample() -> None:
    counts = metrics.result(_DATA, Setting("grep-any", 0.0)).counts

    assert counts.found == pytest.approx(2.5)
    assert counts.false_hits == pytest.approx(2.5)
    assert counts.known_relevant == pytest.approx(4.5)


def test_grep_any_counts_at_least_the_relevant_hits_already_known() -> None:
    data = metrics.Data(
        sample.QUERIES,
        labels.Labels(sample.EXPERT, {**sample.JUDGE, ("q01", 4): 3}),
        sample.RUNS,
        {},
        [item for item in sample.SELECTION if (item.query, item.snippet) != ("q01", 2)],
        sample.POOL_SIZE,
        store.PYTHON,
    )

    counts = metrics.result(data, Setting("grep-any", 0.0)).counts

    assert (counts.found, counts.false_hits, counts.known_relevant) == (3, 2, 5)


def test_bands_group_labelled_decisions_by_confidence() -> None:
    bands = metrics.bands(_DATA, "jevpipe")

    assert [(band.low, band.decisions) for band in bands] == [
        (0.5, 1),
        (0.6, 0),
        (0.7, 1),
        (0.8, 2),
        (0.9, 4),
    ]
    assert [band.accuracy for band in bands] == [0.0, None, 0.0, 0.5, 0.5]


def test_missed_estimate_scales_the_unflagged_sample() -> None:
    missed = metrics.missed(_DATA)

    assert (missed.sampled, missed.relevant_in_sample, missed.unflagged) == (2, 1, 5)
    assert missed.estimate == pytest.approx(2.5)


def test_repeat_changes_are_the_share_of_flips() -> None:
    changes = metrics.repeat_changes(sample.scored())

    assert changes == {"jevpipe": pytest.approx(1 / sample.POOL_SIZE), "deepseek": 0.0}


def test_a_language_without_repeats_has_no_repeat_changes() -> None:
    assert metrics.repeat_changes(sample.scored(store.SUITES["java"])) is None


def test_per_query_counts_found_false_hits_and_relevant() -> None:
    assert metrics.per_query(sample.scored(), "jevpipe") == [Counts(1, 2, 2), Counts(1, 1, 2)]


def test_every_scored_hit_was_in_the_judged_pool() -> None:
    for run in sample.RUNS.values():
        pooled = selection.flags_of(run)
        for threshold in decisions.THRESHOLDS:
            assert Setting("jevpipe", threshold).flags(run) <= pooled.jevpipe
            assert Setting("deepseek", threshold).flags(run) <= pooled.deepseek


def test_a_retry_adds_its_time_to_the_run() -> None:
    run = sample.run_file("q01", sample.Run([0.8, 0.7, 0.2, 0.5, 0.0, None], seconds=12.0))
    run["jevpipe"]["retried"] = {"records": 1, "answered": 1, "cost": 0.001, "wall_seconds": 3.0}
    data = metrics.Data(
        sample.QUERIES, _DATA.known, {"q01": run}, {}, [], sample.POOL_SIZE, store.PYTHON
    )

    assert data.seconds("q01", "jevpipe") == 15.0
