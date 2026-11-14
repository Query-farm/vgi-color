"""Presentation and result checks for the controlled sRGB contrast investigation."""

import math
import re

import matplotlib.pyplot as plt
from IPython.display import display
from matplotlib.patches import FancyBboxPatch
from vgi_notebooks import finish, prose

INK = "#25344A"
MUTED = "#617086"
ACCENT = "#326C8F"
SOURCE = "Source: synthetic design brief + worker CSS catalog · native VGI Color v0.5.0 · opaque sRGB"


def validate_parameters(brand, label, minimum):
    for name, value in {"BRAND_HEX": brand, "LABEL_HEX": label}.items():
        if (
            not isinstance(value, str)
            or re.fullmatch(r"#[0-9a-fA-F]{6}", value) is None
        ):
            raise ValueError(
                f"{name} must be an opaque six-digit sRGB hex color, e.g. '#ff6347'."
            )
    if (
        isinstance(minimum, bool)
        or not isinstance(minimum, (int, float))
        or not math.isfinite(minimum)
        or not 1 <= minimum <= 21
    ):
        raise ValueError("MIN_CONTRAST must be a finite ratio from 1 to 21.")
    return "'" + brand.lower() + "'", "'" + label.lower() + "'"


def run_note(BRAND_HEX, LABEL_HEX, MIN_CONTRAST):
    prose(
        f"**Local computational run.** Background `{BRAND_HEX}`, label `{LABEL_HEX}`, "
        f"target **{MIN_CONTRAST:g}:1**. "
        "Worker release **v0.5.0**, verified by the VGI GitHub transport. No external color measurements are fetched."
    )


def baseline_finding(baseline_frame, MIN_CONTRAST):
    display(baseline_frame)
    base = baseline_frame.iloc[0]
    status = "meets" if base.contrast_ratio >= MIN_CONTRAST else "misses"
    prose(
        f"The original pair **{status} the {MIN_CONTRAST:g}:1 target** at "
        f"**{base.contrast_ratio:.4f}:1**. Its label/background distance is "
        f"**ΔE₀₀ {base.pair_delta_e:.2f}**. That second number does not determine "
        "whether the text-contrast condition passes. All eligibility comparisons below use "
        "unrounded ratios; displayed digits are for inspection."
    )
    prose(
        f"The background relative luminance is **{base.background_luminance:.4f}** on a 0–1 scale. "
        f"The worker returns text grade **{base.worker_text_grade}** for the original pair, independently of our custom target. "
        f"Its black/white crossover chooses **`{base.alternate_label}`** text; the decision cards below measure that alternative."
    )


def css_chart(css_frame, baseline_frame, BRAND_HEX, LABEL_HEX, MIN_CONTRAST):
    base = baseline_frame.iloc[0]
    fig, ax = plt.subplots(figsize=(10.4, 6.6))
    passed = css_frame.contrast_ratio >= MIN_CONTRAST
    for mask, marker, label in [
        (~passed, "o", "Below target"),
        (passed, "s", "Meets target"),
    ]:
        rows = css_frame.loc[mask]
        ax.scatter(
            rows.pair_delta_e,
            rows.contrast_ratio,
            c=rows.hex,
            marker=marker,
            s=57,
            edgecolors=INK,
            linewidths=0.45,
            alpha=0.93,
            label=label,
            zorder=3,
        )
    ax.axhline(
        MIN_CONTRAST,
        color=ACCENT,
        linestyle="--",
        linewidth=1.4,
        label=f"Target {MIN_CONTRAST:g}:1",
    )
    ax.scatter(
        [base.pair_delta_e],
        [base.contrast_ratio],
        marker="*",
        s=240,
        facecolors=BRAND_HEX,
        edgecolors=INK,
        linewidths=1.1,
        zorder=5,
    )
    ax.annotate(
        "Original brand",
        (base.pair_delta_e, base.contrast_ratio),
        xytext=(28, -27),
        textcoords="offset points",
        ha="left",
        fontsize=9,
        arrowprops={"arrowstyle": "-", "color": MUTED},
    )
    for name, offset in [
        ("blue", (10, -10)),
        ("lime", (-18, -28)),
        ("gray", (-40, 22)),
    ]:
        row = css_frame.loc[css_frame.name.eq(name)].iloc[0]
        ax.annotate(
            name,
            (row.pair_delta_e, row.contrast_ratio),
            xytext=offset,
            textcoords="offset points",
            fontsize=9,
            color=INK,
            arrowprops={"arrowstyle": "-", "color": MUTED},
        )
    ax.set(
        xlabel="Label ↔ background difference · CIEDE2000 ΔE₀₀ (unitless)",
        ylabel="Label / background contrast (ratio : 1)",
        ylim=(0, 22),
    )
    ax.legend(loc="upper left", fontsize=9)
    finish(
        fig,
        "Color separation is not a contrast test",
        f"{len(css_frame)} distinct CSS backgrounds · fixed label {LABEL_HEX} · shape marks the {MIN_CONTRAST:g}:1 decision",
        SOURCE,
        name="01-css-contrast",
    )


