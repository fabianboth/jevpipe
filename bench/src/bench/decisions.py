from dataclasses import dataclass

from bench.contenders import Contender
from bench.records import Decisions, GrepRecord

LOWEST_THRESHOLD = 0.3
THRESHOLDS = tuple(round(LOWEST_THRESHOLD + 0.05 * step, 2) for step in range(13))
DEFAULT_THRESHOLD = 0.5


@dataclass(frozen=True)
class Setting:
    contender: Contender
    threshold: float

    def flags(self, run: Decisions) -> frozenset[int]:
        match self.contender:
            case "jevpipe":
                probability = run["jevpipe"]["probability"]
                return frozenset(
                    i for i, p in enumerate(probability) if says_yes(p, self.threshold, answer=None)
                )
            case "deepseek":
                deepseek = run["deepseek"]
                answers = zip(deepseek["probability"], deepseek["answer"], strict=True)
                return frozenset(
                    i
                    for i, (p, answer) in enumerate(answers)
                    if says_yes(p, self.threshold, answer=answer)
                )
            case "grep-any":
                return frozenset(_grep(run)["any"])
            case "grep-all":
                return frozenset(_grep(run)["all"])
            case "grep-agent":
                return frozenset(_grep(run)["agent"])


def says_yes(probability: float | None, threshold: float, *, answer: bool | None) -> bool:
    return probability >= threshold if probability is not None else answer is True


def _grep(run: Decisions) -> GrepRecord:
    if "grep" not in run:
        message = f"the run of {run['query']} holds no grep results"
        raise KeyError(message)
    return run["grep"]
