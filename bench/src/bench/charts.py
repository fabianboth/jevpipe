from datetime import date

from bench import languages, metrics, store, wording
from bench.counts import Counts
from bench.plots import figures, headline, style

HEADLINE_FILE = store.RESULTS / "chart.png"
TRADEOFF_FILE = store.RESULTS / "tradeoff.png"
CALIBRATION_FILE = store.RESULTS / "calibration.png"
WORDINGS_FILE = store.RESULTS / "wordings.png"
LANGUAGES_FILE = store.RESULTS / "languages.png"
TIMES_FILE = store.RESULTS / "times.png"
_ROWS: tuple[tuple[metrics.Contender, str], ...] = (
    ("grep-agent", "grep"),
    ("jevpipe", "jevpipe"),
    ("deepseek", "DeepSeek V4.1 Flash"),
)
_COLORS: dict[metrics.Model, str] = {"jevpipe": style.JEVPIPE, "deepseek": style.DEEPSEEK}
_MODEL_NAMES: dict[metrics.Model, str] = {"jevpipe": "jevpipe", "deepseek": "DeepSeek V4.1 Flash"}
_BASELINES: tuple[tuple[metrics.Contender, str], ...] = (
    ("grep-agent", "grep, agent's pattern"),
    ("grep-all", "grep, all keywords"),
    ("grep-any", "grep, any keyword"),
)


def draw_all(pooled: list[metrics.Scored]) -> None:
    headline.draw(_headline(pooled), HEADLINE_FILE)
    figures.tradeoff(_tradeoff(pooled), TRADEOFF_FILE)
    figures.calibration(_calibration(pooled), CALIBRATION_FILE)
    figures.dot_rows(_wordings(), WORDINGS_FILE)
    figures.times(_times(pooled), TIMES_FILE)


def _footnote(pooled: list[metrics.Scored]) -> str:
    versions = sorted({v for s in pooled for v in metrics.resolved(s.data, "jevpipe")})
    return (
        f"Extended CodeSearchNet Challenge: {languages.searches(pooled)} test searches in "
        f"{languages.names(pooled)}, {_month(languages.last_run(pooled))}.\n"
        f"Jev {', '.join(versions)} · DeepSeek V4.1 Flash via OpenRouter · grep pattern "
        "written by GPT-6 Astra"
    )


def _headline(pooled: list[metrics.Scored]) -> headline.Headline:
    counts = {contender: languages.pooled(pooled, contender) for contender, _ in _ROWS}
    rows = tuple(
        headline.Row(
            name,
            _detail(pooled, contender),
            counts[contender].found,
            counts[contender].false_hits,
        )
        for contender, name in _ROWS
    )
    searches = languages.searches(pooled)
    header = style.Header(
        _title(counts["jevpipe"], counts["grep-agent"]),
        f"{searches} searches in {languages.names(pooled)}: "
        f"{languages.decisions(pooled):,} yes/no decisions per tool",
        "The extended CodeSearchNet Challenge: every search over every function of its language. "
        "Relevance by the Challenge's\nexperts and, for unrated pairs, a judge model checked "
        "against them. "
        f"Jev 1.13, DeepSeek V4.1 Flash; {_month(metrics.dates(pooled[0].data)['last'])}.",
    )
    known = counts["jevpipe"].known_relevant
    return headline.Headline(
        header,
        f"functions, summed over the {searches} test searches (answer key: {known:,.0f} relevant)",
        rows,
    )


def _detail(pooled: list[metrics.Scored], contender: metrics.Contender) -> str:
    match contender:
        case "grep-any" | "grep-all" | "grep-agent":
            return "agent's pattern · free · instant"
        case "jevpipe" | "deepseek":
            seconds, cost = languages.per_1000(pooled, contender)
            return f"${cost:.3f} and {seconds:.0f} s per 1,000 files"


def _title(jev: Counts, grep: Counts) -> str:
    more = jev.found / grep.found - 1 if grep.found else 0.0
    fewer = ", with fewer false hits" if jev.false_hits < grep.false_hits else ""
    return f"jevpipe finds {more:.0%} more relevant code than grep{fewer}"


def _tradeoff(pooled: list[metrics.Scored]) -> figures.Tradeoff:
    sweeps: dict[metrics.Model, list[tuple[float, Counts]]] = {
        model: languages.sweep(pooled, model) for model in metrics.MODELS
    }
    curves = tuple(
        figures.Curve(
            _MODEL_NAMES[model],
            _COLORS[model],
            tuple(
                (threshold, counts.recall, counts.precision) for threshold, counts in sweeps[model]
            ),
            pooled[0].thresholds[model],
        )
        for model in metrics.MODELS
    )
    baselines = tuple(
        figures.Baseline(
            name,
            languages.pooled(pooled, contender).recall,
            languages.pooled(pooled, contender).precision,
        )
        for contender, name in _BASELINES
    )
    header = style.Header(
        _tradeoff_title(sweeps),
        "Precision and recall for thresholds 0.3 to 0.9 (0.5 is the default); grep has none",
        _footnote(pooled),
    )
    return figures.Tradeoff(header, curves, baselines)