def inversion_finding(inversion):
    if len(inversion):
        v = inversion.iloc[0]
        prose(
            f"A concrete reversal: **{v.larger_distance_color}** is farther from the label "
            f"(ΔE₀₀ {v.larger_distance:.2f}) yet has only **{v.lower_contrast:.2f}:1** contrast. "
            f"**{v.smaller_distance_color}** is closer (ΔE₀₀ {v.smaller_distance:.2f}) "
            f"but yields **{v.higher_contrast:.2f}:1**. Ranking by color difference would "
            "therefore answer a different question. Next we constrain contrast first, "
            "then minimize departure from the brand."
        )
    else:
        prose(
            "No ordering reversal appears in this selected set, which is not evidence that "
            "the two metrics are interchangeable. The next search treats them as separate objectives."
        )


def space_finding(space_frame):
    display(space_frame)
    r = space_frame.iloc[0]
    prose(
        f"Hex **`{r.brand_hex}`** becomes RGB **({r.r},{r.g},{r.b})** and HSL "
        f"**({r.h:.2f}°, {r.s:.4f}, {r.l:.4f})**. The family holds the first two HSL "
        "coordinates fixed. HSL lightness is a construction coordinate, while relative "
        "luminance weights linearized RGB channels; they are not interchangeable."
    )


def shade_chart(candidate_frame, MIN_CONTRAST, LABEL_HEX):
    selected = candidate_frame.loc[candidate_frame.selected]
    fig, axes = plt.subplots(1, 2, figsize=(11.4, 6.5), sharex=True)
    for ax, metric in zip(axes, ["contrast_ratio", "brand_delta_e"]):
        ax.plot(
            candidate_frame.lightness,
            candidate_frame[metric],
            color="#A8B3C0",
            linewidth=1,
            zorder=1,
        )
        ax.scatter(
            candidate_frame.lightness,
            candidate_frame[metric],
            c=candidate_frame.hex,
            edgecolors=INK,
            linewidths=0.35,
            s=26,
            zorder=3,
        )
        ax.set_xlabel("HSL lightness (0–1)")
        if len(selected):
            s = selected.iloc[0]
            ax.axvline(s.lightness, color=ACCENT, linestyle=":", linewidth=1.3)
            ax.scatter(
                [s.lightness],
                [s[metric]],
                marker="*",
                s=225,
                facecolors=s.hex,
                edgecolors=INK,
                linewidths=1.2,
                zorder=5,
            )
    axes[0].axhline(MIN_CONTRAST, linestyle="--", color=ACCENT, linewidth=1.4)
    axes[0].set(
        ylabel="Label / background contrast (ratio : 1)",
        ylim=(0, 22),
        title="1 · Meet the contrast target",
    )
    axes[0].text(
        0.98,
        0.96,
        f"Target {MIN_CONTRAST:g}:1",
        transform=axes[0].transAxes,
        ha="right",
        va="top",
        color=ACCENT,
        fontsize=10,
    )
    axes[1].set(
        ylabel="Original ↔ candidate difference · ΔE₀₀",
        title="2 · Minimize the brand change",
        ylim=(0, None),
    )
    if len(selected):
        s = selected.iloc[0]
        axes[1].text(
            0.98,
            0.96,
            f"Selected {s.hex}\nΔE₀₀ {s.brand_delta_e:.2f}",
            transform=axes[1].transAxes,
            ha="right",
            va="top",
            fontsize=10,
            color=INK,
        )
    finish(
        fig,
        "A constrained search, with two different measurements",
        "Fixed hue and saturation · 0.01 HSL lightness grid + original · star = closest eligible background",
        SOURCE,
        name="02-shade-search",
    )
    eligible_count = int((candidate_frame.contrast_ratio >= MIN_CONTRAST).sum())
    if len(selected):
        s = selected.iloc[0]
        prose(
            f"**{eligible_count} of {len(candidate_frame)} candidates meet {MIN_CONTRAST:g}:1.** "
            f"The closest eligible background is **`{s.hex}`** at **{s.contrast_ratio:.4f}:1**, "
            f"with brand distance **ΔE₀₀ {s.brand_delta_e:.2f}** and HSL lightness **{s.lightness:.3f}**. "
            "This is the optimum within the sampled family only: allowing hue or saturation "
            "to change, or refining the grid, could produce a different answer."
        )
    else:
        prose(
            f"**No sampled background meets {MIN_CONTRAST:g}:1 with label `{LABEL_HEX}`.** "
            "The chosen result is intentionally absent; no failing candidate is presented as a success. "
            "The final comparison still shows the original and the black/white label alternative."
        )


