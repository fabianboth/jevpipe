from pathlib import Path

import pytest

from bench import store


def test_written_file_reads_back_equal(tmp_path: Path) -> None:
    value = {"query": "q00", "probability": [0.1, None, 0.9], "text": "größe"}
    path = tmp_path / "runs" / "q00.json"

    store.write_json(path, value)

    assert store.read_json(path) == value


def test_failed_write_leaves_no_file(tmp_path: Path) -> None:
    path = tmp_path / "broken.json"

    with pytest.raises(TypeError):
        store.write_json(path, {"not json": object()})

    assert list(tmp_path.iterdir()) == []


def test_files_end_lines_with_lf(tmp_path: Path) -> None:
    path = tmp_path / "lines.json"

    store.write_json(path, {"a": [1, 2]})

    content = path.read_bytes()
    assert b"\r" not in content
    assert content.endswith(b"}\n")


def test_lists_of_plain_values_stay_on_one_line(tmp_path: Path) -> None:
    path = tmp_path / "run.json"

    store.write_json(path, {"probability": [0.1, None, 0.9], "items": [{"id": "j00001"}]})

    assert path.read_text(encoding="utf-8") == (
        '{\n  "probability": [0.1, null, 0.9],\n  "items": [\n    {\n      "id": "j00001"\n'
        "    }\n  ]\n}\n"
    )
