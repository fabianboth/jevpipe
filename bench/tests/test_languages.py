import pytest
import sample

from bench import languages, metrics, store
from bench.counts import Counts

_PYTHON = sample.scored()
_JAVA = sample.scored(store.SUITES["java"])


def test_pooled_counts_add_up_the_languages() -> None:
    one = _PYTHON.results["jevpipe"].counts

    assert languages.pooled([_PYTHON, _JAVA], "jevpipe") == one + one


def test_the_sweep_adds_up_each_threshold() -> None:
    alone = dict(metrics.sweep(_PYTHON.data, "jevpipe"))

    for threshold, pooled in languages.sweep([_PYTHON, _JAVA], "jevpipe"):
        assert pooled == alone[threshold] + alone[threshold]


def test_bands_merge_their_counts_and_leave_empty_bands_empty() -> None:
    alone = metrics.bands(_PYTHON.data, "jevpipe")

    merged = languages.bands([_PYTHON, _JAVA], "jevpipe")

    assert [band.decisions for band in merged] == [2 * band.decisions for band in alone]
    assert [band.accuracy for band in merged] == [band.accuracy for band in alone]
    assert merged[1].accuracy is None


def test_a_gap_is_the_difference_of_pooled_f1() -> None:
    gap = languages.gap([_PYTHON, _JAVA], ("jevpipe", "grep-agent"))

    jev_f1 = languages.pooled([_PYTHON, _JAVA], "jevpipe").f1
    grep_f1 = languages.pooled([_PYTHON, _JAVA], "grep-agent").f1
    assert gap.observed == pytest.approx(jev_f1 - grep_f1)


def test_the_median_per_1000_scales_each_search_by_its_pool() -> None:
    found = languages.median_per_1000([_PYTHON], "jevpipe")

    assert found.seconds == pytest.approx(15.0 / sample.POOL_SIZE * 1000)
    assert found.cost == pytest.approx(0.03 / sample.POOL_SIZE * 1000)


def test_spend_per_1000_is_the_total_over_every_decision() -> None:
    spend = languages.spend_per_1000([_PYTHON, _JAVA], "jevpipe")

    assert spend == pytest.approx((0.02 + 0.04) * 2 / (4 * sample.POOL_SIZE) * 1000)


def test_languages_with_files_deepseek_read_in_full_are_left_out_of_cost() -> None:
    javascript = sample.scored(store.SUITES["javascript"])

    assert languages.costed([_PYTHON, javascript, _JAVA]) == [_PYTHON, _JAVA]
    assert languages.costed([javascript]) == [javascript]


def test_searches_decisions_and_names() -> None:
    assert languages.searches([_PYTHON, _JAVA]) == 4
    assert languages.decisions([_PYTHON, _JAVA]) == 4 * sample.POOL_SIZE
    assert languages.names([_PYTHON]) == "Python"
    assert languages.names([_PYTHON, _JAVA, _PYTHON]) == "Python, Java and Python"


def test_counts_add_up() -> None:
    assert Counts(1, 2, 3) + Counts(4, 5, 6) == Counts(5, 7, 9)
