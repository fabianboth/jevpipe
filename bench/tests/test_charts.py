from pathlib import Path

from bench import charts, metrics, studies
from bench.contenders import Model
from bench.counts import Counts
from bench.plots import figures, style


def test_the_title_says_more_or_less_and_mentions_fewer_false_hits() -> None:
    assert charts.title(Counts(124, 50, 200), Counts(100, 80, 200)) == (
        "jevpipe finds 24% more relevant code than grep, with fewer false hits"
    )
    assert charts.title(Counts(95, 90, 200), Counts(100, 80, 200)) == (
        "jevpipe finds 5% less relevant code than grep"
    )
    assert charts.title(Counts(5, 0, 10), Counts(0, 3, 10)) == (
        "jevpipe finds relevant code where grep finds none"
    )


def test_the_tradeoff_title_needs_jevpipe_ahead_at_every_recall() -> None:
    ahead: dict[Model, list[tuple[float, Counts]]] = {
        "jevpipe": [(0.3, Counts(8, 2, 10)), (0.9, Counts(5, 0, 10))],
        "deepseek": [(0.3, Counts(7, 3, 10)), (0.9, Counts(5, 1, 10))],
    }
    crossed: dict[Model, list[tuple[float, Counts]]] = {
        **ahead,
        "deepseek": [(0.3, Counts(9, 1, 10))],
    }

    assert charts.tradeoff_title(ahead).startswith("At every recall")
    assert not charts.tradeoff_title(crossed).startswith("At every recall")


def _bands(right: list[int], decisions: list[int]) -> list[metrics.Band]:
    lows = [0.5, 0.6, 0.7, 0.8, 0.9]
    return [
        metrics.Band(low, round(low + 0.1, 1), count, hits)
        for low, count, hits in zip(lows, decisions, right, strict=True)
    ]


def test_the_calibration_title_only_claims_what_the_bands_show() -> None:
    graded = _bands([5, 6, 7, 8, 80], [10, 10, 10, 10, 82])
    flat = _bands([6, 5, 6, 5, 95], [10, 10, 10, 10, 100])

    assert charts.calibration_title({"jevpipe": graded, "deepseek": flat}).startswith(
        "DeepSeek is nearly always sure"
    )
    assert charts.calibration_title({"jevpipe": flat, "deepseek": graded}).startswith("Sure of")


def test_counts_of_languages_read_naturally() -> None:
    assert charts.count_of(6, 6) == "all 6"
    assert charts.count_of(4, 6) == "4 of 6"


def test_figures_are_the_same_bytes_every_time(tmp_path: Path) -> None:
    chart = figures.Times(
        style.Header("Title", "Subtitle", "Footnote"),
        (figures.Strip("jevpipe", style.JEVPIPE, (1.0, 2.0, 3.0)),),
        "seconds",
    )
    first, second = tmp_path / "first.png", tmp_path / "second.png"

    figures.times(chart, first)
    figures.times(chart, second)

    assert first.read_bytes() == second.read_bytes()


def _study(jev: tuple[float, float]) -> studies.Study:
    results = (
        studies.Result("Jev 1.13", jev[0], jev[1], 0.3),
        studies.Result("small", 0.82, 0.12, 1.0),
        studies.Result("large", 0.87, 1.1, 1.2),
    )
    return studies.Study("Source", "2026-09-19", ("task",), 100, results)


def test_the_study_title_claims_small_llm_accuracy_only_when_jev_reaches_it() -> None:
    assert charts.study_title(_study((0.83, 0.024))) == (
        "Jev classifies as accurately as small LLMs, 5 times cheaper"
    )
    assert charts.study_title(_study((0.70, 0.024))).startswith("Jev classifies against LLMs")
