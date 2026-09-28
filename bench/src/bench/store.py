import json
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import cast

RESULTS = Path(__file__).resolve().parents[2] / "results"
CACHE = RESULTS.parent / ".cache"
_INDENT = "  "
_PRIMARY = "python"


@dataclass(frozen=True)
class Suite:
    language: str
    label: str
    extension: str

    @property
    def primary(self) -> bool:
        return self.language == _PRIMARY

    @property
    def results(self) -> Path:
        return RESULTS if self.primary else RESULTS / self.language

    @property
    def pool(self) -> Path:
        return CACHE / "pool" if self.primary else CACHE / self.language / "pool"

    @property
    def runs(self) -> Path:
        return self.results / "runs"

    @property
    def repeats(self) -> Path:
        return self.results / "repeat"

    @property
    def judged(self) -> Path:
        return self.results / "judge"

    def name_of(self, index: int) -> str:
        return f"{index:04d}{self.extension}"


SUITES = {
    suite.language: suite
    for suite in (
        Suite("python", "Python", ".py"),
        Suite("java", "Java", ".java"),
        Suite("javascript", "JavaScript", ".js"),
        Suite("php", "PHP", ".php"),
        Suite("ruby", "Ruby", ".rb"),
        Suite("go", "Go", ".go"),
    )
}
PYTHON = SUITES[_PRIMARY]


def now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def write_json(path: Path, value: object) -> None:
    write_bytes(path, (_encode(value, 0) + "\n").encode())


def write_bytes(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f"{path.name}.tmp")
    temporary.write_bytes(content)
    temporary.replace(path)


def _encode(value: object, level: int) -> str:
    inner = _INDENT * (level + 1)
    outer = _INDENT * level
    if isinstance(value, dict) and value:
        entries = cast("dict[str, object]", value)
        lines = [
            f"{inner}{json.dumps(key, ensure_ascii=False)}: {_encode(item, level + 1)}"
            for key, item in entries.items()
        ]
        return "{\n" + ",\n".join(lines) + f"\n{outer}}}"
    if isinstance(value, list) and not _scalars(cast("list[object]", value)):
        items = cast("list[object]", value)
        lines = [f"{inner}{_encode(item, level + 1)}" for item in items]
        return "[\n" + ",\n".join(lines) + f"\n{outer}]"
    return json.dumps(value, ensure_ascii=False)


def _scalars(items: list[object]) -> bool:
    return all(not isinstance(item, dict | list) for item in items)


def read_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))
