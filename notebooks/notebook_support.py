"""Small presentation and path helpers; color science stays in visible SQL."""
from __future__ import annotations


import matplotlib.pyplot as plt
import pandas as pd
import seaborn as sns
from IPython.display import Markdown, display

SOURCE = "Source: synthetic design brief + worker CSS catalog · Rust vgi-color via Haybarn / VGI · opaque sRGB"
INK = "#25344A"
MUTED = "#617086"
ACCENT = "#326C8F"


def sql_text(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def prose(value: str) -> None:
    display(Markdown(value))


def set_theme() -> None:
    sns.set_theme(style="whitegrid", context="notebook", rc={
        "font.family": "DejaVu Sans", "font.sans-serif": ["DejaVu Sans"],
        "figure.dpi": 150, "figure.facecolor": "white", "axes.facecolor": "white",
        "text.color": INK, "axes.labelcolor": INK, "xtick.color": MUTED,
        "ytick.color": MUTED, "grid.color": "#E6EAF0", "axes.edgecolor": "#CBD3DD",
        "axes.spines.top": False, "axes.spines.right": False,
        "axes.titlelocation": "left", "axes.titleweight": "semibold",
        "legend.frameon": False, "text.parse_math": False,
    })
    pd.set_option("display.max_columns", 15)
    pd.set_option("display.precision", 4)


def chart(fig, title: str, subtitle: str) -> None:
    fig.text(0.08, 0.985, title, ha="left", va="top", fontsize=17, fontweight="semibold")
    fig.text(0.08, 0.91, subtitle, ha="left", va="top", fontsize=10.5, color=MUTED)
    sns.despine(fig=fig)
    fig.tight_layout(rect=(0, 0.08, 1, 0.83), w_pad=3)
    fig.text(0.08, 0.025, SOURCE, ha="left", va="bottom", fontsize=8.3, color=MUTED)
    plt.show()
