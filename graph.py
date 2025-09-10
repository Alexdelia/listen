#!/usr/bin/env python3

import os
import subprocess
import time
from contextlib import contextmanager
from datetime import datetime, timedelta

import matplotlib.pyplot as plt
from matplotlib.dates import num2date

DATA_FILE = "listen.ron"
GRAPH_FILE = "history.svg"

NEW_FORMAT_TIMESTAMP = 1736956035

CONTENT_COLOR = "#d5397b"
TEXT_COLOR = "#cccccc"


@contextmanager
def timer(name: str):
    print(f"\x1b[1;32m{name}\x1b[0m start")
    start = time.perf_counter()
    yield
    end = time.perf_counter()
    elapsed = end - start
    print(f"\x1b[1;32m{name}\x1b[0m: \x1b[1;35m{elapsed:.3f}\x1b[22ms\x1b[0m")


with timer(os.path.basename(__file__)):
    with timer("data extraction"):
        with timer("git log"):
            datapoints = (
                subprocess.run(
                    f"git log --format='%H %cs %ct' -- {DATA_FILE} | awk '!seen[$2]++ {{print $1, $2, $3, $4}}'",
                    shell=True,
                    text=True,
                    capture_output=True,
                )
                .stdout.strip()
                .splitlines()[::-1]
            )

        with timer("line count"):
            data = []

            for datapoint in datapoints:
                commit, date, timestamp = datapoint.split()

                recording_count = (
                    int(
                        subprocess.run(
                            f"git show {commit}:{DATA_FILE} | wc -l",
                            shell=True,
                            text=True,
                            capture_output=True,
                        ).stdout.strip()
                    )
                    - 2
                )

                if int(timestamp) <= NEW_FORMAT_TIMESTAMP:
                    recording_count //= 5

                data.append((datetime.strptime(date, "%Y-%m-%d"), recording_count))

            data.sort(key=lambda x: x[0])

    with timer("graph plotting"):
        x, y = zip(*data)
        min_x, max_x = x[0], x[-1]
        min_y, max_y = y[0], y[-1]

        for text_type in [
            "text.color",
            "axes.labelcolor",
            "xtick.color",
            "ytick.color",
            "axes.titlecolor",
        ]:
            plt.rcParams[text_type] = TEXT_COLOR
        plt.rcParams["font.family"] = "monospace"

        plt.plot(x, y, color=CONTENT_COLOR, alpha=0.5)

        # plt.plot(x, y, color=CONTENT_COLOR)
        plt.ylabel("recording")
        plt.title(f"recording count in {DATA_FILE} over time")

        plt.gcf().autofmt_xdate(
            rotation=-45,
            ha="left",
        )
        plt.fill_between(x, y, color=CONTENT_COLOR, alpha=0.4)
        for spine in plt.gca().spines.values():
            spine.set_visible(False)
        plt.margins(x=0, y=0)
        plt.xlim(min_x, max_x)
        plt.ylim(min_y, max_y + 1)

        min_x_edge = min_x.replace(tzinfo=None) + timedelta(days=15)
        max_x_edge = max_x.replace(tzinfo=None) - timedelta(days=15)
        xticks = []
        for tick in plt.gca().get_xticks():
            tick = num2date(tick).replace(tzinfo=None)
            if tick > min_x_edge and tick < max_x_edge:
                xticks.append(tick)
        plt.xticks([min_x] + xticks + [max_x])
        plt.yticks([tick for tick in plt.gca().get_yticks() if tick < max_y - 33] + [max_y])

        plt.gca().xaxis.set_label_coords(-0.05, 0.5)
        plt.gca().yaxis.tick_right()
        plt.gca().yaxis.set_label_position("right")

        today = datetime.today().replace(hour=0, minute=0, second=0, microsecond=0)
        total_days = (today - min_x).days
        average = max_y / total_days if total_days > 0 else 0.0

        plt.text(
            0.1,
            0.9,
            f"days: {total_days}\n avg: {average:.2f}",
            color=TEXT_COLOR,
            transform=plt.gca().transAxes,
            va="top",
            family="monospace",
            linespacing=1.5,
        )

        plt.savefig(
            GRAPH_FILE,
            format=GRAPH_FILE.split(".")[-1],
            transparent=True,
            bbox_inches="tight",
            pad_inches=0,
        )