def _tradeoff_title(sweeps: dict[metrics.Model, list[tuple[float, Counts]]]) -> str:
    ahead = all(
        any(
            jev.recall >= other.recall and jev.precision > other.precision
            for _, jev in sweeps["jevpipe"]
        )
        for _, other in sweeps["deepseek"]
    )
    if ahead:
        return "At every recall DeepSeek reaches, jevpipe is more precise"
    return "jevpipe and DeepSeek trade precision for recall differently"


def _calibration(pooled: list[metrics.Scored]) -> figures.Calibration:
    bands = {model: languages.bands(pooled, model) for model in metrics.MODELS}
    labels = tuple(f"{band.low:.1f}-{band.high:.1f}" for band in bands["jevpipe"])
    series = tuple(
        figures.Bars(
            _MODEL_NAMES[model],
            _COLORS[model],
            tuple(band.accuracy for band in bands[model]),
            tuple(band.decisions for band in bands[model]),
        )
        for model in metrics.MODELS
    )
    header = style.Header(
        "DeepSeek is nearly always sure; jevpipe grades its answers",
        "Share of decisions at 0.5 that were right, by how sure the model was (white: pairs)",
        _footnote(pooled),
    )
    return figures.Calibration(header, labels, series)


def _wordings() -> figures.DotRows:
    chosen = wording.frozen_wordings()
    series = tuple(
        figures.Dots(
            _MODEL_NAMES[model],
            _COLORS[model],
            wording.trial_f1s(model),
            chosen[model] - 1,
        )
        for model in metrics.MODELS
    )
    labels = tuple(template.replace("{query}", "…") for template in wording.WORDINGS)
    header = style.Header(
        "The wording moves F1 by up to 0.11 on the dev queries",
        "F1 of the four question wordings on the 20 dev queries; the ringed one was kept",
        "Each model kept its best wording before any test query was scored.",
    )
    return figures.DotRows(header, labels, series, "F1 on the dev queries' expert-rated pairs")


def _times(pooled: list[metrics.Scored]) -> figures.Times:
    strips = tuple(
        figures.Strip(
            _MODEL_NAMES[model], _COLORS[model], languages.seconds_per_1000(pooled, model)
        )
        for model in metrics.MODELS
    )
    jev_seconds, jev_cost = languages.per_1000(pooled, "jevpipe")
    other_seconds, other_cost = languages.per_1000(pooled, "deepseek")
    grep = max(scored.results["grep-agent"].seconds_median for scored in pooled)
    header = style.Header(
        f"jevpipe reads 1,000 functions in {jev_seconds:.0f} s, DeepSeek in {other_seconds:.0f} s",
        f"100 requests in flight each; median ${jev_cost:.3f} against ${other_cost:.3f} per "
        f"1,000 functions; grep takes under {grep:.1f} s a search",
        _footnote(pooled),
    )
    return figures.Times(
        header, strips, "seconds per 1,000 functions (one dot per search, time over its folder)"
    )


def draw_languages(everything: list[metrics.Scored], pooled: list[metrics.Scored]) -> None:
    figures.dot_rows(_languages(everything, pooled), LANGUAGES_FILE)


def _languages(everything: list[metrics.Scored], pooled: list[metrics.Scored]) -> figures.DotRows:
    rows = [
        (
            f"{scored.data.suite.label} · {len(scored.data.ids('test'))} searches"
            + ("" if scored in pooled else " · judge missed its check"),
            {c: scored.results[c].counts.f1 for c in languages.SHOWN},
        )
        for scored in everything
    ]
    rows.append(
        (
            f"Pooled, {len(pooled)} languages · {languages.searches(pooled)} searches",
            {c: languages.pooled(pooled, c).f1 for c in languages.SHOWN},
        )
    )
    series = tuple(
        figures.Dots(name, color, tuple(values[contender] for _, values in rows), None)
        for contender, name, color in (
            ("grep-agent", "grep, agent's pattern", style.GREP),
            ("deepseek", "DeepSeek V4.1 Flash", style.DEEPSEEK),
            ("jevpipe", "jevpipe", style.JEVPIPE),
        )
    )
    over_grep = sum(1 for _, values in rows[:-1] if values["jevpipe"] > values["grep-agent"])
    over_deepseek = sum(1 for _, values in rows[:-1] if values["jevpipe"] > values["deepseek"])
    header = style.Header(
        f"jevpipe beats grep in {_count_of(over_grep, len(everything))} languages "
        f"and DeepSeek in {_count_of(over_deepseek, len(everything))}",
        "F1 per language on its test searches, thresholds fixed on the Python dev searches",
        "Extended CodeSearchNet Challenge. Pooled over the languages whose judge passed its check "
        "against the experts.\nGrep patterns written per language by GPT-6 Astra.",
    )
    return figures.DotRows(header, tuple(label for label, _ in rows), series, "F1")


def _count_of(count: int, total: int) -> str:
    return f"all {total}" if count == total else f"{count} of {total}"


def _month(day: str) -> str:
    return f"{date.fromisoformat(day):%B %Y}"
