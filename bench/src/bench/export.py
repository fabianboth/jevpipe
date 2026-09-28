import gzip
import io
import json
from collections.abc import Callable, Iterable, Iterator, Mapping
from dataclasses import dataclass
from datetime import date
from pathlib import Path
from typing import Literal, TypedDict

from bench import (
    agreement,
    codex,
    dataset,
    decisions,
    judge,
    labels,
    licenses,
    pool,
    selection,
    store,
)

QUERIES_FILE = "queries.jsonl.gz"
CORPUS_FILE = "corpus.jsonl.gz"
QRELS_FILE = "qrels.jsonl.gz"
CARD_FILE = "README.md"

type RatedBecause = Literal["expert", "pooled", "sample"]


@dataclass(frozen=True)
class Source:
    suite: store.Suite
    queries: dataset.Queries
    snippets: pool.Pool
    known: labels.Labels
    reasons: Mapping[labels.Pair, tuple[str, ...]]
    licenses: licenses.Licenses

    def license_of(self, index: int) -> str | None:
        location = pool.locate(self.snippets.snippets[index].url)
        if location is None:
            return None
        return licenses.reported(self.licenses.by_repository.get(location.repository))

    @property
    def agreement(self) -> agreement.Agreement:
        return agreement.agreement(self.known.expert, self.known.judge)


class QueryRow(TypedDict):
    language: str
    search_id: str
    search: str
    split: str


class CorpusRow(TypedDict):
    id: str
    language: str
    code: str
    url: str
    repository: str
    commit: str
    path: str
    first_line: int
    last_line: int
    license: str | None
    sha256: str


class QrelRow(TypedDict):
    language: str
    search_id: str
    function_id: str
    relevance: float
    relevant: bool
    source: str
    expert_ratings: list[int]
    judge_rating: int | None
    rated_because: RatedBecause


def function_id(suite: store.Suite, index: int) -> str:
    return f"{suite.language}/{index:04d}"


def query_rows(source: Source) -> Iterator[QueryRow]:
    for query in source.queries.queries:
        yield {
            "language": source.suite.language,
            "search_id": query.id,
            "search": query.text,
            "split": "tuning" if query.split == "dev" else "test",
        }


def corpus_rows(source: Source, code_of: Callable[[int], str]) -> Iterator[CorpusRow]:
    for index, snippet in enumerate(source.snippets.snippets):
        location = pool.locate(snippet.url)
        if location is None:
            continue
        yield {
            "id": function_id(source.suite, index),
            "language": source.suite.language,
            "code": code_of(index),
            "url": snippet.url,
            "repository": location.repository,
            "commit": location.commit,
            "path": location.path,
            "first_line": location.first,
            "last_line": location.last,
            "license": source.license_of(index),
            "sha256": snippet.sha256,
        }


def qrel_rows(source: Source) -> Iterator[QrelRow]:
    rated = sorted({*source.known.expert, *source.known.judge})
    for pair in rated:
        query_id, index = pair
        rating = source.known.rating(pair)
        relevant = source.known.relevant(pair)
        if rating is None or relevant is None:
            continue
        expert = source.known.expert.get(pair, ())
        yield {
            "language": source.suite.language,
            "search_id": query_id,
            "function_id": function_id(source.suite, index),
            "relevance": round(rating, 4),
            "relevant": relevant,
            "source": "expert" if expert else "judge",
            "expert_ratings": list(expert),
            "judge_rating": source.known.judge.get(pair),
            "rated_because": _rated_because(pair, source),
        }


def _rated_because(pair: labels.Pair, source: Source) -> RatedBecause:
    if pair in source.known.expert:
        return "expert"
    if "hit" in source.reasons.get(pair, ()):
        return "pooled"
    return "sample"


def load(suite: store.Suite) -> Source:
    reasons = {(item.query, item.snippet): item.reasons for item in selection.selected(suite)}
    return Source(
        suite,
        dataset.load(suite),
        pool.load(suite),
        selection.labels_of(suite),
        reasons,
        licenses.load(),
    )


