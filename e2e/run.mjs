// End-to-end run in headless Chrome, in a fresh profile, of the page reading the fixed pages.
//
// 1. Open the page, choose the scanned letter (`scan.png`) and press Read while the model is still
//    loading: it is read the moment the model is ready. Check its lines read as the reference
//    reads them (as many lines, the text's character error rate at most --max-error-rate), the
//    Markdown drawn, and that nothing was requested after the model was ready.
// 2. Read the page of equations (`equations.png`) with math read as LaTeX, and check the same.
// 3. Reload: the revisit must load every part from OPFS.
//
//   PORT=8787 node e2e/run.mjs [--threads 8] [--build web-threaded] [--max-error-rate 0.01]
//   node e2e/run.mjs --url https://example.org    # a deployed page, no local server
//
// Results (JSON, pages, screenshots) go to e2e/results/<timestamp>-*, or to `--out <dir>`.

import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { fetched, flags, log, run, STAMP } from '../vendor/fusor-ai/packages/e2e/kit.mjs';
import { agreement } from '../scripts/agreement.mjs';

const ROOT = fileURLToPath(new URL('..', import.meta.url));
const values = flags({
  'max-error-rate': { type: 'string', default: '0.01' },
});
const DEADLINE_MS = 10 * 60 * 1000;

const pages = JSON.parse(readFileSync(join(ROOT, 'fixtures/pages.json'), 'utf8'));
const references = JSON.parse(readFileSync(join(ROOT, 'fixtures/references.json'), 'utf8'));
const maxErrorRate = Number(values['max-error-rate']);

// The page's state machine is `main[data-phase]`: idle, reading, broken.
const idle = () => /^(idle|broken)$/.test(document.querySelector('main')?.dataset.phase ?? '');
const loadedOrBroken = () => document.querySelector('main')?.dataset.loaded === 'true' || document.querySelector('main')?.dataset.phase === 'broken';

/** The page read (its lines and Markdown), the blocks drawn and the metric tiles, as the page
 * shows them. */
async function readPage(page) {
  const tiles = ['time', 'line-count', 'block-count', 'batches', 'requests'];
  const shown = await Promise.all(tiles.map((id) => page.textContent(`#${id}`)));
  return {
    markdown: await page.textContent('#text'),
    lines: await page.$$eval('#lines .line-text', (lines) => lines.map((line) => line.textContent)),
    drawn: await page.$$eval('#answer .block', (blocks) => blocks.length),
    status: await page.textContent('#status'),
    ...Object.fromEntries(tiles.map((id, i) => [id, shown[i]])),
  };
}

/** Chooses page `name`'s image, sets its options and presses Read; resolves once it is read,
 * with the requests made after the model was ready. */
async function read(page, name, requestsSeen) {
  const { image, math = false } = pages[name];
  await page.setInputFiles('#file', join(ROOT, 'fixtures/images', image));
  await page.waitForFunction(() => document.querySelector('#pick')?.dataset.empty === 'false', null, { timeout: DEADLINE_MS });
  await page.setChecked('#math', math);
  const t0 = Date.now();
  await page.click('#read');
  await page.waitForFunction(loadedOrBroken, null, { timeout: DEADLINE_MS, polling: 250 });
  const mark = requestsSeen.length;
  await page.waitForFunction(idle, null, { timeout: DEADLINE_MS, polling: 250 });
  return { ...(await readPage(page)), wallMs: Date.now() - t0, fetched: fetched(requestsSeen, mark) };
}

function checkPage(check, name, readOut) {
  check(readOut.status === 'Done.', `${name}: ${readOut['line-count']} lines, ${readOut['block-count']} blocks, ${readOut.batches} batches in ${readOut.time}; ${(readOut.wallMs / 1000).toFixed(1)} s in all`);
  const result = agreement(references[name], { lines: readOut.lines.map((text) => ({ text })), markdown: readOut.markdown });
  Object.assign(readOut, { differing: result.differing, errorRate: result.errorRate, sameMarkdown: result.markdown });
  check(result.lines === result.wanted, `${name}: ${result.lines} lines, the reference ${result.wanted}`);
  check(result.errorRate <= maxErrorRate, `${name}: character error rate ${result.errorRate.toFixed(4)} against the reference (at most ${maxErrorRate}), ${result.differing} lines differ, Markdown ${result.markdown ? 'the same' : 'differs'}`);
  check(readOut.drawn > 0, `${name}: ${readOut.drawn} blocks drawn`);
  check(readOut.fetched.length === 0 && readOut.requests === '0', `${name}: requests while reading: ${readOut.fetched.length} seen, page counts ${readOut.requests}`);
}

const launch = {
  root: ROOT,
  name: 'indra-ocr',
  values,
  fields: { cases: {} },
};
await run(launch, async ({ page, requests: requestsSeen, query, base, runDir, results, check, revisit }) => {
  await page.goto(`${base}/${query}`);
  await page.waitForSelector('main[data-phase]');
  results.readWhileLoading = (await page.getAttribute('main', 'data-loaded')) !== 'true';
  log(`page open, the model ${results.readWhileLoading ? 'still loading' : 'loaded'}`);
  for (const name of Object.keys(pages)) {
    const readOut = await read(page, name, requestsSeen);
    results.runtime ??= await page.textContent('#runtime');
    checkPage(check, name, readOut);
    results.cases[name] = readOut;
    const written = { markdown: readOut.markdown, lines: readOut.lines };
    writeFileSync(join(runDir, `${STAMP}-${name}.page.json`), `${JSON.stringify(written, null, 2)}\n`);
    await page.screenshot({ path: join(runDir, `${STAMP}-${name}.png`), fullPage: true });
  }

  await revisit(loadedOrBroken, DEADLINE_MS);
});
