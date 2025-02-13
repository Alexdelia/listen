#!/usr/bin/env python3

import subprocess
import time
from contextlib import contextmanager
from datetime import datetime

import matplotlib.pyplot as plt

DATA_FILE = "listen.ron"
GRAPH_FILE = "history.svg"

NEW_FORMAT_TIMESTAMP = 1736956035


@contextmanager
def timer(name: str):
    print(f"\x1b[1;32m{name}\x1b[0m start")
    start = time.perf_counter()
    yield
    end = time.perf_counter()
    elapsed = end - start
    print(f"\x1b[1;32m{name}\x1b[0m: \x1b[1;35m{elapsed:.3f}\x1b[22ms\x1b[0m")


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

            line_count = int(
                subprocess.run(
                    f"git show {commit}:{DATA_FILE} | wc -l",
                    shell=True,
                    text=True,
                    capture_output=True,
                ).stdout.strip()
            )

            if int(timestamp) <= NEW_FORMAT_TIMESTAMP:
                line_count = line_count // 5

            recording_count = line_count - 2

            data.append((datetime.strptime(date, "%Y-%m-%d"), recording_count))

        data.sort(key=lambda x: x[0])

with timer("graph plotting"):
    x, y = zip(*data)
    plt.plot(x, y)
    plt.ylabel("recording")
    plt.title(f"recording count in {DATA_FILE} over time")

    plt.gcf().autofmt_xdate()
    plt.fill_between(x, y, color="skyblue", alpha=0.4)
    for spine in plt.gca().spines.values():
        spine.set_visible(False)

    for text_type in [
        "text.color",
        "axes.labelcolor",
        "xtick.color",
        "ytick.color",
        "axes.titlecolor",
    ]:
        plt.rcParams[text_type] = "white"

    plt.savefig(
        GRAPH_FILE,
        format=GRAPH_FILE.split(".")[-1],
        transparent=True,
        bbox_inches="tight",
        pad_inches=0,
    )
