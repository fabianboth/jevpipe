import itertools
import math
import statistics
from datetime import date
from pathlib import Path

from bench import (
    codex,
    dataset,
    decisions,
    deepseek,
    jev,
    languages,
    metrics,
    openrouter,
    store,
    studies,
    wording,
)
from bench.contenders import MODELS, NAMES, SHORT_NAMES, SHOWN, Contender, Model
from bench.counts import Counts
from bench.plots import figures, headline, style

_COLORS: dict[Contender, str] = {
    "grep-any": style.GREP,
    "grep-all": style.GREP,
    "grep-agent": style.GREP,
    "jevpipe": style.JEVPIPE,
    "deepseek": style.DEEPSEEK,
}
_BASELINES: tuple[Contender, ...] = ("grep-agent", "grep-all", "grep-any")
_LANGUAGE_ORDER: tuple[Contender, ...] = ("grep-agent", "deepseek", "jevpipe")
_SURE = 0.9
_LEVEL = 0.03


def draw_all(pooled: list[metrics.Scored]) -> None:
    headline.draw(_headline(pooled), _file("chart.png"))
    figures.tradeoff(_tradeoff(pooled), _file("tradeoff.png"))
    figures.calibration(_calibration(pooled), _file("calibration.png"))
    figures.dot_rows(_wordings(), _file("wordings.png"))
    figures.times(_times(pooled), _file("times.png"))


def draw_studies() -> None:
    figures.cost_bars(_classification(studies.ay_automate()), _file("classification.png"))


def _classification(study: studies.Study) -> figures.CostBars:
    ranked = sorted(study.results, key=lambda result: result.accuracy, reverse=True)
    rows = tuple(
        figures.CostRow(
            result.name,
            result.cost_per_1000,
            (f"{result.accuracy:.1%}", f"{result.seconds:.2f} s"),
            stressed=result is study.jev,
        )
        for result in ranked
    )
    header = style.Header(
        study_title(study),
        f"Independent study, mean of {len(study.tasks)} labelled tasks: {', '.join(study.tasks)}",
        f"Data: {study.source}, {_day(study.published)}; {study.decisions:,} labelled "
        "decisions.\nRedrawn from its published tables; cost from OpenRouter prices at the time.",
    )
    return figures.CostBars(header, rows, ("accuracy", "median time"), "cost per 1,000 decisions")


def study_title(study: studies.Study) -> str:
    jev_result = study.jev
    cheapest = min(result.cost_per_1000 for result in study.others)
    ratio = cheapest / jev_result.cost_per_1000
    best = max(result.accuracy for result in study.others)
    small = [result.accuracy for result in study.others if result.accuracy < best]
    level = bool(small) and jev_result.accuracy >= statistics.mean(small) - _LEVEL
    accuracy = "as accurately as small LLMs" if level else "against LLMs"
    return f"Jev classifies {accuracy}, {ratio:.0f} times cheaper"


def draw_languages(everything: list[metrics.Scored], pooled: list[metrics.Scored]) -> None:
    figures.dot_rows(_languages(everything, pooled), _file("languages.png"))


def _file(name: str) -> Path:
    return store.RESULTS / name


def _models_line(pooled: list[metrics.Scored]) -> str:
    versions = sorted({v for s in pooled for v in metrics.resolved(s.data, "jevpipe")})
    return (
        f"Jev {', '.join(versions)} · {deepseek.LABEL} via OpenRouter · grep pattern "
        f"written by {codex.LABEL}"
    )


def _set_aside(pooled: list[metrics.Scored]) -> str:
    left_out = [s.data.suite.label for s in pooled if s not in languages.costed(pooled)]
    if not left_out:
        return ""
    return (
        f"\nCost and time leave out {' and '.join(left_out)}: DeepSeek read its minified bundles "
        "in full, jevpipe cut them."
    )


def _short_set_aside(pooled: list[metrics.Scored]) -> str:
    left_out = [s.data.suite.label for s in pooled if s not in languages.costed(pooled)]
    if not left_out:
        return ""
    return f"Cost and time without {' and '.join(left_out)}: DeepSeek read its bundles in full."


def _footnote(pooled: list[metrics.Scored]) -> str:
    return (
        f"Extended CodeSearchNet Challenge: {languages.searches(pooled)} test searches in "
        f"{languages.names(pooled)}, {_month(languages.last_run(pooled))}.\n" + _models_line(pooled)
    )


