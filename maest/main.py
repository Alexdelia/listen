import json
import subprocess
import sys

import numpy
import torch
from transformers import AutoFeatureExtractor, AutoModelForAudioClassification

RATE = 16000
SPAN_SECOND = 90
WINDOW_SECOND = 30
WINDOW_COUNT = 3


def decode(audio):
    pcm = subprocess.run(
        ["ffmpeg", "-v", "quiet", "-i", audio, "-ac", "1", "-ar", str(RATE), "-f", "f32le", "-"],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    return numpy.frombuffer(pcm, dtype=numpy.float32)


def middle(samples, length):
    start = max(0, (len(samples) - length) // 2)
    return samples[start : start + length]


def windows(samples):
    length = WINDOW_SECOND * RATE
    if len(samples) <= length:
        return [samples]
    starts = numpy.linspace(0, len(samples) - length, WINDOW_COUNT).astype(int)
    return [samples[start : start + length] for start in starts]


def scores(audio, weights):
    device = "cuda" if torch.cuda.is_available() else "cpu"
    extractor = AutoFeatureExtractor.from_pretrained(weights, trust_remote_code=True)
    model = AutoModelForAudioClassification.from_pretrained(weights, trust_remote_code=True).to(device).eval()
    clips = windows(middle(decode(audio), SPAN_SECOND * RATE))
    if not clips[0].size:
        sys.exit(f"no audio decoded from {audio}")
    inputs = extractor(clips, sampling_rate=RATE, return_tensors="pt").to(device)
    with torch.no_grad():
        probability = torch.sigmoid(model(**inputs).logits).mean(0).cpu().tolist()
    return {model.config.id2label[index]: p for index, p in enumerate(probability)}


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(f"usage: {sys.argv[0]} <audio> <weights_dir>")
    json.dump(scores(sys.argv[1], sys.argv[2]), sys.stdout)
