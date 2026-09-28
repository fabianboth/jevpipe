import json
from collections import Counter

from bench import (
    bootstrap,
    charts,
    codex,
    dataset,
    judge,
    labels,
    languages,
    metrics,
    pool,
    runs,
    store,
    wording,
)
from bench.counts import Counts

_RESULTS_FILE = "results.json"
_NUMBERS_FILE = "numbers.md"
_LANGUAGES_FILE = store.RESULTS / "languages.json"
_DIGITS = 4
_NAMES: dict[metrics.Contender, str] = {
    "grep-any": "grep, any keyword",
    "grep-all": "grep, all keywords",
    "grep-agent": "grep, agent-written pattern",
    "jevpipe": "jevpipe",
    "deepseek": "DeepSeek V4.1 Flash",
}


def score() -> None:
    primary = _load(store.PYTHON)
    thresholds = metrics.chosen_thresholds(primary)
    python = metrics.scored(primary, thresholds)
    others = [
        metrics.scored(_load(suite), thresholds)
        for suite in store.SUITES.values()
        if not suite.primary and judge.finished(suite)
    ]
    everything = [python, *others]
    for scored in everything:
        _write(scored)
    charts.draw_all(_passed(everything))
    store.write_json(_LANGUAGES_FILE, _languages(everything))
    charts.draw_languages(everything, _passed(everything))
    for scored in everything:
        print(_headline(scored))


def _write(scored: metrics.Scored) -> None:
    folder = scored.data.suite.results
    store.write_json(folder / _RESULTS_FILE, _report(scored))
    (folder / _NUMBERS_FILE).write_text(_numbers(scored), encoding="utf-8", newline="\n")


def _headline(scored: metrics.Scored) -> str:
    parts = [
        f"{_NAMES[contender]} {scored.results[contender].counts.f1:.2f}"
        for contender in languages.SHOWN
    ]
    return f"score: {scored.data.suite.label}: F1 " + ", ".join(parts)


def _load(suite: store.Suite) -> metrics.Data:
    known = labels.Labels(labels.load_experts(suite), judge.judged_ratings(suite))
    return metrics.Data(
        dataset.load(suite),
        known,
        runs.load_runs(suite),
        runs.load_repeats() if suite.primary else {},
        judge.selected(suite),
        len(pool.load(suite).snippets),
        suite,
    )


def _passed(everything: list[metrics.Scored]) -> list[metrics.Scored]:
    return [scored for scored in everything if _agreement(scored).passed]


def _agreement(scored: metrics.Scored) -> judge.Agreement:
    return judge.agreement(scored.data.known.expert, scored.data.known.judge)


def _languages(everything: list[metrics.Scored]) -> dict[str, object]:
    def contenders(counts_of: dict[metrics.Contender, Counts]) -> dict[str, object]:
        return {contender: _counts(counts) for contender, counts in counts_of.items()}

    def pooled(group: list[metrics.Scored]) -> dict[str, object]:
        return {
            "languages": [scored.data.suite.language for scored in group],
            "test_searches": languages.searches(group),
            "contenders": contenders({c: languages.pooled(group, c) for c in languages.SHOWN}),
            "gaps": {
                f"{first} - {second}": _gap(languages.gap(group, (first, second)))
                for first, second in languages.COMPARISONS
            },
            "per_1000": {
                model: dict(zip(("seconds", "cost"), languages.per_1000(group, model), strict=True))
                for model in metrics.MODELS
            },
            "sweep": {
                model: [
                    {"threshold": threshold, **_counts(counts)}
                    for threshold, counts in languages.sweep(group, model)
                ]
                for model in metrics.MODELS
            },
            "bands": {model: _bands(languages.bands(group, model)) for model in metrics.MODELS},
        }

    return {
        "thresholds": everything[0].thresholds,
        "languages": {
            scored.data.suite.language: {
                "test_searches": len(scored.data.ids("test")),
                "pool": scored.data.pool_size,
                "judge": {
                    "f1": _round(_agreement(scored).judge.f1),
                    "passed": _agreement(scored).passed,
                    "pairs_rated_twice": _agreement(scored).twice,
                },
                "contenders": contenders({c: scored.results[c].counts for c in languages.SHOWN}),
            }
            for scored in everything
        },
        "python": pooled(everything[:1]),
        "pooled": pooled(_passed(everything)),
        "pooled_with_every_language": pooled(everything),
    }


def _gap(found: bootstrap.Gap) -> dict[str, float]:
    return {
        "f1_gap": _round(found.observed),
        "low": _round(found.low),
        "high": _round(found.high),
        "above_zero": _round(found.above_zero),
    }


def _round(value: float) -> float:
    return round(value, _DIGITS)


def _counts(counts: Counts) -> dict[str, float]:
    return {
        "found": _round(counts.found),
        "false_hits": _round(counts.false_hits),
        "known_relevant": _round(counts.known_relevant),
        "precision": _round(counts.precision),
        "recall": _round(counts.recall),
        "f1": _round(counts.f1),
    }