def _headline(pooled: list[metrics.Scored]) -> headline.Headline:
    counts = {contender: languages.pooled(pooled, contender) for contender in SHOWN}
    rows = tuple(
        headline.Row(
            SHORT_NAMES[contender],
            _detail(pooled, contender),
            counts[contender].found,
            counts[contender].false_hits,
        )
        for contender in SHOWN
    )
    searches = languages.searches(pooled)
    header = style.Header(
        title(counts["jevpipe"], counts["grep-agent"]),
        f"{searches} searches in {languages.names(pooled)}: "
        f"{languages.decisions(pooled):,} yes/no decisions per tool",
        "Extended CodeSearchNet Challenge: relevance by its experts or, for unrated pairs, a judge "
        f"model checked against them.\n{jev.LABEL}, {deepseek.LABEL}, "
        f"{_month(languages.last_run(pooled))}. {_short_set_aside(pooled)}",
    )
    known = counts["jevpipe"].known_relevant
    return headline.Headline(
        header,
        f"functions, summed over the {searches} test searches (answer key: {known:,.0f} relevant)",
        rows,
    )


def _detail(pooled: list[metrics.Scored], contender: Contender) -> str:
    match contender:
        case "grep-any" | "grep-all" | "grep-agent":
            return "agent's pattern · free · instant"
        case "jevpipe" | "deepseek":
            costed = languages.costed(pooled)
            spend = languages.spend_per_1000(costed, contender)
            seconds = languages.median_per_1000(costed, contender).seconds
            return f"${spend:.3f} and {seconds:.0f} s per 1,000 files"


def title(jev_counts: Counts, grep: Counts) -> str:
    if not grep.found:
        return "jevpipe finds relevant code where grep finds none"
    change = jev_counts.found / grep.found - 1
    amount = f"{abs(change):.0%} {'more' if change >= 0 else 'less'}"
    fewer = ", with fewer false hits" if jev_counts.false_hits < grep.false_hits else ""
    return f"jevpipe finds {amount} relevant code than grep{fewer}"


def _tradeoff(pooled: list[metrics.Scored]) -> figures.Tradeoff:
    sweeps: dict[Model, list[tuple[float, Counts]]] = {
        model: languages.sweep(pooled, model) for model in MODELS
    }
    curves = tuple(
        figures.Curve(
            NAMES[model],
            _COLORS[model],
            tuple(
                (threshold, counts.recall, counts.precision) for threshold, counts in sweeps[model]
            ),
            pooled[0].thresholds[model],
        )
        for model in MODELS
    )
    baselines = tuple(
        figures.Baseline(
            NAMES[contender],
            languages.pooled(pooled, contender).recall,
            languages.pooled(pooled, contender).precision,
        )
        for contender in _BASELINES
    )
    lowest, highest = decisions.THRESHOLDS[0], decisions.THRESHOLDS[-1]
    header = style.Header(
        tradeoff_title(sweeps),
        f"Precision and recall for thresholds {lowest} to {highest}; ringed: the threshold "
        "chosen on the Python tuning searches. grep has none",
        _footnote(pooled),
    )
    return figures.Tradeoff(header, curves, baselines)


def tradeoff_title(sweeps: dict[Model, list[tuple[float, Counts]]]) -> str:
    ahead = all(
        any(
            jev_counts.recall >= other.recall and jev_counts.precision > other.precision
            for _, jev_counts in sweeps["jevpipe"]
        )
        for _, other in sweeps["deepseek"]
    )
    if ahead:
        return "At every recall DeepSeek reaches, jevpipe is more precise"
    return "jevpipe and DeepSeek trade precision for recall differently"


def _calibration(pooled: list[metrics.Scored]) -> figures.Calibration:
    bands: dict[Model, list[metrics.Band]] = {
        model: languages.bands(pooled, model) for model in MODELS
    }
    labels = tuple(f"{band.low:.1f}-{band.high:.1f}" for band in bands["jevpipe"])
    series = tuple(
        figures.Bars(
            NAMES[model],
            _COLORS[model],
            tuple(band.accuracy or 0.0 for band in bands[model]),
            tuple(band.decisions for band in bands[model]),
        )
        for model in MODELS
    )
    header = style.Header(
        calibration_title(bands),
        "Share of decisions at 0.5 that were right, by how sure the model was (white: pairs)",
        _footnote(pooled),
    )
    return figures.Calibration(header, labels, series)


def calibration_title(bands: dict[Model, list[metrics.Band]]) -> str:
    sure = {model: _sure_share(bands[model]) for model in MODELS}
    graded = _rising(bands["jevpipe"]) and not _rising(bands["deepseek"])
    if graded and sure["deepseek"] > sure["jevpipe"]:
        return "DeepSeek is nearly always sure; jevpipe grades its answers"
    return (
        f"Sure of {sure['jevpipe']:.0%} of its answers, jevpipe; "
        f"of {sure['deepseek']:.0%}, DeepSeek"
    )


