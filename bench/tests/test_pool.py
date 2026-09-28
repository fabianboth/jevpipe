from bench import pool, store

_CODE = {
    "https://github.com/o/r/blob/abc123/src/pkg/b.py#L2-L3": "def b():\n    return 2",
    "https://github.com/o/r/blob/abc123/a.py#L1": "import a",
}


def test_a_range_parses_to_the_raw_file_and_lines() -> None:
    source = pool.parse("https://github.com/o/r/blob/abc123/src/pkg/b.py#L2-L3")

    assert source == pool.Source("https://raw.githubusercontent.com/o/r/abc123/src/pkg/b.py", 2, 3)


def test_a_single_line_anchor_is_a_one_line_range() -> None:
    source = pool.parse("https://github.com/o/r/blob/abc123/a.py#L7")

    assert source == pool.Source("https://raw.githubusercontent.com/o/r/abc123/a.py", 7, 7)


def test_an_unknown_url_does_not_parse() -> None:
    assert pool.parse("https://gitlab.com/o/r/a.py#L7") is None


def test_cut_keeps_first_and_last_line() -> None:
    text = "one\r\ntwo\r\nthree\r\nfour\r\n"

    assert pool.cut(text, pool.Source("", 2, 3)) == "two\nthree"


def test_names_follow_url_order_and_missing_urls_are_listed() -> None:
    urls = [*_CODE, "https://github.com/o/r/blob/abc123/gone.py#L1", *_CODE]

    built, codes = pool.build(urls, _CODE.get, store.PYTHON)

    assert [snippet.name for snippet in built.snippets] == ["0000.py", "0001.py"]
    assert [snippet.url for snippet in built.snippets] == sorted(_CODE)
    assert codes == [_CODE[url] for url in sorted(_CODE)]
    assert built.missing == ("https://github.com/o/r/blob/abc123/gone.py#L1",)


def test_names_carry_the_language_extension_and_round_trip() -> None:
    assert store.SUITES["java"].name_of(42) == "0042.java"
    assert pool.index_of_name(store.SUITES["java"].name_of(42)) == 42
