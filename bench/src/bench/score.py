from bench import charts, dataset, metrics, numbers, pool, report, runs, selection, store
from bench.contenders import NAMES, SHOWN

_RESULTS_FILE = "results.json"
_NUMBERS_FILE = "numbers.md"
_LANGUAGES_FILE = "languages.json"


def score() -> None:
    primary = _load(store.PYTHON)
    thresholds = metrics.chosen_thresholds(primary)
    everything = [
        metrics.scored(_load(suite), thresholds)
        for suite in store.SUITES.values()
        if suite.primary or selection.finished(suite)
    ]
    for scored in everything:
        _write(scored)
    passed = [scored for scored in everything if scored.agreement.passed]
    store.write_json(store.RESULTS / _LANGUAGES_FILE, report.all_languages(everything, passed))
    charts.draw_all(passed)
    charts.draw_languages(everything, passed)
    for scored in everything:
        print(_headline(scored))


def _load(suite: store.Suite) -> metrics.Data:
    return metrics.Data(
        dataset.load(suite),
        selection.labels_of(suite),
        runs.load_runs(suite),
        runs.load_repeats() if suite.primary else {},
        selection.selected(suite),
        len(pool.load(suite).snippets),
        suite,
    )


def _write(scored: metrics.Scored) -> None:
    summary = report.summarize(scored)
    language_report = report.language(summary)
    folder = scored.data.suite.results
    store.write_json(folder / _RESULTS_FILE, language_report)
    rendered = numbers.render(summary, language_report)
    (folder / _NUMBERS_FILE).write_text(rendered, encoding="utf-8", newline="\n")


def _headline(scored: metrics.Scored) -> str:
    parts = [f"{NAMES[c]} {scored.results[c].counts.f1:.2f}" for c in SHOWN]
    return f"score: {scored.data.suite.label}: F1 " + ", ".join(parts)