def _sure_share(bands: list[metrics.Band]) -> float:
    total = sum(band.decisions for band in bands)
    sure = sum(band.decisions for band in bands if band.low >= _SURE)
    return sure / total if total else 0.0


def _rising(bands: list[metrics.Band]) -> bool:
    accuracies = [band.accuracy for band in bands if band.accuracy is not None]
    return all(low <= high for low, high in itertools.pairwise(accuracies))


def _wordings() -> figures.DotRows:
    chosen = wording.frozen_wordings()
    f1s = {model: wording.trial_f1s(model) for model in MODELS}
    series = tuple(
        figures.Dots(NAMES[model], _COLORS[model], f1s[model], chosen[model] - 1)
        for model in MODELS
    )
    spread = max(max(values) - min(values) for values in f1s.values())
    labels = tuple(template.replace("{query}", "…") for template in wording.WORDINGS)
    header = style.Header(
        f"The wording moves F1 by up to {spread:.2f} on the tuning searches",
        f"F1 of the {len(wording.WORDINGS)} question wordings on the {dataset.DEV_QUERIES} "
        "Python tuning searches; the ringed one was kept",
        "Each model kept its best wording before any test search was scored.",
    )
    return figures.DotRows(header, labels, series, "F1 on the tuning searches' expert-rated pairs")


def _times(pooled: list[metrics.Scored]) -> figures.Times:
    costed = languages.costed(pooled)
    strips = tuple(
        figures.Strip(NAMES[model], _COLORS[model], languages.seconds_per_1000(costed, model))
        for model in MODELS
    )
    seconds = {model: languages.median_per_1000(costed, model).seconds for model in MODELS}
    spend = {model: languages.spend_per_1000(costed, model) for model in MODELS}
    slowest = max(
        scored.data.seconds(query_id, "grep-agent")
        for scored in costed
        for query_id in scored.data.ids("test")
    )
    header = style.Header(
        f"jevpipe reads 1,000 functions in {seconds['jevpipe']:.0f} s, "
        f"DeepSeek in {seconds['deepseek']:.0f} s",
        f"{openrouter.IN_FLIGHT} requests in flight each; ${spend['jevpipe']:.3f} against "
        f"${spend['deepseek']:.3f} per 1,000 functions; grep takes at most "
        f"{math.ceil(slowest * 10) / 10:.1f} s a search",
        _footnote(costed) + _set_aside(pooled),
    )
    return figures.Times(
        header, strips, "seconds per 1,000 functions (one dot per search, time over its folder)"
    )


def _languages(everything: list[metrics.Scored], pooled: list[metrics.Scored]) -> figures.DotRows:
    rows = [
        (
            f"{scored.data.suite.label} · {len(scored.data.ids('test'))} searches"
            + ("" if scored in pooled else " · judge missed its check"),
            {c: scored.results[c].counts.f1 for c in SHOWN},
        )
        for scored in everything
    ]
    rows.append(
        (
            f"Pooled, {len(pooled)} languages · {languages.searches(pooled)} searches",
            {c: languages.pooled(pooled, c).f1 for c in SHOWN},
        )
    )
    series = tuple(
        figures.Dots(
            NAMES[contender], _COLORS[contender], tuple(v[contender] for _, v in rows), None
        )
        for contender in _LANGUAGE_ORDER
    )
    over_grep = sum(1 for _, values in rows[:-1] if values["jevpipe"] > values["grep-agent"])
    over_deepseek = sum(1 for _, values in rows[:-1] if values["jevpipe"] > values["deepseek"])
    header = style.Header(
        f"jevpipe beats grep in {count_of(over_grep, len(everything))} languages "
        f"and DeepSeek in {count_of(over_deepseek, len(everything))}",
        "F1 per language on its test searches, thresholds fixed on the Python tuning searches",
        "Extended CodeSearchNet Challenge. Pooled over the languages whose judge passed its check "
        f"against the experts.\nGrep patterns written per language by {codex.LABEL}.",
    )
    return figures.DotRows(header, tuple(label for label, _ in rows), series, "F1")


def count_of(count: int, total: int) -> str:
    return f"all {total}" if count == total else f"{count} of {total}"


def _day(day: str) -> str:
    published = date.fromisoformat(day)
    return f"{published.day} {published:%B %Y}"


def _month(day: str) -> str:
    return f"{date.fromisoformat(day):%B %Y}"
