# AGENTS.md

Rules for coding agents in this repository that are its own. fusor-ai's
`vendor/fusor-ai/AGENTS.md` is the standard and binds here too (`CLAUDE.md`
imports both); this file adds what is particular to this product.

indra-ocr-wasm is Datalab's Indra OCR (datalab-to/indra-ocr): the model, a
pipeline of small networks (a line detector, a page-orientation classifier,
script and math classifiers, CTC line recognizers with n-gram language models, a
math reader and a reading-order network with layout heads), running in the
browser on the CPU, sealed from the network, reading an image of a page into its
lines, layout blocks and Markdown. The README describes what it does; don't
invent scope, features or options beyond the task.

## Repository

- `vendor/fusor-ai/models/indra-ocr/runtime`: the wasm-bindgen surface of the
  model (bundle parts, a page's RGBA pixels and options in, the page's JSON out),
  built as four browser runtimes.
- `vendor/fusor-ai/models/indra-ocr/cli`: the native CLI `indra-ocr`
  (`compile`, `read`, `bench`, `parity`).
- `app/`: the page, a [fusor](https://github.com/fusor-rs/fusor) app with its own
  Cargo workspace (fusor pins wasm-bindgen 0.2.117, the runtime 0.2.129): the
  image, the options, reading and the live measurements as Rust signals in
  `src/app.rs`, the Markdown drawn as headings, text, formulas and tables in
  `src/markdown.rs`, the template in `web/index.html`, the engine bridge, the
  image decoding and the page's boxes drawn over the preview in `web/app.js`,
  and in `engine/` the engine worker, beside the runtimes `just runtimes`
  builds.
- `scripts/`: the checkpoint and image downloads (pinned, sha256-checked), the
  Node smoke test and the agreement with the reference; `e2e/run.mjs`: the
  headless Chrome run; `fixtures/`: the pages and the reference's JSON of each;
  `benchmarks/reference.py`: the reference's speed.
- [fusor-ai](https://github.com/fusor-rs/fusor-ai): the model (`indra-ocr`),
  its compiler (`indra-ocr-compile`), the kernels, blocks and bundle format come
  from `vendor/fusor-ai`, a git submodule pinned to a commit pushed to fusor-ai,
  with the browser loader and compute pool, the runtime build, the import check
  and the dev server.

What belongs where: model code (the networks' wiring and tensor names, the
image steps, the decoding, the reading order and the layout) and anything
another model could reuse go to fusor-ai, following its `AGENTS.md`. This
repository holds what turns the model into an app: the page, the checks against
the reference and the deployment. A change to fusor-ai lands there first; this
repository never patches it.

Constraints that hold everywhere:

- fusor-ai comes only from `vendor/fusor-ai`, never a crates.io version, a
  `[patch]` table or a sibling checkout. The submodule moves to another pushed
  fusor-ai commit in a change of its own, with the parity results and the e2e
  result it gave. No copies of fusor-ai code here.
- The reference is Datalab's `indra-ocr` package at the version the justfile
  pins, on ONNX Runtime with one thread, reading images with `--mode ocr`. Its
  image steps are OpenCV's on arm64 (`media::opencv`); a page read whole must
  give the reference's JSON.
- The page owns every capability: decoding the images (files, drops or pastes)
  to RGBA pixels, caching the bundle (OPFS) and measuring itself. The runtime
  receives the pixels and options and does no I/O. Nothing is fetched while
  reading.
- The weights are under Datalab's modified OpenRAIL-M license: the page and the
  README credit Datalab with links, and the bundle ships the license beside it
  (`model/LICENSE`). Don't remove it.

## Checks

A change is done only when this passes:

```sh
just check
```

It runs formatting and Clippy on the page and syntax checks of the scripts;
fusor-ai's `just check` covers the runtime and the CLI. A change to the runtime,
the page, the CLI or fusor-ai also needs `just runtimes`, `just smoke`, `just
parity` (the same results as before unless the change is meant to move them) and
`just e2e`, which must read every fixed page as the reference does, as many
lines with the text's character error rate at most 0.01; a change meant to move
speed also needs `just bench`. The model is checked network by network and step
by step against the reference goldens by `indra-ocr parity`; extend it rather
than adding a parallel harness.

## Measuring

A speed change is judged on the page (`web-threaded-relaxed`, 8 workers, headless
Chrome through `just e2e`): the time from Read to the page. Medians of at least
three fresh-profile runs with their spread and the machine's load average; a
change smaller than the spread is not a result, and nothing else runs on the
machine during a measurement. `indra-ocr bench` explains where the time went,
stage by stage. Numerically every accepted change is bit-exact unless it is meant
to move the numbers: the fixed pages get the same JSON, `just parity` prints the
same digits. fusor-ai's `knowledge.md` records what the measurements taught
about V8, the kernels and their stores.

## Commits

- Don't commit, push, tag, release or deploy unless asked.
- Git runs as `andreespirela` here: the local config pins the author, the
  credential user and the remote URL, as in the other fusor repositories. Don't
  change it, and clone fusor repositories with
  `https://andreespirela@github.com/fusor-rs/<name>.git`.
