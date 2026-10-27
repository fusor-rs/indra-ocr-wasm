# Datalab's Indra OCR in the browser. `just ui` builds everything, the model included, and serves
# the page; `just build` then `just serve` do the same in two steps.

import 'vendor/fusor-ai/just/product.just'

name := "indra-ocr"
checkpoint := env_var_or_default("CHECKPOINT", ".cache/checkpoint")
goldens := "fixtures/goldens"
images := "fixtures/images"
# The reference the goldens come from (vendor/fusor-ai/models/indra-ocr/reference): Datalab's
# indra-ocr package on ONNX Runtime, pinned.
reference := "uv run --no-project --python 3.12 --with indra-ocr==1.1.2 --with onnxruntime==1.31.0 --with numpy==2.5.4 --with opencv-python-headless==5.0.0.93 --with safetensors==0.8.0 python"

host:
    bash -n scripts/fetch-checkpoint.sh
    bash -n scripts/fetch-images.sh
    python3 -m py_compile benchmarks/reference.py
    node --check app/web/app.js
    node --check app/engine/worker.js
    node --check scripts/node-smoke.mjs
    node --check scripts/agreement.mjs
    node --check e2e/run.mjs
    node --check scripts/render-images.mjs

# Downloads the pinned checkpoint (92 MB, sha256-checked) unless it is already there, then
# compiles the bundle (a second) with the model's license beside it.
model: native
    scripts/fetch-checkpoint.sh {{checkpoint}}
    rm -rf {{model}}
    target/release/indra-ocr compile {{checkpoint}} {{model}}
    cp {{checkpoint}}/LICENSE {{model}}/LICENSE

# Try it on localhost. The bundle and the runtimes are rebuilt every time (incremental, seconds
# when nothing changed), so they always come from the same fusor-ai.
ui: model runtimes page serve

# The fixed pages the checks read: Indra's own sample images at a pinned commit (sha256-checked),
# each with its options in fixtures/pages.json.
images:
    scripts/fetch-images.sh {{images}}

# Every network call and pipeline step against the reference's goldens
# (vendor/fusor-ai/models/indra-ocr/reference), and each page read whole against the reference's.
parity: model
    test -f {{goldens}}/golden_f32_scan.safetensors || just goldens
    target/release/indra-ocr parity {{model}} {{goldens}}/golden_f32_scan.safetensors {{goldens}}/golden_f32_equations.safetensors

# Golden tensors of every page and the reference's JSON of each (fixtures/references.json), from the
# reference on the CPU with one thread.
goldens: images
    scripts/fetch-checkpoint.sh {{checkpoint}}
    {{reference}} {{fusor_ai}}/models/indra-ocr/reference/dump.py {{checkpoint}} fixtures/pages.json {{images}} {{goldens}} fixtures/references.json

bench: native images
    target/release/indra-ocr bench {{model}} fixtures/pages.json scan --images {{images}}

# The reference's speed on this CPU (benchmarks/reference.py), on the same pages as `just e2e`,
# with as many threads as the page's compute workers.
onnxruntime threads="8": images
    scripts/fetch-checkpoint.sh {{checkpoint}}
    {{reference}} benchmarks/reference.py {{checkpoint}} fixtures/pages.json {{images}} {{threads}}

# Its tensor parts are named by their BLAKE3, so /model/parts/ can be cached as immutable.
# The page, its four runtimes and the bundle beside it, as a static site in target/site.
site: runtimes page
    rm -rf target/site
    mkdir -p target/site
    cp -R {{app}}/dist/. target/site/
    find target/site -name '.fusor-*.json' -delete
    node scripts/site-bundle.mjs {{model}} target/site/model
    cp {{model}}/LICENSE target/site/model/

# The Space must already exist as a static Space: creating it with the upload makes a Gradio
# Space, which Hugging Face refuses for a free organization.
# The site to the Hugging Face Space fusorlabs/indra-ocr-wasm, configured by space/README.md.
deploy-space: site
    #!/usr/bin/env bash
    set -euo pipefail
    cp space/README.md target/site/
    uv run --no-project --with huggingface_hub==0.36.2 python - target/site <<'PY'
    import sys
    from huggingface_hub import HfApi

    HfApi().upload_folder(
        repo_id="fusorlabs/indra-ocr-wasm",
        repo_type="space",
        folder_path=sys.argv[1],
        delete_patterns="*",
        commit_message="Deploy indra-ocr-wasm {{version}}",
    )
    PY

# The fixed inputs the checks read.
fixtures: images
