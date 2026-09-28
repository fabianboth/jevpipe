from bench import dataset, store

_CSV = """Language,Query,GitHubUrl,Relevance,Notes
Python,aes encryption,https://github.com/a/b/blob/1/x.py#L1-L2,3,
Python,aes encryption,https://github.com/a/b/blob/1/x.py#L1-L2,1,
Go,aes encryption,https://github.com/a/b/blob/1/x.go#L1-L2,3,
Python,sort a list,https://github.com/a/b/blob/1/y.py#L4,0,
"""


def test_ratings_keep_every_python_rating_per_pair() -> None:
    ratings = dataset.ratings_of(_CSV, store.PYTHON)

    assert ratings == {
        ("aes encryption", "https://github.com/a/b/blob/1/x.py#L1-L2"): (3, 1),
        ("sort a list", "https://github.com/a/b/blob/1/y.py#L4"): (0,),
    }


def test_a_mean_of_two_or_more_is_relevant() -> None:
    assert dataset.is_relevant(dataset.mean([3, 1]))
    assert not dataset.is_relevant(dataset.mean([2, 1]))


def test_split_is_deterministic_and_disjoint() -> None:
    texts = [f"query {index}" for index in range(99)]

    first = dataset.split(texts)
    second = dataset.split(reversed(texts))

    assert first == second
    assert len(first.of_split("dev")) == dataset.DEV_QUERIES
    assert len(first.of_split("test")) == 99 - dataset.DEV_QUERIES
    test_ids = {query.id for query in first.of_split("test")}
    assert len(first.repeat) == dataset.REPEAT_QUERIES
    assert set(first.repeat) <= test_ids


def test_query_ids_follow_the_sorted_texts() -> None:
    queries = dataset.split(["sort a list", "aes encryption"] + [f"q {i}" for i in range(30)])

    assert queries.by_id("q00").text == "aes encryption"


def test_other_languages_keep_the_primary_ids_and_are_all_test() -> None:
    primary = dataset.split(["sort a list", "aes encryption"] + [f"q {i}" for i in range(30)])

    queries = dataset.all_test(primary, ["sort a list"])

    assert [(query.text, query.split) for query in queries.queries] == [("sort a list", "test")]
    assert queries.queries[0].id == next(q.id for q in primary.queries if q.text == "sort a list")
    assert queries.repeat == ()


def test_ratings_follow_the_language() -> None:
    java = dataset.ratings_of(_CSV, store.SUITES["go"])

    assert java == {("aes encryption", "https://github.com/a/b/blob/1/x.go#L1-L2"): (3,)}
