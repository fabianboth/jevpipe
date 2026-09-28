from collections.abc import Sequence
from typing import TypedDict, cast

from bench import codex, dataset, grep, store
from bench.limits import StageError

_BATCH = 20
_PATTERNS_FILE = "patterns.json"
_INSTRUCTIONS = """You are a coding agent. You are about to search a large {language} codebase with
ripgrep for code that does what a developer asked for. For each search below, write the one
ripgrep regular expression you would run to find the functions that do it: think of synonyms,
abbreviations and the identifier forms (snake_case, CamelCase) such code would use. The search runs
case-insensitive over whole files, as `rg --ignore-case --files-with-matches -e <pattern>`, so the
pattern should find the right functions without matching most of the codebase. Use ripgrep's regex
syntax (Rust regex: no lookaround, no backreferences). You have not seen the codebase; do not run
commands. Return one pattern per id."""
_SCHEMA: dict[str, object] = {
    "type": "object",
    "additionalProperties": False,
    "required": ["patterns"],
    "properties": {
        "patterns": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["id", "pattern"],
                "properties": {"id": {"type": "string"}, "pattern": {"type": "string"}},
            },
        }
    },
}


class _Written(TypedDict):
    id: str
    pattern: str


class _Answer(TypedDict):
    patterns: list[_Written]


class Pattern(TypedDict):
    pattern: str
    retried: bool


class _PatternsFile(TypedDict):
    model: str
    written: str
    patterns: dict[str, Pattern]


def write_patterns(suite: store.Suite, parallel: int) -> None:
    path = suite.results / _PATTERNS_FILE
    if path.is_file():
        print(f"patterns: {suite.label} already written")
        return
    queries = dataset.load(suite).queries
    batches = [queries[start : start + _BATCH] for start in range(0, len(queries), _BATCH)]
    written: dict[str, str] = {}

    def task(batch: Sequence[dataset.Query]) -> None:
        written.update(_ask(batch, suite, ""))
        print(f"patterns: batch of {len(batch)} written")

    codex.run_all([lambda batch=batch: task(batch) for batch in batches], parallel)
    patterns: dict[str, Pattern] = {}
    for query in queries:
        first = written.get(query.id)
        if first is not None and grep.compiles(first):
            patterns[query.id] = {"pattern": first, "retried": False}
        else:
            patterns[query.id] = {"pattern": _retry(query, suite, first), "retried": True}
    data: _PatternsFile = {
        "model": codex.MODEL,
        "written": store.now(),
        "patterns": patterns,
    }
    store.write_json(path, data)
    retried = sum(pattern["retried"] for pattern in patterns.values())
    print(f"patterns: {len(patterns)} written, {retried} asked again")


def _ask(queries: Sequence[dataset.Query], suite: store.Suite, note: str) -> dict[str, str]:
    searches = "\n\n".join(f"id: {query.id}\nsearch: {query.text}" for query in queries)
    prompt = f"{_INSTRUCTIONS.format(language=suite.label)}{note}\n\n{searches}"
    answer = cast("_Answer", codex.ask(prompt, _SCHEMA))
    return {written["id"]: written["pattern"] for written in answer["patterns"]}


def _retry(query: dataset.Query, suite: store.Suite, rejected: str | None) -> str:
    note = (
        "\n\nThe pattern you wrote before did not compile in ripgrep: " + rejected
        if rejected
        else ""
    )
    pattern = _ask([query], suite, note).get(query.id)
    if pattern is None or not grep.compiles(pattern):
        message = f"no ripgrep pattern for {query.id} compiled after asking twice"
        raise StageError(message)
    return pattern


def load(suite: store.Suite) -> dict[str, str]:
    data = cast("_PatternsFile", store.read_json(suite.results / _PATTERNS_FILE))
    return {query_id: written["pattern"] for query_id, written in data["patterns"].items()}
