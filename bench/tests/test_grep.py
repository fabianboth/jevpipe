from pathlib import Path

import pytest

from bench import grep

_FILES = {
    "0000.py": "def encrypt(data):\n    return AES.new(key).encrypt(data)",
    "0001.py": "def decrypt(data):\n    return data[::-1]",
    "0002.py": "def sort_list(items):\n    return sorted(items)",
    "0003.py": "import itertools\nitertools.permutations([1, 2])",
}


@pytest.fixture
def folder(tmp_path: Path) -> Path:
    for name, code in _FILES.items():
        (tmp_path / name).write_text(code, encoding="utf-8")
    return tmp_path


def test_keywords_drop_fillers_and_repeats() -> None:
    assert grep.keywords("How to sort a list in Python") == ("sort", "list", "python")
    assert grep.keywords("aes encryption, aes!") == ("aes", "encryption")


def test_any_keyword_finds_files_with_one_word(folder: Path) -> None:
    hits = grep.any_keyword(folder, ["aes", "permutations"])

    assert hits.indexes == (0, 3)


def test_all_keywords_needs_every_word(folder: Path) -> None:
    assert grep.all_keywords(folder, ["data", "encrypt"]).indexes == (0,)
    assert grep.all_keywords(folder, ["data", "sorted"]).indexes == ()


def test_a_pattern_is_a_case_insensitive_regex(folder: Path) -> None:
    assert grep.pattern(folder, r"sort(ed|_list)|PERMUTATION").indexes == (2, 3)


def test_an_invalid_pattern_does_not_compile() -> None:
    assert grep.compiles(r"aes|cipher")
    assert not grep.compiles(r"aes(")