def export() -> None:
    sources = [load(suite) for suite in store.SUITES.values() if selection.finished(suite)]
    folder = store.CACHE / "export"
    folder.mkdir(parents=True, exist_ok=True)
    counts = {
        QUERIES_FILE: _write_lines(
            folder / QUERIES_FILE, (row for s in sources for row in query_rows(s))
        ),
        CORPUS_FILE: _write_lines(
            folder / CORPUS_FILE,
            (row for s in sources for row in corpus_rows(s, _code_reader(s.suite))),
        ),
        QRELS_FILE: _write_lines(
            folder / QRELS_FILE, (row for s in sources for row in qrel_rows(s))
        ),
    }
    (folder / CARD_FILE).write_text(card(sources), encoding="utf-8", newline="\n")
    print("export: " + ", ".join(f"{count:,} rows in {name}" for name, count in counts.items()))


def _code_reader(suite: store.Suite) -> Callable[[int], str]:
    return lambda index: pool.code(suite, index)


def _write_lines(path: Path, rows: Iterable[object]) -> int:
    count = 0
    with (
        path.open("wb") as raw,
        gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as compressed,
        io.TextIOWrapper(compressed, encoding="utf-8", newline="\n") as text,
    ):
        for row in rows:
            text.write(json.dumps(row, ensure_ascii=False) + "\n")
            count += 1
    return count


def card(sources: list[Source]) -> str:
    filled = {
        "table": "\n".join(_card_row(source) for source in sources),
        "checks": "\n".join(_check_row(source) for source in sources),
        "notes": " ".join(
            [_experts_line(sources)]
            + [_missed_line(source) for source in sources if not source.agreement.passed]
        ).strip(),
        "judge_model": codex.LABEL,
        "batch": str(selection.BATCH),
        "shown": f"{judge.SHOWN_CHARACTERS:,}",
        "gate": f"{agreement.GATE_F1}",
        "lowest": f"{decisions.LOWEST_THRESHOLD}",
        "dev": str(dataset.DEV_QUERIES),
        "looked_up": _month(sources[0].licenses.looked_up),
    }
    text = _CARD
    for key, value in filled.items():
        text = text.replace(f"{{{key}}}", value)
    return text


def _month(day: str) -> str:
    return f"{date.fromisoformat(day):%B %Y}" if day else "the time of export"


def _check_row(source: Source) -> str:
    found = source.agreement
    same = found.judge.same_share
    cells = [
        source.suite.label,
        f"{found.judge.pairs:,}",
        "-" if same is None else f"{same:.0%}",
        f"{found.judge.counts.f1:.3f}",
        "passed" if found.passed else "**missed**",
    ]
    return "| " + " | ".join(cells) + " |"


def _experts_line(sources: list[Source]) -> str:
    lines = [
        f"Where two experts rated the same pair ({experts.pairs:,} {source.suite.label} pairs), "
        f"they agree on {experts.same_share or 0.0:.0%} of verdicts, F1 {experts.counts.f1:.3f}."
        for source in sources
        if (experts := source.agreement.experts) is not None
    ]
    return " ".join(lines)


def _missed_line(source: Source) -> str:
    judge_counts = source.agreement.judge.counts
    called = judge_counts.found + judge_counts.false_hits
    more = (
        f", and calls more pairs relevant ({called:,.0f}) than its experts did "
        f"({judge_counts.known_relevant:,.0f})"
        if called > judge_counts.known_relevant
        else ""
    )
    return (
        f"In {source.suite.label} the judge missed the gate on only "
        f"{judge_counts.known_relevant:,.0f} relevant expert pairs{more}; use its ratings there "
        "with that in mind."
    )


def _card_row(source: Source) -> str:
    qrels = list(qrel_rows(source))
    functions = len(source.snippets.snippets)
    experts = sum(row["source"] == "expert" for row in qrels)
    relevant = sum(row["relevant"] for row in qrels)
    return (
        f"| {source.suite.label} | {len(source.queries.queries)} | {functions:,} | "
        f"{experts:,} | {len(qrels) - experts:,} | {relevant:,} |"
    )


