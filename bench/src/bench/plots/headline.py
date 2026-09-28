from dataclasses import dataclass
from pathlib import Path

from matplotlib.axes import Axes
from matplotlib.ticker import StrMethodFormatter

from bench.plots import style

_BAR = 0.3
_GAP = 0.03
_ROW_INCHES = 0.95
_LEFT = 0.31


@dataclass(frozen=True)
class Row:
    name: str
    detail: str
    found: float
    false_hits: float


@dataclass(frozen=True)
class Headline:
    header: style.Header
    axis: str
    rows: tuple[Row, ...]


def draw(chart: Headline, path: Path) -> None:
    target = style.canvas(chart.header, _ROW_INCHES * len(chart.rows), _LEFT)
    _bars(target.axes, chart.rows)
    _row_labels(target.axes, chart.rows)
    style.frame(target.axes, "x")
    target.axes.spines["bottom"].set_visible(False)
    target.axes.tick_params(axis="y", length=0)
    target.axes.xaxis.set_major_formatter(StrMethodFormatter("{x:,.0f}"))
    style.axis_label(target.axes, chart.axis, "")
    style.legend(target, [("relevant found", style.FOUND), ("false hits", style.FALSE_HITS)])
    style.save(target.figure, path)


def _positions(rows: tuple[Row, ...]) -> list[int]:
    return list(range(len(rows)))[::-1]


def _bars(axes: Axes, rows: tuple[Row, ...]) -> None:
    largest = max(max(row.found, row.false_hits) for row in rows)
    for position, row in zip(_positions(rows), rows, strict=True):
        for offset, value, color in (
            (_BAR / 2, row.found, style.FOUND),
            (-_BAR / 2, row.false_hits, style.FALSE_HITS),
        ):
            axes.barh(position + offset, value, height=_BAR - _GAP, color=color, linewidth=0)
            axes.text(
                value + largest * 0.012,
                position + offset,
                f"{value:,.0f}",
                va="center",
                fontsize=9,
                color=style.INK,
                family=style.FONT,
            )
    axes.set_xlim(0, largest * 1.12)
    axes.set_ylim(-0.6, len(rows) - 0.4)


def _row_labels(axes: Axes, rows: tuple[Row, ...]) -> None:
    axes.set_yticks(_positions(rows), labels=[""] * len(rows))
    transform = axes.get_yaxis_transform()
    for position, row in zip(_positions(rows), rows, strict=True):
        axes.text(
            -0.025,
            position + 0.13,
            row.name,
            transform=transform,
            ha="right",
            va="center",
            fontsize=11,
            fontweight="bold",
            color=style.INK,
            family=style.FONT,
        )
        axes.text(
            -0.025,
            position - 0.2,
            row.detail,
            transform=transform,
            ha="right",
            va="center",
            fontsize=8.5,
            color=style.SECONDARY,
            family=style.FONT,
            parse_math=False,
        )
