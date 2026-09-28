from dataclasses import dataclass
from pathlib import Path
from typing import Literal

from matplotlib.axes import Axes
from matplotlib.figure import Figure
from matplotlib.patches import Patch

SURFACE = "#fcfcfb"
INK = "#0b0b0b"
SECONDARY = "#52514e"
MUTED = "#898781"
GRID = "#e1e0d9"
BASELINE = "#c3c2b7"
FOUND = "#2a78d6"
FALSE_HITS = "#eb6834"
JEVPIPE = "#2a78d6"
DEEPSEEK = "#1baf7a"
GREP = "#898781"
FONT = "DejaVu Sans"
_WIDTH_INCHES = 8.0
_DPI = 200
_HEADER_INCHES = 1.25
_FOOTER_INCHES = 0.95
_RIGHT = 0.94


@dataclass(frozen=True)
class Header:
    title: str
    subtitle: str
    footnote: str


@dataclass(frozen=True)
class Canvas:
    figure: Figure
    axes: Axes
    height: float


def canvas(header: Header, body_inches: float, left: float) -> Canvas:
    height = _HEADER_INCHES + _FOOTER_INCHES + body_inches
    figure = Figure(figsize=(_WIDTH_INCHES, height), dpi=_DPI, facecolor=SURFACE)
    bottom = _FOOTER_INCHES / height
    top = 1 - _HEADER_INCHES / height
    axes = figure.add_axes((left, bottom, _RIGHT - left, top - bottom), facecolor=SURFACE)
    figure.text(
        0.03,
        1 - 0.3 / height,
        header.title,
        fontsize=13.5,
        fontweight="bold",
        color=INK,
        family=FONT,
        va="center",
        parse_math=False,
    )
    figure.text(
        0.03,
        1 - 0.62 / height,
        header.subtitle,
        fontsize=9.5,
        color=SECONDARY,
        family=FONT,
        va="center",
        parse_math=False,
    )
    figure.text(
        0.03,
        0.1 / height,
        header.footnote,
        fontsize=7.5,
        color=MUTED,
        family=FONT,
        va="bottom",
        linespacing=1.5,
        parse_math=False,
    )
    return Canvas(figure, axes, height)


def legend(target: Canvas, entries: list[tuple[str, str]]) -> None:
    handles = [Patch(facecolor=color, label=label) for label, color in entries]
    target.figure.legend(
        handles=handles,
        loc="center left",
        bbox_to_anchor=(0.022, 1 - 0.93 / target.height),
        ncols=len(handles),
        frameon=False,
        labelcolor=SECONDARY,
        handlelength=1.0,
        handleheight=1.0,
        columnspacing=1.2,
        prop={"family": FONT, "size": 9},
    )


def frame(axes: Axes, grid: Literal["both", "x", "y"]) -> None:
    axes.tick_params(colors=MUTED, labelsize=8, length=0)
    for label in [*axes.get_xticklabels(), *axes.get_yticklabels()]:
        label.set_fontfamily(FONT)
    axes.grid(axis=grid, color=GRID, linewidth=0.8)
    axes.set_axisbelow(True)
    for side in ("top", "right"):
        axes.spines[side].set_visible(False)
    for side in ("left", "bottom"):
        axes.spines[side].set_color(BASELINE)


def rows(count: int) -> list[int]:
    return list(range(count))[::-1]


def row_limits(axes: Axes, count: int) -> None:
    axes.set_ylim(-0.6, count - 0.4)


def row_labels(axes: Axes, labels: list[str], size: float) -> None:
    axes.set_yticks(rows(len(labels)), labels=labels)
    for label in axes.get_yticklabels():
        label.set_color(INK)
        label.set_fontsize(size)


def axis_label(axes: Axes, x: str, y: str) -> None:
    axes.set_xlabel(x, color=MUTED, fontsize=8.5, family=FONT, labelpad=6)
    axes.set_ylabel(y, color=MUTED, fontsize=8.5, family=FONT, labelpad=6)


def save(figure: Figure, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(path, facecolor=SURFACE, metadata={"Software": None})
