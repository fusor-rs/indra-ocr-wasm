# Benchmarks

How fast Indra OCR reads a page on one machine, in every runtime this project has: the page in
Chrome (eight threads and one), the native `indra-ocr` CLI, the single-threaded runtimes in Node,
and the reference, Datalab's `indra-ocr` package on ONNX Runtime. Everything here was measured on
2026-10-22 on the machine below; nothing is quoted from elsewhere.

The measure is the time from an image to its page: the skew search and the orientation network on
a preview, the line detector, the script and math classifiers, the recognizers batch by batch (and
the language models on unsure lines), the math reader on lines with math, and the reading-order
network, layout blocks and Markdown.

## The machine

| | |
|---|---|
| Computer | Apple M4 Pro, 14 cores (10 performance, 4 efficiency), 24 GB |
| OS | macOS 27.0.1 (Darwin 27.0.0) |
| Native CLI | `indra-ocr` 0.1.2, Rust 1.99.0, release build |
| Browser | Chrome headless shell (Playwright), Node 22.16.0 |
| Reference | indra-ocr 1.1.2, onnxruntime 1.31.0, opencv-python-headless 5.0.0.93 (`benchmarks/reference.py`) |
| Bundle | f32, 155 MB: the networks 48 MB, the language models' tables 107 MB |

Other work ran on the machine throughout (1-minute load average 3–4 on 14 cores). Every number is
one run, the eight-thread ones in Chrome two (both beside each other). Rerun them with `just e2e`
(add `--threads 1`), `just smoke`, `just bench` and `just onnxruntime`.

## The pages

`fixtures/pages.json`, each read by the reference into `fixtures/references.json`; both are
Indra's own samples:

| Page | Image | Lines | Blocks | Read with |
|---|---|---:|---:|---|
| `scan`: a scanned letter in Korean and English | 1653 × 2337 | 36 | 9 | the Korean (CJK), base and world recognizers, language models |
| `equations`: a page of a mathematics paper | 1654 × 2339 | 38 | 22 | math read as LaTeX (17 lines) |

## In Chrome

`just e2e` (threaded, eight compute threads, relaxed SIMD and plain SIMD128) and `just e2e
--threads 1` (single-threaded, relaxed SIMD). The first page is read while the model loads, so its
time includes the end of the load.

| Page | Threads | Time to the page |
|---|---:|---:|
| scan | 8 | 0.61 s, 0.63 s (0.63–0.68 s on SIMD128) |
| equations | 8 | 0.53 s, 0.53 s (0.54 s on SIMD128) |
| scan | 1 | 1.67 s |
| equations | 1 | 2.07 s |

Every run made no network request after the model was ready, and the revisit loaded the bundle
from the browser's storage in 0.3 s.

## In Node

`just smoke`, single-threaded: the scan in 1.57 s, the equations in 2.21 s, on SIMD128 and relaxed
SIMD alike.

## Native, against ONNX Runtime

`indra-ocr bench` and `just onnxruntime`, both after a warm-up read.

| Page | Threads | indra-ocr | Reference |
|---|---:|---:|---:|
| scan | 8 | 0.25 s | 0.23 s |
| equations | 8 | 0.43 s | 0.46 s |
| scan | 1 | 1.11 s | 0.58 s |
| equations | 1 | 1.86 s | 0.87 s |

On eight threads the two are even. On one, ONNX Runtime is about twice as fast: its convolutions
and matrix products are f32 kernels tuned for the CPU (MLAS), while fusor-ai's f32 path is written
for WebAssembly SIMD first and this model's networks are small and narrow.

## Agreement with the reference

Both pages read exactly as the reference reads them, in Chrome, in Node and natively: every
line's text, box and confidence, every block and the Markdown. `just parity` checks every network
call and pipeline step against the reference's tensors: every image the networks read (the
detector's input, the orientation thumbnail, every line crop) is bit-exact, because the image steps
reproduce OpenCV's arm64 build, and every network's outputs agree within 2·10⁻⁴.

## Download

The 155 MB bundle against the checkpoint's 92 MB: the networks are f32 as in the checkpoint (48 MB),
and the language models are kept as sorted tables ready to search (107 MB) where the checkpoint
deflates them (46 MB).
