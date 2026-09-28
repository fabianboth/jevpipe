from bench import dataset, labels, pool

_EXPERT = {("q00", 1): (3, 2), ("q00", 2): (1, 1)}


def test_expert_rating_wins_over_the_judge() -> None:
    known = labels.Labels(_EXPERT, {("q00", 1): 0, ("q00", 2): 3})

    assert known.relevant(("q00", 1)) is True
    assert known.relevant(("q00", 2)) is False


def test_the_judge_fills_gaps() -> None:
    known = labels.Labels(_EXPERT).with_judge({("q00", 3): 2, ("q00", 4): 1})

    assert known.relevant(("q00", 3)) is True
    assert known.relevant(("q00", 4)) is False


def test_an_unrated_pair_is_unknown() -> None:
    known = labels.Labels(_EXPERT)

    assert known.rating(("q00", 5)) is None
    assert known.relevant(("q00", 5)) is None


def test_expert_pairs_use_query_ids_and_pool_indexes() -> None:
    queries = dataset.split(["aes encryption"] + [f"q {i}" for i in range(30)])
    snippets = pool.Pool(
        (pool.Snippet("0000.py", "u0", ""), pool.Snippet("0001.py", "u1", "")), ("gone",)
    )
    ratings = {("aes encryption", "u1"): (3,), ("aes encryption", "gone"): (2,)}

    assert labels.experts(queries, snippets, ratings) == {("q00", 1): (3,)}
