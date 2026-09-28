from collections import Counter

from bench import labels, selection

_POOL_SIZE = 60


def _flags(**sets: set[int]) -> selection.Flags:
    return selection.Flags(
        frozenset(sets.get("jevpipe", set())),
        frozenset(sets.get("deepseek", set())),
        frozenset(sets.get("grep_any", set())),
        frozenset(sets.get("grep_all", set())),
        frozenset(sets.get("grep_agent", set())),
    )


_RATED: set[labels.Pair] = {("q00", 0), ("q00", 1), ("q01", 2)}
_FLAGS = {
    "q00": _flags(jevpipe={1, 3}, deepseek={4}, grep_all={5}, grep_agent={6}, grep_any={5, 7, 8}),
    "q01": _flags(grep_any=set(range(10, 50)), jevpipe={2}),
}


def _by_pair(items: list[selection.Item]) -> dict[labels.Pair, set[str]]:
    return {(item.query, item.snippet): set(item.reasons) for item in items}


def test_every_rated_pair_is_validated_first() -> None:
    items = selection.select(_RATED, _FLAGS, _POOL_SIZE)

    first = items[: len(_RATED)]
    assert {(item.query, item.snippet) for item in first} == _RATED
    assert all(item.reasons == ("validation",) for item in first)


def test_unrated_hits_of_models_and_two_greps_are_selected() -> None:
    reasons = _by_pair(selection.select(_RATED, _FLAGS, _POOL_SIZE))

    for index in (3, 4, 5, 6):
        assert "hit" in reasons["q00", index]
    assert "hit" not in reasons.get(("q00", 7), set())
    assert reasons["q00", 1] == {"validation"}


def test_grep_any_sample_draws_at_most_twenty_unrated_hits() -> None:
    reasons = _by_pair(selection.select(_RATED, _FLAGS, _POOL_SIZE))

    sampled = [pair for pair, why in reasons.items() if "grep-any-sample" in why]
    per_query = Counter(query for query, _ in sampled)
    assert per_query["q00"] == 3
    assert per_query["q01"] == selection.GREP_ANY_SAMPLE
    assert all(10 <= index < 50 for query, index in sampled if query == "q01")


def test_unflagged_sample_avoids_every_flagged_and_rated_pair() -> None:
    items = selection.select(_RATED, _FLAGS, _POOL_SIZE)

    for item in items:
        if "unflagged-sample" in item.reasons:
            pair = (item.query, item.snippet)
            assert item.snippet not in _FLAGS[item.query].any_flag()
            assert pair not in _RATED
    unflagged = Counter(item.query for item in items if "unflagged-sample" in item.reasons)
    assert unflagged == {"q00": selection.UNFLAGGED_SAMPLE, "q01": selection.UNFLAGGED_SAMPLE}


def test_each_pair_is_selected_once_with_opaque_ids() -> None:
    items = selection.select(_RATED, _FLAGS, _POOL_SIZE)

    pairs = [(item.query, item.snippet) for item in items]
    assert len(pairs) == len(set(pairs))
    assert [item.id for item in items] == [f"j{n:05d}" for n in range(1, len(items) + 1)]


def test_selection_is_deterministic() -> None:
    assert selection.select(_RATED, _FLAGS, _POOL_SIZE) == selection.select(
        sorted(_RATED), dict(reversed(_FLAGS.items())), _POOL_SIZE
    )


def test_validation_and_the_rest_are_batched_apart_in_order() -> None:
    items = selection.select(_RATED, _FLAGS, _POOL_SIZE)

    split = selection.batches(items)

    assert [len(batch) for batch in split] == [
        len(_RATED),
        selection.BATCH,
        len(items) - len(_RATED) - selection.BATCH,
    ]
    assert [item for batch in split for item in batch] == items
    assert all(item.reasons == ("validation",) for item in split[0])


def test_validation_alone_is_the_start_of_the_full_selection() -> None:
    validation = selection.validation_items(_RATED)

    assert selection.select(_RATED, _FLAGS, _POOL_SIZE)[: len(validation)] == validation
