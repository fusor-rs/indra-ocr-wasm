---
title: indra-ocr-wasm
short_description: Indra OCR, pages to Markdown in the browser, on the CPU
sdk: static
app_file: index.html
pinned: false
license: other
models:
  - datalab-to/indra-ocr
custom_headers:
  cross-origin-embedder-policy: require-corp
  cross-origin-opener-policy: same-origin
  cross-origin-resource-policy: cross-origin
---

Pages to Markdown with Datalab's [Indra OCR](https://huggingface.co/datalab-to/indra-ocr),
computed on your own CPU in the browser: the model, compiled to WebAssembly with fusor-ai,
downloads once and then runs sealed from the network, reading every line's box and text and the
page's layout off an image you pick. The model is Datalab's, under a modified OpenRAIL-M
license, which ships beside the bundle as `model/LICENSE`. The page's code is MIT; its source is
[github.com/fusor-rs/indra-ocr-wasm](https://github.com/fusor-rs/indra-ocr-wasm).
