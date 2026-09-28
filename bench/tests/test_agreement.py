from bench import agreement, labels


def test_the_judge_is_compared_on_every_pair_the_experts_rated() -> None:
    expert = {("q00", 0): (3, 3), ("q00", 1): (0, 3, 3), ("q00", 2): (1,), ("q00", 3): (2, 0)}
    judged = {("q00", 0): 3, ("q00", 1): 1, ("q00", 2): 2, ("q00", 9): 3}

    found = agreement.agreement(expert, judged)

    judge = found.judge
    assert (judge.pairs, judge.same) == (3, 1)
    assert (judge.counts.found, judge.counts.false_hits, judge.counts.known_relevant) == (1, 1, 2)
    assert found.twice == 2
    assert found.experts is None
    assert found.judge_on_twice is None


def test_with_enough_pairs_rated_twice_judge_and_experts_are_compared_alike() -> None:
    expert: dict[labels.Pair, tuple[int, ...]] = {}
    judged: dict[labels.Pair, int] = {}
    for index in range(agreement.MIN_RATED_TWICE):
        expert["q00", index] = (3, 3) if index % 2 else (0, 3)
        judged["q00", index] = 3

    found = agreement.agreement(expert, judged)

    assert found.twice == agreement.MIN_RATED_TWICE
    assert found.judge_on_twice is not None
    assert found.experts is not None
    assert found.judge_on_twice.same_share == 1.0
    assert found.experts.same_share == 0.5


def test_the_gate_is_on_the_judges_f1() -> None:
    expert = {("q00", index): (3,) for index in range(4)}
    agrees = agreement.agreement(expert, dict.fromkeys(expert, 3))
    disagrees = agreement.agreement(expert, dict.fromkeys(expert, 0))

    assert agrees.passed
    assert not disagrees.passed
    assert "only 0 pairs rated twice" in agreement.describe(agrees)
