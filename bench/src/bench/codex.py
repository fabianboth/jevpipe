import json
import subprocess
import tempfile
from collections.abc import Callable, Sequence
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

from bench import store, tools
from bench.limits import LimitReachedError, StageError

MODEL = "gpt-6-astra"
LABEL = "GPT-6 Astra"
_EFFORT = "low"
_TIMEOUT_SECONDS = 1800
_USAGE_LIMIT_SIGNS = ("usage limit", "quota")
_ERROR_TAIL_LINES = 5


def ask(prompt: str, schema: dict[str, object]) -> object:
    codex = tools.find("codex")
    empty_folder = store.CACHE / "codex-empty"
    empty_folder.mkdir(parents=True, exist_ok=True)
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
            *("-C", str(empty_folder)),
            *("--output-schema", str(schema_file)),
            *("-o", str(output_file)),
            "-",
        ]
        try:
            result = subprocess.run(
                command,
                input=prompt,
                capture_output=True,
                text=True,
                encoding="utf-8",
                timeout=_TIMEOUT_SECONDS,
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            message = f"codex did not answer within {_TIMEOUT_SECONDS} s"
            raise StageError(message) from error
        if result.returncode != 0:
            raise failure(result.returncode, result.stderr)
        return json.loads(output_file.read_text(encoding="utf-8"))


def failure(status: int, stderr: str) -> Exception:
    tail = "\n".join(stderr.strip().splitlines()[-_ERROR_TAIL_LINES:])
    if any(sign in tail.lower() for sign in _USAGE_LIMIT_SIGNS):
        return LimitReachedError("the Codex usage limit is reached; rerun the stage later")
    return StageError(f"codex exited with {status}: {tail}")


def run_all(tasks: Sequence[Callable[[], None]], parallel: int) -> None:
    executor = ThreadPoolExecutor(max_workers=parallel)
    try:
        for future in as_completed([executor.submit(task) for task in tasks]):
            future.result()
    finally:
        executor.shutdown(cancel_futures=True)