def decision_chart(decision_frame, MIN_CONTRAST, BRAND_HEX):
    fig, axes = plt.subplots(1, len(decision_frame), figsize=(11.4, 5.6), squeeze=False)
    for ax, row in zip(axes[0], decision_frame.itertuples()):
        ax.set(xlim=(0, 1), ylim=(0, 1))
        ax.axis("off")
        ax.text(
            0.5,
            0.96,
            row.option,
            ha="center",
            va="top",
            fontsize=12,
            fontweight="semibold",
            color=INK,
        )
        ax.add_patch(
            FancyBboxPatch(
                (0.06, 0.35),
                0.88,
                0.47,
                boxstyle="round,pad=0.02,rounding_size=0.035",
                facecolor=row.background,
                edgecolor="#CBD3DD",
                linewidth=0.8,
            )
        )
        ax.text(
            0.5,
            0.585,
            "Read the label",
            ha="center",
            va="center",
            color=row.foreground,
            fontsize=16,
        )
        ax.text(
            0.5,
            0.25,
            f"{row.contrast_ratio:.4f}:1 · {'MEETS' if row.meets_target else 'BELOW'} target",
            ha="center",
            va="center",
            fontsize=10.5,
            color=INK,
        )
        ax.text(
            0.5,
            0.12,
            f"Background {row.background} · text {row.foreground}\nBackground ΔE₀₀ {row.background_delta_e:.2f}",
            ha="center",
            va="center",
            fontsize=9.2,
            color=MUTED,
            linespacing=1.7,
        )
    finish(
        fig,
        "Two ways to change the design brief",
        f"Contrast target {MIN_CONTRAST:g}:1 · background ΔE₀₀ is measured from the original {BRAND_HEX}",
        SOURCE,
        name="03-design-options",
    )
    alt = decision_frame.iloc[-1]
    alt_status = "meets" if alt.meets_target else "does not meet"
    prose(
        f"Keeping `{BRAND_HEX}` and using **`{alt.foreground}`** text gives "
        f"**{alt.contrast_ratio:.4f}:1**, which **{alt_status}** the current target. "
        "Its background ΔE₀₀ is zero because the background is unchanged; "
        "the label itself may change substantially. The choice depends on which part "
        "of the design brief can move. Neither metric evaluates text size, focus states, "
        "color-only meaning, typography, or the whole user experience."
    )


def check_results(validation):
    display(validation.T.rename(columns={0: "passed"}))
    assert validation.notna().all().all(), (
        "A required relationship returned no evidence."
    )
    assert validation.astype(bool).all().all(), (
        "A color-science or selection relationship failed."
    )
    prose(
        f"**All {validation.shape[1]} relationship checks passed.** Eligibility uses the full "
        "returned contrast ratio, including near a threshold; chart labels and tables alone are rounded."
    )
