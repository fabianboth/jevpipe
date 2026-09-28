import math
import statistics
import textwrap
from dataclasses import dataclass
from pathlib import Path

from matplotlib.axes import Axes
from matplotlib.ticker import PercentFormatter

from bench.plots import style

_MARKER = 5.0
_CHOSEN_MARKER = 9.0
_RING = 1.5
_GOLDEN = 0.618033988749895
_LABEL_SWITCH = 0.5
_DODGE = 0.12
_PADDING = 0.05


@dataclass(frozen=True)
class Curve:
    name: str
    color: str
    points: tuple[tuple[float, float, float], ...]
    chosen: float


@dataclass(frozen=True)
class Baseline:
    name: str
    recall: float
    precision: float


@dataclass(frozen=True)
class Tradeoff:
    header: style.Header
    curves: tuple[Curve, ...]
    baselines: tuple[Baseline, ...]


@dataclass(frozen=True)
class Bars:
    name: str
    color: str
    values: tuple[float, ...]
    counts: tuple[int, ...]


@dataclass(frozen=True)
class Calibration:
    header: style.Header
    bands: tuple[str, ...]
    series: tuple[Bars, ...]


@dataclass(frozen=True)
class Dots:
    name: str
    color: str
    values: tuple[float, ...]
    chosen: int | None


@dataclass(frozen=True)
class DotRows:
    header: style.Header
    labels: tuple[str, ...]
    series: tuple[Dots, ...]
    axis: str


@dataclass(frozen=True)
class Strip:
    name: str
    color: str
    seconds: tuple[float, ...]


@dataclass(frozen=True)
class Times:
    header: style.Header
    strips: tuple[Strip, ...]
    axis: str


def tradeoff(chart: Tradeoff, path: Path) -> None:
    target = style.canvas(chart.header, 4.2, 0.1)
    axes = target.axes
    for number, curve in enumerate(chart.curves):
        _curve(axes, curve, below=number > 0)
    for baseline in chart.baselines:
        axes.plot(
            baseline.recall,
            baseline.precision,
            "o",
            color=style.GREP,
            markersize=_MARKER + 2,
            markeredgecolor=style.SURFACE,
            markeredgewidth=_RING,
        )
        _baseline_label(axes, baseline)
    recalls = [r for curve in chart.curves for _, r, _ in curve.points]
    recalls += [baseline.recall for baseline in chart.baselines]
    axes.set_xlim(
        max(0.0, math.floor((min(recalls) - _PADDING) * 10) / 10),
        min(1.0, math.ceil((max(recalls) + _PADDING) * 10) / 10),
    )
    axes.set_ylim(0, 1)
    axes.xaxis.set_major_formatter(PercentFormatter(1.0))
    axes.yaxis.set_major_formatter(PercentFormatter(1.0))
    style.frame(axes, "both")
    style.axis_label(axes, "recall: share of the answer key found", "precision")
    entries = [(curve.name, curve.color) for curve in chart.curves] + [("grep", style.GREP)]
    style.legend(target, entries)
    style.save(target.figure, path)


def _curve(axes: Axes, curve: Curve, *, below: bool) -> None:
    recalls = [recall for _, recall, _ in curve.points]
    precisions = [precision for _, _, precision in curve.points]
    axes.plot(
        recalls,
        precisions,
        "-o",
        color=curve.color,
        linewidth=2,
        markersize=_MARKER,
        markeredgewidth=0,
        solid_capstyle="round",
        solid_joinstyle="round",
    )
    shift = -0.045 if below else 0.025
    for threshold, recall, precision in curve.points:
        chosen = threshold == curve.chosen
        if chosen:
            axes.plot(
                recall,
                precision,
                "o",
                color=curve.color,
                markersize=_CHOSEN_MARKER,
                markeredgecolor=style.SURFACE,
                markeredgewidth=_RING,
            )
        if chosen or threshold in {curve.points[0][0], curve.points[-1][0]}:
            _label(axes, (recall - 0.01, precision + shift), f"{threshold:.1f}")


