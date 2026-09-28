import hashlib
from pathlib import Path

import httpx

from bench import store
from bench.limits import StageError

_GONE = frozenset({404, 410})


def cached(client: httpx.Client, url: str) -> bytes | None:
    path = _cache_file(url)
    if not path.is_file():
        response = client.get(url, follow_redirects=True)
        if response.status_code in _GONE:
            return None
        if not response.is_success:
            message = f"downloading {url} failed with {response.status_code}; run the stage again"
            raise StageError(message)
        store.write_bytes(path, response.content)
    return path.read_bytes()


def _cache_file(url: str) -> Path:
    return store.CACHE / "raw" / hashlib.sha256(url.encode()).hexdigest()
