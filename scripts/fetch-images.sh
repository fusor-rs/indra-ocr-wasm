#!/usr/bin/env bash
# Downloads the fixed pages the checks read into $1, from Indra's own samples (datalab-to/indra,
# Apache-2.0) at a pinned commit, each checked by sha256 (files already present are only checked).
# Each page goes with its options in `fixtures/pages.json`.
set -euo pipefail
DIR="$1"
REPO="https://raw.githubusercontent.com/datalab-to/indra/0e96b93c6810f17cf8352acbb4487cc9e45b66d5"
FILES="
5580833d8920e9f0e74605b1045bc226944cd137a3e3426ba766782e7aa9e171  examples/samples/scan.png
2eb7c46bc9c84d959d41e3f00f5244ec1789a54335614645736b97157ad5acff  examples/samples/equations.png
"
mkdir -p "$DIR"
while read -r _ path; do
  file="$(basename "$path")"
  if [ -z "$path" ] || [ -s "$DIR/$file" ]; then continue; fi
  curl -fsSL --retry 3 -o "$DIR/$file.part" "$REPO/$path"
  mv "$DIR/$file.part" "$DIR/$file"
done <<< "$FILES"
(cd "$DIR" && awk 'NF { n = split($2, parts, "/"); print $1 "  " parts[n] }' <<< "$FILES" | shasum -a 256 -c --quiet -)
