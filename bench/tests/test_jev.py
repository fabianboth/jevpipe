import json

from bench import jev


def _line(record: str, noul: float) -> str:
    return json.dumps({"record": record, "answers": {"match": {"type": "noul", "noul": noul}}})


def test_probabilities_follow_the_names_with_gaps_for_unanswered() -> None:
    stdout = "\n".join(
        [
            _line("0001.py", 0.9),
            json.dumps({"record": "0002.py", "outcome": "skipped", "reason": "empty"}),
            _line("0000.py", 0.1),
            json.dumps({"record": "0003.py", "outcome": "failed", "reason": "timeout"}),
        ]
    )

    parsed = jev.parse(["0000.py", "0001.py", "0002.py", "0003.py", "0004.py"], stdout)

    assert parsed.probability == (0.1, 0.9, None, None, None)
    assert (parsed.skipped, parsed.failed) == (1, 2)


def test_cost_comes_from_the_summary_line() -> None:
    stderr = "jevpipe: 943 records, 943 answered, 0 skipped, 0 failed, $0.0204, 8.7s\n"

    assert jev.cost_of(stderr) == 0.0204
    assert jev.cost_of("jevpipe: 0 records, 0 answered, 0 skipped, 0 failed, 0.1s") == 0.0
