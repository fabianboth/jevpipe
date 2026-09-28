import json
import shutil
import subprocess
import tempfile
from collections.abc import Callable, Sequence
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

from bench import store
from bench.limits import LimitReachedError, StageError

MODEL = "gpt-6-astra"
_EFFORT = "low"
_EMPTY_FOLDER = store.CACHE / "codex-empty"
_TIMEOUT_SECONDS = 1800
_USAGE_LIMIT_SIGNS = ("usage limit", "quota")


def ask(prompt: str, schema: dict[str, object]) -> object:
    codex = shutil.which("codex")
    if codex is None:
        message = "codex is not on the path"
        raise StageError(message)
    _EMPTY_FOLDER.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as folder:
        schema_file = Path(folder) / "schema.json"
        output_file = Path(folder) / "output.json"
        schema_file.write_text(json.dumps(schema), encoding="utf-8")
        command = [
            codex,
            "exec",
            *("-m", MODEL),
            *("-c", f"model_reasoning_effort={_EFFORT}"),
            *("-s", "read-only"),
            "--ephemeral",
            "--skip-git-repo-check",
            "--ignore-rules",
            *("-C", str(_EMPTY_FOLDER)),
            *("--output-schema", str(schema_file)),
            *("-o", str(output_file)),
            "-",
        ]
        result = subprocess.run(
            command,
            input=prompt,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=_TIMEOUT_SECONDS,
            check=False,
        )
        if result.returncode != 0:
            raise _failure(result.returncode, result.stderr)
        return json.loads(output_file.read_text(encoding="utf-8"))


def _failure(status: int, stderr: str) -> Exception:
    if any(sign in stderr.lower() for sign in _USAGE_LIMIT_SIGNS):
        return LimitReachedError("the Codex usage limit is reached; rerun the stage later")
    return StageError(f"codex exited with {status}: {stderr.strip()[-2000:]}")


def run_all(tasks: Sequence[Callable[[], None]], parallel: int) -> None:
    with ThreadPoolExecutor(max_workers=parallel) as executor:
        futures = [executor.submit(task) for task in tasks]
        try:
            for future in as_completed(futures):
                future.result()
        except LimitReachedError, StageError:
            for future in futures:
                future.cancel()
            raise
