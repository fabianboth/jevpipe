from typing import NotRequired, TypedDict


class GrepRecord(TypedDict):
    version: str
    any: list[int]
    all: list[int]
    agent: list[int]
    seconds: dict[str, float]


class Retried(TypedDict):
    records: int
    answered: int
    version: NotRequired[str]
    cost: float
    wall_seconds: float


class JevRecord(TypedDict):
    version: NotRequired[str]
    retried: NotRequired[Retried]
    model: str
    resolved_model: str
    started: str
    first: bool
    wall_seconds: float
    cost: float
    failed: int
    skipped: int
    probability: list[float | None]


class DeepSeekRecord(TypedDict):
    retried: NotRequired[Retried]
    model: str
    resolved_model: str
    started: str
    first: bool
    wall_seconds: float
    cost: float
    failed: int
    providers: dict[str, int]
    answer: list[bool | None]
    probability: list[float | None]


class RepeatFile(TypedDict):
    query: str
    jevpipe: JevRecord
    deepseek: DeepSeekRecord


class RunFile(RepeatFile):
    grep: GrepRecord


type Decisions = RepeatFile | RunFile
