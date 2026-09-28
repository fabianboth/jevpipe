from collections import Counter
from collections.abc import Sequence
from dataclasses import dataclass

from bench import bootstrap, codex, dataset, languages, metrics, pool, wording
from bench.agreement import Agreement, Verdicts
from bench.contenders import CONTENDERS, MODELS, SHOWN, Contender, Model
from bench.counts import Counts

_DIGITS = 4


@dataclass(frozen=True)
class Summary:
    scored: metrics.Scored
    sweeps: dict[Model, list[tuple[float, Counts]]]
    bands: dict[Model, list[metrics.Band]]
    missed: metrics.Missed
    per_1000: dict[Contender, languages.PerThousand]

    @property
    def estimated_relevant(self) -> float:
        return self.scored.results["grep-any"].counts.known_relevant + self.missed.estimate


def summarize(scored: metrics.Scored) -> Summary:
    data = scored.data
    return Summary(
        scored,
        {model: metrics.sweep(data, model) for model in MODELS},
        {model: metrics.bands(data, model) for model in MODELS},
        metrics.missed(data),
        {c: languages.median_per_1000([scored], c) for c in CONTENDERS},
    )


def rounded(value: float) -> float:
    return round(value, _DIGITS)


def counts_of(counts: Counts) -> dict[str, float]:
    return {
        "found": rounded(counts.found),
        "false_hits": rounded(counts.false_hits),
        "known_relevant": rounded(counts.known_relevant),
        "precision": rounded(counts.precision),
        "recall": rounded(counts.recall),
        "f1": rounded(counts.f1),
    }


def language(summary: Summary) -> dict[str, object]:
    scored = summary.scored
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
                **counts_of(result.counts),
                "estimated": contender == "grep-any",
                "unjudged": result.unjudged,
                "seconds_median": rounded(result.seconds_median),
                "seconds_p90": rounded(result.seconds_p90),
                "cost_per_1000": rounded(result.cost_per_1000),
                "median_per_1000": _per_1000(summary.per_1000[contender]),
            }
            for contender, result in scored.results.items()
        },
        "sweep": {model: _sweep(summary.sweeps[model]) for model in MODELS},
        "bands": {model: bands_of(summary.bands[model]) for model in MODELS},
        "missed": _missed(summary),
        "judge": _judge(scored),
        "repeat": _repeat(scored),
        "tally": {model: vars(metrics.tally(data, model)) for model in MODELS},
        "spend": {
            model: rounded(sum(run[model]["cost"] for run in data.runs.values()))
            for model in MODELS
        },
    }


def _sweep(points: list[tuple[float, Counts]]) -> list[dict[str, float]]:
    return [{"threshold": threshold, **counts_of(counts)} for threshold, counts in points]


def bands_of(bands: list[metrics.Band]) -> list[dict[str, object]]:
    return [
        {
            "confidence": f"{band.low:.1f}-{band.high:.1f}",
            "decisions": band.decisions,
            "accuracy": None if band.accuracy is None else rounded(band.accuracy),
        }
        for band in bands
    ]


def _per_1000(found: languages.PerThousand) -> dict[str, float]:
    return {"seconds": rounded(found.seconds), "cost": rounded(found.cost)}


def _missed(summary: Summary) -> dict[str, object]:
    missed = summary.missed
    estimated = summary.estimated_relevant
    results = summary.scored.results
    return {
        "sampled": missed.sampled,
        "relevant_in_sample": missed.relevant_in_sample,
        "unflagged": missed.unflagged,
        "estimate": rounded(missed.estimate),
        "estimated_relevant": rounded(estimated),
        "recall_against_estimate": {
            contender: rounded(results[contender].counts.found / estimated) for contender in SHOWN
        },
    }


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
        for model in MODELS
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


def _judge(scored: metrics.Scored) -> dict[str, object]:
    data = scored.data
    found = scored.agreement
    return {
        "model": codex.MODEL,
        "selected_pairs": len(data.selection),
        "judged_pairs": len(data.known.judge),
        "validation_pairs": found.judge.pairs,
        "against_experts": verdicts_of(found.judge),
        "pairs_rated_twice": found.twice,
        "on_pairs_rated_twice": _maybe(found.judge_on_twice),
        "expert_against_experts": _maybe(found.experts),
        "passed": found.passed,
    }


def verdicts_of(found: Verdicts) -> dict[str, object]:
    same = found.same_share
    return {
        "pairs": found.pairs,
        "same_verdict": None if same is None else rounded(same),
        **counts_of(found.counts),
    }


def _maybe(found: Verdicts | None) -> dict[str, object] | None:
    return None if found is None else verdicts_of(found)


def _repeat(scored: metrics.Scored) -> dict[str, object]:
    changes = metrics.repeat_changes(scored)
    return {
        "queries": sorted(scored.data.repeats),
        "changed_share": None
        if changes is None
        else {model: rounded(share) for model, share in changes.items()},
    }


def pooled(group: Sequence[metrics.Scored]) -> dict[str, object]:
    return {
        "languages": [scored.data.suite.language for scored in group],
        "test_searches": languages.searches(group),
        "contenders": {c: counts_of(languages.pooled(group, c)) for c in SHOWN},
        "gaps": {
            f"{first} - {second}": _gap(languages.gap(group, (first, second)))
            for first, second in languages.COMPARISONS
        },
        "median_per_1000": {
            model: _per_1000(languages.median_per_1000(group, model)) for model in MODELS
        },
        "sweep": {model: _sweep(languages.sweep(group, model)) for model in MODELS},
        "bands": {model: bands_of(languages.bands(group, model)) for model in MODELS},
    }


def _gap(found: bootstrap.Gap) -> dict[str, float]:
    return {
        "f1_gap": rounded(found.observed),
        "low": rounded(found.low),
        "high": rounded(found.high),
        "above_zero": rounded(found.above_zero),
    }


def all_languages(
    everything: list[metrics.Scored], passed: list[metrics.Scored]
) -> dict[str, object]:
    return {
        "thresholds": everything[0].thresholds,
        "languages": {scored.data.suite.language: _one(scored) for scored in everything},
        "python": pooled(everything[:1]),
        "pooled": pooled(passed),
        "pooled_with_every_language": pooled(everything),
    }


def _one(scored: metrics.Scored) -> dict[str, object]:
    found: Agreement = scored.agreement
    return {
        "test_searches": len(scored.data.ids("test")),
        "pool": scored.data.pool_size,
        "judge": {
            "f1": rounded(found.judge.counts.f1),
            "same_verdict": verdicts_of(found.judge)["same_verdict"],
            "passed": found.passed,
            "pairs_rated_twice": found.twice,
        },
        "median_per_1000": {
            model: _per_1000(languages.median_per_1000([scored], model)) for model in MODELS
        },
        "contenders": {c: counts_of(scored.results[c].counts) for c in SHOWN},
    }
