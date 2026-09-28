from bench import dataset, export, labels, licenses, pool, store

_QUERIES = dataset.Queries((dataset.Query("q00", "copy to clipboard", "test"),), ())
_URL = "https://github.com/o/{}/blob/abc123/src/a.py#L3-L5"
_POOL = pool.Pool(
    (
        pool.Snippet("0000.py", _URL.format("free"), "h0"),
        pool.Snippet("0001.py", _URL.format("free"), "h1"),
        pool.Snippet("0002.py", _URL.format("dropped"), "h2"),
        pool.Snippet("0003.py", _URL.format("custom"), "h3"),
    ),
    (),
)
_SOURCE = export.Source(
    store.PYTHON,
    _QUERIES,
    _POOL,
    labels.Labels({("q00", 0): (3, 2)}, {("q00", 0): 1, ("q00", 1): 0, ("q00", 2): 3}),
    {("q00", 0): ("validation",), ("q00", 1): ("hit",), ("q00", 2): ("unflagged-sample",)},
    licenses.Licenses("2026-09-28", {"o/free": "MIT", "o/dropped": "", "o/custom": "NOASSERTION"}),
)


def test_the_corpus_holds_every_function_with_its_source_and_license() -> None:
    rows = list(export.corpus_rows(_SOURCE, lambda index: f"code {index}"))

    assert [row["id"] for row in rows] == [f"python/000{index}" for index in range(4)]
    first = rows[0]
    assert first["code"] == "code 0"
    assert (first["repository"], first["commit"], first["path"]) == ("o/free", "abc123", "src/a.py")
    assert (first["first_line"], first["last_line"], first["license"]) == (3, 5, "MIT")
    assert rows[2]["license"] is None
    assert rows[3]["license"] == "NOASSERTION"


def test_qrels_prefer_experts_keep_the_judge_and_say_why_a_pair_was_rated() -> None:
    expert, pooled, sampled = export.qrel_rows(_SOURCE)

    assert (expert["relevance"], expert["relevant"], expert["source"]) == (2.5, True, "expert")
    assert expert["judge_rating"] == 1
    assert expert["rated_because"] == "expert"
    assert (pooled["relevance"], pooled["relevant"], pooled["source"]) == (0, False, "judge")
    assert pooled["rated_because"] == "pooled"
    assert (sampled["function_id"], sampled["rated_because"]) == ("python/0002", "sample")


def test_the_card_fills_every_number_from_the_data() -> None:
    card = export.card([_SOURCE])

    assert "{" not in card.split("---", 2)[2]
    assert "| Python | 1 | 0% | 0.000 | **missed** |" in card
    assert "\nIn Python the judge missed the gate on only 1 relevant expert pairs;" in card
    assert "September 2026" in card


def test_the_card_counts_each_language() -> None:
    card = export.card([_SOURCE])

    assert "| Python | 1 | 4 | 1 | 2 | 2 |" in card
