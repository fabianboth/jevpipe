import argparse
import io
import sys
from collections.abc import Callable
from dataclasses import dataclass
from typing import cast

from bench import export, judge, licenses, patterns, runs, score, store, wording
from bench.limits import LimitReachedError, StageError
from bench.prepare import prepare

_LIMIT_EXIT = 3
_PER_LANGUAGE = frozenset({"prepare", "patterns", "run", "judge"})


@dataclass(frozen=True)
class Options:
    suite: store.Suite
    queries: tuple[str, ...] | None
    parallel: int
    continue_judging: bool
    retry_failed: bool


def _prepare(options: Options) -> None:
    prepare(options.suite)


def _patterns(options: Options) -> None:
    patterns.write_patterns(options.suite, options.parallel)


def _wordings(_: Options) -> None:
    wording.try_wordings()


def _run(options: Options) -> None:
    if options.retry_failed:
        runs.retry_failed(options.suite)
    else:
        runs.run_queries(options.suite, options.queries)


def _repeat(options: Options) -> None:
    runs.repeat_queries(options.queries)


def _judge(options: Options) -> None:
    judge.judge_pairs(options.suite, options.parallel, continue_judging=options.continue_judging)


def _score(_: Options) -> None:
    score.score()


def _licenses(_: Options) -> None:
    licenses.look_up()


def _export(_: Options) -> None:
    export.export()


_STAGES: dict[str, tuple[str, Callable[[Options], None]]] = {
    "prepare": ("download the ratings and snippets, build the pool and the split", _prepare),
    "patterns": ("ask the agent for one ripgrep pattern per query", _patterns),
    "wordings": ("try the question wordings on the dev queries and freeze the best", _wordings),
    "run": ("search every query with grep, jevpipe and DeepSeek over the pool", _run),
    "repeat": ("run jevpipe and DeepSeek again on the repeat queries", _repeat),
    "judge": ("judge the selected pairs with the agent, validation first", _judge),
    "score": ("compute the metrics, the numbers and the chart, offline", _score),
    "licenses": ("look up each source repository's license on GitHub", _licenses),
    "export": ("write queries, corpus, qrels and the dataset card for publishing", _export),
}


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="bench", description="jevpipe against grep and DeepSeek on the CodeSearchNet Challenge"
    )
    stages = parser.add_subparsers(dest="stage", required=True)
    for name, (help_text, _) in _STAGES.items():
        stage = stages.add_parser(name, help=help_text, description=help_text)
        if name in _PER_LANGUAGE:
            stage.add_argument(
                "--language",
                choices=sorted(store.SUITES),
                default=store.PYTHON.language,
                help="the CodeSearchNet language to work on (default: python)",
            )
        if name in {"run", "repeat"}:
            stage.add_argument("--queries", help="only these query ids, comma-separated")
        if name == "run":
            stage.add_argument(
                "--retry-failed",
                action="store_true",
                help="ask jevpipe again about the records a stored run left unanswered",
            )
        if name in {"patterns", "judge"}:
            stage.add_argument("--parallel", type=int, default=4, help="agent calls at once")
        if name == "judge":
            stage.add_argument(
                "--continue-judging",
                action="store_true",
                help="go on after a validation F1 below the gate",
            )
    return parser


def _options(namespace: argparse.Namespace) -> Options:
    language: str = getattr(namespace, "language", store.PYTHON.language)
    queries: str | None = getattr(namespace, "queries", None)
    parallel: int = getattr(namespace, "parallel", 4)
    continue_judging: bool = getattr(namespace, "continue_judging", False)
    retry_failed: bool = getattr(namespace, "retry_failed", False)
    return Options(
        store.SUITES[language],
        tuple(queries.split(",")) if queries else None,
        parallel,
        continue_judging,
        retry_failed,
    )


def main() -> None:
    stdout = cast("io.TextIOWrapper[io.BufferedWriter]", sys.stdout)
    stdout.reconfigure(line_buffering=True)
    namespace = _parser().parse_args()
    stage: str = namespace.stage
    _, run = _STAGES[stage]
    try:
        run(_options(namespace))
    except LimitReachedError as error:
        print(f"bench {stage}: stopped by a limit: {error}", file=sys.stderr)
        sys.exit(_LIMIT_EXIT)
    except StageError as error:
        print(f"bench {stage}: {error}", file=sys.stderr)
        sys.exit(1)
