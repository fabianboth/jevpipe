import hashlib
import re
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from pathlib import PurePath
from typing import TypedDict, cast

import httpx

from bench import dataset, store

_URL = re.compile(r"https://github\.com/([^/]+)/([^/]+)/blob/([0-9a-f]+)/(.+?)#L(\d+)(?:-L(\d+))?$")
_POOL_FILE = "pool.json"
_RAW = store.CACHE / "raw"


@dataclass(frozen=True)
class Source:
    raw_url: str
    first: int
    last: int


@dataclass(frozen=True)
class Snippet:
    name: str
    url: str
    sha256: str


@dataclass(frozen=True)
class Pool:
    snippets: tuple[Snippet, ...]
    missing: tuple[str, ...]

    def index_of(self) -> dict[str, int]:
        return {snippet.url: index for index, snippet in enumerate(self.snippets)}


class _SnippetRecord(TypedDict):
    name: str
    url: str
    sha256: str


class _PoolFile(TypedDict):
    dataset: dict[str, str]
    snippets: list[_SnippetRecord]
    missing: list[str]


def index_of_name(name: str) -> int:
    return int(PurePath(name).stem)


@dataclass(frozen=True)
class Location:
    repository: str
    commit: str
    path: str
    first: int
    last: int


def locate(url: str) -> Location | None:
    match = _URL.match(url)
    if match is None:
        return None
    owner, repository, commit, path, first, last = match.groups()
    return Location(f"{owner}/{repository}", commit, path, int(first), int(last or first))


def parse(url: str) -> Source | None:
    location = locate(url)
    if location is None:
        return None
    raw_url = (
        f"https://raw.githubusercontent.com/{location.repository}/{location.commit}/{location.path}"
    )
    return Source(raw_url, location.first, location.last)


def cut(text: str, source: Source) -> str:
    lines = [line.removesuffix("\r") for line in text.split("\n")]
    return "\n".join(lines[source.first - 1 : source.last])


def build(
    urls: Iterable[str], code_of: Callable[[str], str | None], suite: store.Suite
) -> tuple[Pool, list[str]]:
    snippets: list[Snippet] = []
    codes: list[str] = []
    missing: list[str] = []
    for url in sorted(set(urls)):
        code = code_of(url)
        if code is None:
            missing.append(url)
            continue
        digest = hashlib.sha256(code.encode()).hexdigest()
        snippets.append(Snippet(suite.name_of(len(snippets)), url, digest))
        codes.append(code)
    return Pool(tuple(snippets), tuple(missing)), codes


def fetcher(client: httpx.Client) -> Callable[[str], str | None]:
    def code_of(url: str) -> str | None:
        source = parse(url)
        if source is None:
            return None
        text = _fetch_raw(client, source.raw_url)
        return None if text is None else cut(text, source)

    return code_of


def _fetch_raw(client: httpx.Client, raw_url: str) -> str | None:
    cached = _RAW / hashlib.sha256(raw_url.encode()).hexdigest()
    if not cached.is_file():
        response = client.get(raw_url, follow_redirects=True)
        if not response.is_success:
            return None
        cached.parent.mkdir(parents=True, exist_ok=True)
        cached.write_bytes(response.content)
    return cached.read_bytes().decode("utf-8", errors="replace")


def write_files(codes: list[str], suite: store.Suite) -> None:
    suite.pool.mkdir(parents=True, exist_ok=True)
    for stale in suite.pool.glob(f"*{suite.extension}"):
        stale.unlink()
    for index, text in enumerate(codes):
        (suite.pool / suite.name_of(index)).write_text(text, encoding="utf-8", newline="\n")


def save(pool: Pool, suite: store.Suite) -> None:
    data: _PoolFile = {
        "dataset": dataset.source(),
        "snippets": [{"name": s.name, "url": s.url, "sha256": s.sha256} for s in pool.snippets],
        "missing": list(pool.missing),
    }
    store.write_json(suite.results / _POOL_FILE, data)


def load(suite: store.Suite) -> Pool:
    data = cast("_PoolFile", store.read_json(suite.results / _POOL_FILE))
    snippets = tuple(Snippet(s["name"], s["url"], s["sha256"]) for s in data["snippets"])
    return Pool(snippets, tuple(data["missing"]))


def code(suite: store.Suite, index: int) -> str:
    return (suite.pool / suite.name_of(index)).read_text(encoding="utf-8")