def _baseline_label(axes: Axes, baseline: Baseline) -> None:
    right_side = baseline.recall < _LABEL_SWITCH
    axes.text(
        baseline.recall + (0.012 if right_side else -0.012),
        baseline.precision,
        baseline.name,
        ha="left" if right_side else "right",
        va="center",
        fontsize=8.5,
        color=style.SECONDARY,
        family=style.FONT,
    )


def _label(axes: Axes, at: tuple[float, float], text: str) -> None:
    axes.text(
        at[0], at[1], text, fontsize=8.5, color=style.SECONDARY, family=style.FONT, parse_math=False
    )


def calibration(chart: Calibration, path: Path) -> None:
    target = style.canvas(chart.header, 3.6, 0.1)
    axes = target.axes
    width = 0.8 / len(chart.series)
    for number, series in enumerate(chart.series):
        offset = (number - (len(chart.series) - 1) / 2) * width
        for position, (value, count) in enumerate(zip(series.values, series.counts, strict=True)):
            axes.bar(position + offset, value, width=width - 0.03, color=series.color, linewidth=0)
            axes.text(
                position + offset,
                value + 0.015,
                f"{value:.0%}",
                ha="center",
                fontsize=8.5,
                color=style.INK,
                family=style.FONT,
            )
            axes.text(
                position + offset,
                0.02,
                f"{count:,}",
                ha="center",
                fontsize=7,
                color=style.SURFACE,
                family=style.FONT,
            )
    axes.set_xticks(range(len(chart.bands)), labels=list(chart.bands))
    axes.set_ylim(0, 1.05)
    axes.yaxis.set_major_formatter(PercentFormatter(1.0))
    style.frame(axes, "y")
    style.axis_label(
        axes, "how sure the model was: max(p, 1 - p)", "decisions at 0.5 that were right"
    )
    style.legend(target, [(series.name, series.color) for series in chart.series])
    style.save(target.figure, path)


def dot_rows(chart: DotRows, path: Path) -> None:
    target = style.canvas(chart.header, 0.75 * len(chart.labels), 0.42)
    axes = target.axes
    rows = style.rows(len(chart.labels))
    for place, series in enumerate(chart.series):
        dodge = (place - (len(chart.series) - 1) / 2) * _DODGE
        for number, (row, value) in enumerate(zip(rows, series.values, strict=True)):
            chosen = number == series.chosen
            axes.plot(
                value,
                row - dodge,
                "o",
                color=series.color,
                markersize=_CHOSEN_MARKER if chosen else _MARKER + 2,
                markeredgecolor=style.INK if chosen else style.SURFACE,
                markeredgewidth=_RING,
            )
            if chosen:
                _label(axes, (value - 0.004, row - dodge + 0.16), f"{value:.3f}")
    style.frame(axes, "x")
    style.row_labels(axes, [textwrap.fill(label, 44) for label in chart.labels], 8.5)
    style.row_limits(axes, len(chart.labels))
    style.axis_label(axes, chart.axis, "")
    style.legend(target, [(series.name, series.color) for series in chart.series])
    style.save(target.figure, path)


def times(chart: Times, path: Path) -> None:
    target = style.canvas(chart.header, 0.8 * len(chart.strips), 0.21)
    axes = target.axes
    rows = style.rows(len(chart.strips))
    for row, strip in zip(rows, chart.strips, strict=True):
        jitter = [((number * _GOLDEN) % 1 - 0.5) * 0.4 for number in range(len(strip.seconds))]
        axes.scatter(
            strip.seconds,
            [row + offset for offset in jitter],
            s=22,
            color=strip.color,
            alpha=0.55,
            linewidths=0,
        )
        median = statistics.median(strip.seconds)
        axes.plot([median, median], [row - 0.32, row + 0.32], color=style.INK, linewidth=2)
        axes.text(
            median,
            row + 0.38,
            f"median {median:.1f} s",
            ha="center",
            fontsize=8.5,
            color=style.INK,
            family=style.FONT,
        )
    axes.set_xlim(left=0)
    style.frame(axes, "x")
    style.row_labels(axes, [strip.name for strip in chart.strips], 10)
    axes.set_ylim(-0.6, len(chart.strips) - 0.3)
    style.axis_label(axes, chart.axis, "")
    style.save(target.figure, path)
