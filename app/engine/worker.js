// The engine's home: a module Worker that loads the model and reads pages, off the page's main
// thread.
//
// Load: fusor-ai's worker kit (`dist/host/worker.js`) starts the best runtime build and streams
// every part of the bundle into it; the runtime checks each part's size and BLAKE3.
//
// Read: the page's RGBA pixels and options start a reading (`start()`), then one `step()` per
// macrotask — straightening the page, finding its lines, sorting them by script, then one batch
// of lines at a time — each posted to the page, so a stop request is seen between steps. A page
// read before the model is ready waits for it.
//
// After the model is loaded nothing else is fetched: reading is entirely local.

import { engine, load, post, ready, requestsSinceReady, serve, yieldTask } from './dist/host/worker.js';

let stoppedJob = null;

serve({
  load: ({ modelUrl, threads, build }) =>
    load({
      modelUrl,
      threads,
      build,
      glueName: 'indra_ocr_runtime',
      cacheName: 'indra-ocr-parts',
      distUrl: new URL('./dist/', import.meta.url),
      onRuntime: (started) => post({ type: 'runtime', build: started.build.name, threads: started.threads }),
      onReady: (progress) => post({ type: 'ready', ...progress }),
    }),
  read,
  stop: ({ job }) => (stoppedJob = job),
});

async function read({ job, rgba, width, height, options }) {
  await ready;
  const stopped = { type: 'done', job, stopped: true, page: null };
  if (stoppedJob === job) return post(stopped);
  const { rt } = engine;
  rt.start(rgba, width, height, options);
  for (;;) {
    await yieldTask();
    if (stoppedJob === job) {
      rt.cancel();
      return post({ ...stopped, networkRequests: requestsSinceReady() });
    }
    const progress = JSON.parse(rt.step());
    const message = { job, ...progress, networkRequests: requestsSinceReady() };
    if (progress.page) return post({ ...message, type: 'done', stopped: false });
    post({ ...message, type: 'step' });
  }
}
