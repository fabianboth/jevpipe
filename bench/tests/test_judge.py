from collections import Counter

from bench import judge, labels

_POOL_SIZE = 60


def _flags(**sets: set[int]) -> judge.Flags:
    return judge.Flags(
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


def _by_pair(items: list[judge.Item]) -> dict[labels.Pair, set[str]]:
    return {(item.query, item.snippet): set(item.reasons) for item in items}


def test_every_rated_pair_is_validated_first() -> None:
    items = judge.select(_RATED, _FLAGS, _POOL_SIZE)

    first = items[: len(_RATED)]
    assert {(item.query, item.snippet) for item in first} == _RATED
    assert all(item.reasons == ("validation",) for item in first)


def test_unrated_hits_of_models_and_two_greps_are_selected() -> None:
    reasons = _by_pair(judge.select(_RATED, _FLAGS, _POOL_SIZE))

    for index in (3, 4, 5, 6):
        assert "hit" in reasons["q00", index]
    assert "hit" not in reasons.get(("q00", 7), set())
    assert reasons["q00", 1] == {"validation"}


def test_grep_any_sample_draws_at_most_twenty_unrated_hits() -> None:
    reasons = _by_pair(judge.select(_RATED, _FLAGS, _POOL_SIZE))

    sampled = [pair for pair, why in reasons.items() if "grep-any-sample" in why]
    per_query = Counter(query for query, _ in sampled)
    assert per_query["q00"] == 3
    assert per_query["q01"] == judge.GREP_ANY_SAMPLE
    assert all(10 <= index < 50 for query, index in sampled if query == "q01")


def test_unflagged_sample_avoids_every_flagged_and_rated_pair() -> None:
    items = judge.select(_RATED, _FLAGS, _POOL_SIZE)

    for item in items:
        if "unflagged-sample" in item.reasons:
            pair = (item.query, item.snippet)
            assert item.snippet not in _FLAGS[item.query].any_flag()
            assert pair not in _RATED
    unflagged = Counter(item.query for item in items if "unflagged-sample" in item.reasons)
    assert unflagged == {"q00": judge.UNFLAGGED_SAMPLE, "q01": judge.UNFLAGGED_SAMPLE}


def test_each_pair_is_selected_once_with_opaque_ids() -> None:
    items = judge.select(_RATED, _FLAGS, _POOL_SIZE)

    pairs = [(item.query, item.snippet) for item in items]
    assert len(pairs) == len(set(pairs))
    assert [item.id for item in items] == [f"j{n:05d}" for n in range(1, len(items) + 1)]


def test_selection_is_deterministic() -> None:
    assert judge.select(_RATED, _FLAGS, _POOL_SIZE) == judge.select(
        sorted(_RATED), dict(reversed(_FLAGS.items())), _POOL_SIZE
    )


def test_validation_and_the_rest_are_batched_apart_in_order() -> None:
    items = judge.select(_RATED, _FLAGS, _POOL_SIZE)

    split = judge.batches(items)

    assert [len(batch) for batch in split] == [len(_RATED), judge.BATCH, len(items) - 43]
    assert [item for batch in split for item in batch] == items
    assert all(item.reasons == ("validation",) for item in split[0])


def test_validation_alone_is_the_start_of_the_full_selection() -> None:
    validation = judge.validation_items(_RATED)

    assert judge.select(_RATED, _FLAGS, _POOL_SIZE)[: len(validation)] == validation


def test_bad_ids_and_repeats_are_dropped() -> None:
    judgments: list[judge.Judgment] = [
        {"id": "j00001", "relevance": 3},
        {"id": "j00002", "relevance": 0},
        {"id": "j00002", "relevance": 1},
        {"id": "j99999", "relevance": 2},
    ]

    usable = judge.usable_ratings(judgments, {"j00001", "j00002", "j00003"})

    assert usable == {"j00001": 3}


def test_agreement_of_judge_and_experts() -> None:
    expert = {("q00", 0): (3, 3), ("q00", 1): (0, 3, 3), ("q00", 2): (1,), ("q00", 3): (2, 0)}
    judged = {("q00", 0): 3, ("q00", 1): 1, ("q00", 2): 2, ("q00", 9): 3}

    found = judge.agreement(expert, judged)

    assert found.judged == 3
    assert (found.judge.found, found.judge.false_hits, found.judge.known_relevant) == (1, 1, 2)
    assert (found.experts.found, found.experts.false_hits, found.experts.known_relevant) == (
        1,
        1,
        2,
    )


def test_the_judge_sees_long_code_cut_with_a_note() -> None:
    short = "x" * judge.SHOWN_CHARACTERS
    long = "y" * (judge.SHOWN_CHARACTERS + 1234)

    assert judge.shown_code(short) == short
    assert judge.shown_code(long) == "y" * judge.SHOWN_CHARACTERS + "\n[cut: 1,234 more characters]"
