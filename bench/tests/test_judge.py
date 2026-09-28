from bench import judge


def test_bad_ids_and_repeats_are_dropped() -> None:
    judgments: list[judge.Judgment] = [
        {"id": "j00001", "relevance": 3},
        {"id": "j00002", "relevance": 0},
        {"id": "j00002", "relevance": 1},
        {"id": "j99999", "relevance": 2},
    ]

    usable = judge.usable_ratings(judgments, {"j00001", "j00002", "j00003"})

    assert usable == {"j00001": 3}


def test_the_judge_sees_long_code_cut_with_a_note() -> None:
    short = "x" * judge.SHOWN_CHARACTERS
    long = "y" * (judge.SHOWN_CHARACTERS + 1234)

    assert judge.shown_code(short) == short
    assert judge.shown_code(long) == "y" * judge.SHOWN_CHARACTERS + "\n[cut: 1,234 more characters]"
