import shutil
import subprocess
from concurrent.futures import ThreadPoolExecutor
from typing import cast

from bench import pool, store
from bench.limits import StageError

_FILE = store.RESULTS / "licenses.json"
_LOOKUPS = 8
_UNKNOWN = frozenset({"null", ""})


def reported(license_id: str | None) -> str | None:
    return None if license_id is None or license_id in _UNKNOWN else license_id


def load() -> dict[str, str | None]:
    if not store.exists(_FILE):
        return {}
    return cast("dict[str, str | None]", store.read_json(_FILE))


def repositories() -> list[str]:
    found: set[str] = set()
    for suite in store.SUITES.values():
        if store.exists(suite.results / "pool.json"):
            for snippet in pool.load(suite).snippets:
                location = pool.locate(snippet.url)
                if location is not None:
                    found.add(location.repository)
    return sorted(found)


def look_up() -> None:
    known = load()
    todo = [repository for repository in repositories() if repository not in known]
    with ThreadPoolExecutor(_LOOKUPS) as executor:
        for repository, license_id in zip(todo, executor.map(_license_of, todo), strict=True):
            known[repository] = license_id
    store.write_json(_FILE, dict(sorted(known.items())))
    missing = sum(reported(license_id) is None for license_id in known.values())
    print(f"licenses: {len(todo)} looked up, {len(known)} known, {missing} without a license today")


def _license_of(repository: str) -> str | None:
    gh = shutil.which("gh")
    if gh is None:
        message = "the GitHub CLI (gh) is not on the path"
        raise StageError(message)
    result = subprocess.run(
        [gh, "api", f"repos/{repository}", "--jq", ".license.spdx_id"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    return result.stdout.strip() if result.returncode == 0 else None