_CARD = """---
license: other
license_name: per-function-and-mit
pretty_name: CodeSearchNet Challenge, extended
language:
- en
- code
task_categories:
- text-retrieval
tags:
- code-search
- relevance
- llm-as-a-judge
size_categories:
- 10K<n<100K
configs:
- config_name: queries
  data_files: queries.jsonl.gz
- config_name: corpus
  data_files: corpus.jsonl.gz
- config_name: qrels
  data_files: qrels.jsonl.gz
  default: true
---

# CodeSearchNet Challenge, extended: every search over every function

The [CodeSearchNet Challenge](https://github.com/github/CodeSearchNet) (Husain et al., 2019) has
99 natural-language code searches, and experts rated a few candidate functions for each. This
dataset treats every rated function of a language as one codebase and searches all of it: for each
search, every function in its language is a candidate. The experts' ratings are kept, and the
pairs they never rated but a search tool returned were rated on the same scale by a judge model,
validated against the experts first. Random samples of the pairs only a loose any-keyword grep
returned, and of the pairs nothing returned, were rated too.

| Language | Searches | Functions | Rated by experts | Rated by the judge | Relevant |
|---|---:|---:|---:|---:|---:|
{table}

It was made for the [jevpipe code search benchmark](https://github.com/fabianboth/jevpipe/tree/main/bench),
which describes the method, the judge's validation and the results in full.

## Configurations

- `queries`: `language`, `search_id`, `search` (the text), `split` (`tuning` for the {dev} Python
  searches used to tune the benchmark's tools, `test` for all others).
- `corpus`: one row per function: `id`, `language`, `code` (the rated lines at the rated commit),
  `url`, `repository`, `commit`, `path`, `first_line`, `last_line`, `license` (the repository's
  SPDX license id as GitHub reported it in {looked_up}, `NOASSERTION` or `null`) and `sha256`
  of `code`.
- `qrels`: one row per rated search-function pair: `relevance` (the experts' mean rating, else the
  judge's rating, 0 to 3), `relevant` (`relevance` of 2 or more), `source` (`expert` or `judge`),
  `expert_ratings`, `judge_rating` (also given for expert-rated pairs, to check the judge), and
  `rated_because` (`expert`, `pooled`: a search tool returned it, or `sample`: one of the random
  samples above).

## How the judge rated

{judge_model} through the Codex CLI, low reasoning effort, no tools, in batches of {batch} pairs
mixed across searches, seeing an opaque id, the search and the code (cut at {shown} characters),
never which tool returned the pair. The prompt gave the Challenge's 0-3 scale and its description of
each level. Before any of its ratings were used, it rated every expert-rated pair of the language;
the gate, set in advance, was an F1 of {gate} on "relevant" against the experts:

| Language | Expert-rated pairs | Same verdict as the experts | F1 | Gate |
|---|---:|---:|---:|---|
{checks}

{notes}

Pooled pairs came from four searches: grep with a pattern a coding agent wrote per search, grep
for all of the search's keywords, jevpipe with a probability of {lowest} or more, and DeepSeek V4.1
Flash answering yes or with a probability of {lowest} or more.

## Use it with care

- **Unrated is unknown, not irrelevant.** The judged pairs were found by the three tools above; a
  new tool will return relevant functions nobody rated. Treat pairs missing from `qrels` as
  unjudged, use a metric that allows for that, or rate the new pairs the same way.
- **Judge ratings are model output**, checked against the experts but not expert ratings.
- **Relevance is from 2019 searches** on public code that may be in any model's training data.

## Licenses

The functions come from the CodeSearchNet corpus, which kept only projects whose license allows
redistributing parts of them. Each function in `corpus` stays under its repository's license,
named in `license` and traceable through `url`. `NOASSERTION` means the repository has a license
file that GitHub cannot match to a standard license; read it in the repository. `null` means GitHub
finds no license in the repository today; CodeSearchNet selected the project in 2019 because its
license then permitted redistribution, and that license still covers the code at the rated commit
that `url` points to. The Challenge's ratings are from its MIT-licensed repository. The searches'
split, the judge ratings and the dataset's structure are released under the MIT license.

## Citation

Husain, H., Wu, H.-H., Gazit, T., Allamanis, M., & Brockschmidt, M. (2019). CodeSearchNet
Challenge: Evaluating the State of Semantic Code Search. arXiv:1909.09436.
"""
