import subprocess
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path
from typing import TypedDict, cast

from bench import pool, store, tools
from bench.limits import StageError

_LOOKUPS = 8
_UNKNOWN = frozenset({"null", ""})
_NOT_FOUND = "HTTP 404"


@dataclass(frozen=True)
class Licenses:
    looked_up: str
    by_repository: dict[str, str | None]


class _LicensesFile(TypedDict):
    looked_up: str
    licenses: dict[str, str | None]


def _file() -> Path:
    return store.RESULTS / "licenses.json"


def reported(license_id: str | None) -> str | None:
    return None if license_id is None or license_id in _UNKNOWN else license_id


def load() -> Licenses:
    path = _file()
    if not path.is_file():
        return Licenses("", {})
    data = cast("_LicensesFile", store.read_json(path))
    return Licenses(data["looked_up"], data["licenses"])


def repositories() -> list[str]:
    found: set[str] = set()
    for suite in store.SUITES.values():
        if pool.stored(suite):
            for snippet in pool.load(suite).snippets:
                location = pool.locate(snippet.url)
                if location is not None:
                    found.add(location.repository)
    return sorted(found)


def look_up() -> None:
    known = load().by_repository
    todo = [repository for repository in repositories() if repository not in known]
    gh = tools.find("gh")

    def license_of(repository: str) -> str | None:
        return _license_of(gh, repository)

    with ThreadPoolExecutor(_LOOKUPS) as executor:
        for repository, license_id in zip(todo, executor.map(license_of, todo), strict=True):
            known[repository] = license_id
    data: _LicensesFile = {"looked_up": store.now()[:10], "licenses": dict(sorted(known.items()))}
    store.write_json(_file(), data)
    missing = sum(reported(license_id) is None for license_id in known.values())
    print(f"licenses: {len(todo)} looked up, {len(known)} known, {missing} without a license today")


def _license_of(gh: str, repository: str) -> str | None:
    result = subprocess.run(
        [gh, "api", f"repos/{repository}", "--jq", ".license.spdx_id"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if result.returncode == 0:
        return result.stdout.strip()
    if _NOT_FOUND in result.stderr:
        return None
    message = f"looking up {repository} failed: {result.stderr.strip()}; run the stage again"
    raise StageError(message)