def _report(scored: metrics.Scored) -> dict[str, object]:
    data = scored.data
    snippets = pool.load(data.suite)
    return {
        "dataset": {
            **dataset.source(),
            "language": data.suite.label,
            "snippets": len(snippets.snippets),
            "missing": len(snippets.missing),
            "dev_queries": len(data.ids("dev")),
            "test_queries": len(data.ids("test")),
        },
        "dates": metrics.dates(data),
        "models": _models(scored),
        "contenders": {
            contender: {
                **_counts(result.counts),
                "unjudged": result.unjudged,
                "seconds_median": _round(result.seconds_median),
                "seconds_p90": _round(result.seconds_p90),
                "cost_per_1000": _round(result.cost_per_1000),
            }
            for contender, result in scored.results.items()
        },
        "sweep": {
            model: [
                {"threshold": threshold, **_counts(counts)}
                for threshold, counts in metrics.sweep(data, model)
            ]
            for model in metrics.MODELS
        },
        "bands": {model: _bands(metrics.bands(data, model)) for model in metrics.MODELS},
        "missed": vars(metrics.missed(data)),
        "judge": _judge(data),
        "repeat": {
            "queries": sorted(data.repeats),
            "changed_share": {
                model: _round(share)
                for model, share in metrics.repeat_changes(data, scored.thresholds).items()
            },
        },
        "tally": {model: vars(metrics.tally(data, model)) for model in metrics.MODELS},
        "spend": {
            model: _round(sum(run[model]["cost"] for run in data.runs.values()))
            for model in metrics.MODELS
        },
    }


def _bands(bands: list[metrics.Band]) -> list[dict[str, object]]:
    return [
        {
            "confidence": f"{band.low:.1f}-{band.high:.1f}",
            "decisions": band.decisions,
            "accuracy": _round(band.accuracy),
        }
        for band in bands
    ]


def _models(scored: metrics.Scored) -> dict[str, object]:
    data = scored.data
    frozen = wording.frozen_wordings()
    templates = wording.frozen_templates()
    models: dict[str, object] = {
        model: {
            "model": metrics.requested(data, model),
            "resolved_model": metrics.resolved(data, model),
            "wording": frozen[model],
            "question": templates[model],
            "threshold": scored.thresholds[model],
        }
        for model in metrics.MODELS
    }
    models["deepseek_providers"] = _providers(data)
    models["grep"] = {"version": next(iter(data.runs.values()))["grep"]["version"]}
    models["agent"] = codex.MODEL
    return models


def _providers(data: metrics.Data) -> dict[str, int]:
    total: Counter[str] = Counter()
    for run in data.runs.values():
        total.update(run["deepseek"]["providers"])
    return dict(total.most_common())


def _judge(data: metrics.Data) -> dict[str, object]:
    found = judge.agreement(data.known.expert, data.known.judge)
    return {
        "model": codex.MODEL,
        "selected_pairs": len(data.selection),
        "judged_pairs": len(data.known.judge),
        "validation_pairs": found.judged,
        "against_experts": _counts(found.judge),
        "expert_against_experts": _counts(found.experts),
    }


def _numbers(scored: metrics.Scored) -> str:
    report = _report(scored)
    data = scored.data
    lines = [
        "# Numbers",
        "",
        (
            f"Generated by `bench score` from `bench/results/`. Test queries: "
            f"{len(data.ids('test'))}; thresholds chosen on the {len(data.ids('dev'))} dev "
            f"queries: jevpipe {scored.thresholds['jevpipe']:.2f}, "
            f"DeepSeek {scored.thresholds['deepseek']:.2f}."
        ),
        "",
        "## Contenders",
        "",
        (
            "| Contender | Found | False hits | Precision | Recall | F1 | Unjudged "
            "| Median s | p90 s | $ / 1k |"
        ),
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for contender in metrics.CONTENDERS:
        result = scored.results[contender]
        counts = result.counts
        lines.append(
            f"| {_NAMES[contender]} | {counts.found:,.0f} | {counts.false_hits:,.0f} | "
            f"{counts.precision:.3f} | {counts.recall:.3f} | {counts.f1:.3f} | "
            f"{result.unjudged} | {result.seconds_median:.2f} | {result.seconds_p90:.2f} | "
            f"{result.cost_per_1000:.4f} |"
        )
    lines += _sweep_lines(data)
    lines += _band_lines(data)
    lines += ["", "## Judge, misses, repeats", "", "```json"]
    lines += [
        _json_line(key, report[key]) for key in ("judge", "missed", "repeat", "tally", "spend")
    ]
    lines += ["```"]
    return "\n".join(lines) + "\n"


def _sweep_lines(data: metrics.Data) -> list[str]:
    lines = [
        "",
        "## Thresholds",
        "",
        (
            "| Threshold | jevpipe P | jevpipe R | jevpipe F1 "
            "| DeepSeek P | DeepSeek R | DeepSeek F1 |"
        ),
        "|---:|---:|---:|---:|---:|---:|---:|",
    ]
    sweeps = [metrics.sweep(data, model) for model in metrics.MODELS]
    for (threshold, jev), (_, other) in zip(*sweeps, strict=True):
        lines.append(
            f"| {threshold:.2f} | {jev.precision:.3f} | {jev.recall:.3f} | {jev.f1:.3f} | "
            f"{other.precision:.3f} | {other.recall:.3f} | {other.f1:.3f} |"
        )
    return lines


def _band_lines(data: metrics.Data) -> list[str]:
    lines = [
        "",
        "## Confidence bands",
        "",
        "Accuracy of the 0.5 decision on labelled pairs, by confidence max(p, 1 - p).",
        "",
        "| Confidence | jevpipe pairs | jevpipe accuracy | DeepSeek pairs | DeepSeek accuracy |",
        "|---|---:|---:|---:|---:|",
    ]
    bands = [metrics.bands(data, model) for model in metrics.MODELS]
    for jev, other in zip(*bands, strict=True):
        lines.append(
            f"| {jev.low:.1f}-{jev.high:.1f} | {jev.decisions:,} | {jev.accuracy:.3f} | "
            f"{other.decisions:,} | {other.accuracy:.3f} |"
        )
    return lines


def _json_line(key: str, value: object) -> str:
    return f'"{key}": {json.dumps(value, ensure_ascii=False)}'
