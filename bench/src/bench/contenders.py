from typing import Literal

from bench import deepseek

type Contender = Literal["grep-any", "grep-all", "grep-agent", "jevpipe", "deepseek"]
type Model = Literal["jevpipe", "deepseek"]

CONTENDERS: tuple[Contender, ...] = ("grep-any", "grep-all", "grep-agent", "jevpipe", "deepseek")
MODELS: tuple[Model, ...] = ("jevpipe", "deepseek")
SHOWN: tuple[Contender, ...] = ("grep-agent", "jevpipe", "deepseek")
NAMES: dict[Contender, str] = {
    "grep-any": "grep, any keyword",
    "grep-all": "grep, all keywords",
    "grep-agent": "grep, agent's pattern",
    "jevpipe": "jevpipe",
    "deepseek": deepseek.LABEL,
}
SHORT_NAMES: dict[Contender, str] = {**NAMES, "grep-agent": "grep"}
