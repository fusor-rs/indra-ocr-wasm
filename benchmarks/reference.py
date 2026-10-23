"""The reference's speed on the CPU, measured as the page measures itself: the time from a page's
image to its lines, blocks and Markdown, with Datalab's indra-ocr package on ONNX Runtime
(`--mode ocr`, math as each page in pages.json asks), after one warm-up read.

    python reference.py <checkpoint dir> <pages.json> <images dir> <threads>
"""

import json
import sys
import time
from pathlib import Path

import indra_ocr
from indra_ocr import pipeline

checkpoint, pages, images, threads = sys.argv[1:5]
for name, spec in json.loads(Path(pages).read_text()).items():
    converter = indra_ocr.load(path=checkpoint, threads=int(threads), mode="ocr", math=spec["math"])
    gray = pipeline.read_images(Path(images) / spec["image"])[0]
    converter.ocr(gray)
    start = time.perf_counter()
    page = converter.ocr(gray)
    seconds = time.perf_counter() - start
    print(f"{name}: threads {threads}, {len(page.lines)} lines, {len(page.blocks)} blocks in {seconds:.2f}s", flush=True)
