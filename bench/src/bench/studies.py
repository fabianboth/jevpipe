import statistics
from dataclasses import dataclass
from typing import TypedDict, cast

from bench import store

_AY_AUTOMATE = "ayautomate-2026-09.json"
_JEV = "Jev 1.13"


class _Model(TypedDict):
    accuracy: list[float]
    cost_per_1000: list[float]
    median_seconds: float


class _StudyFile(TypedDict):
    source: str
    published: str
    tasks: list[str]
    decisions: int
    models: dict[str, _Model]


@dataclass(frozen=True)
class Result:
    name: str
    accuracy: float
    cost_per_1000: float
    seconds: float


@dataclass(frozen=True)
class Study:
    source: str
    published: str
    tasks: tuple[str, ...]
    decisions: int
    results: tuple[Result, ...]

    @property
    def jev(self) -> Result:
        return next(result for result in self.results if result.name == _JEV)

    @property
    def others(self) -> tuple[Result, ...]:
        return tuple(result for result in self.results if result.name != _JEV)


def ay_automate() -> Study:
    data = cast("_StudyFile", store.read_json(store.RESULTS.parent / "data" / _AY_AUTOMATE))
    results = tuple(
        Result(
            name,
            statistics.mean(model["accuracy"]),
            statistics.mean(model["cost_per_1000"]),
            model["median_seconds"],
        )
        for name, model in data["models"].items()
    )
    return Study(
        data["source"], data["published"], tuple(data["tasks"]), data["decisions"], results
    )
