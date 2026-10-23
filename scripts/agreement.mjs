// How a page read agrees with the reference's (fixtures/references.json): its lines' count, the
// lines whose text differs, the character error rate of the whole text (edit distance over the
// reference's length), the largest difference of a line's box (when the page's lines have boxes),
// and whether the Markdown is the same.

export function agreement(reference, page) {
  const ours = page.lines ?? [];
  const wanted = reference.lines;
  const differing = wanted.filter((line, index) => ours[index]?.text !== line.text).length;
  const text = (lines) => lines.map((line) => line.text).join('\n');
  const characters = [...text(wanted)].length;
  const errorRate = distance([...text(ours)], [...text(wanted)]) / Math.max(characters, 1);
  let maxBox = ours.some((line) => line.box) ? 0 : null;
  wanted.forEach((line, index) => {
    const box = ours[index]?.box;
    if (maxBox === null) return;
    line.box.forEach((value, at) => (maxBox = Math.max(maxBox, Math.abs((box?.[at] ?? Infinity) - value))));
  });
  return { lines: ours.length, wanted: wanted.length, differing, errorRate, maxBox, markdown: page.markdown === reference.markdown };
}

/** The Levenshtein distance of two arrays of characters. */
function distance(left, right) {
  let previous = Array.from({ length: right.length + 1 }, (_, index) => index);
  for (let i = 1; i <= left.length; i++) {
    const current = [i];
    for (let j = 1; j <= right.length; j++) {
      const substitution = previous[j - 1] + (left[i - 1] === right[j - 1] ? 0 : 1);
      current.push(Math.min(previous[j] + 1, current[j - 1] + 1, substitution));
    }
    previous = current;
  }
  return previous[right.length];
}
