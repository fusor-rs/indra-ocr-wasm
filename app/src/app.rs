use fusor::{JsInputs, Signal, signal};
use serde::{Deserialize, Serialize};

use crate::markdown::{Block, blocks};

/// The runtime's options: `indra_ocr::session::Options`.
#[derive(Serialize)]
struct Options {
    math: bool,
}

/// The image chosen, once decoded.
#[derive(Clone, PartialEq, Deserialize)]
struct Picture {
    name: String,
    width: u32,
    height: u32,
}

impl Picture {
    fn summary(&self) -> String {
        format!("{} · {}×{}", self.name, self.width, self.height)
    }
}

// The runtime build and the bundle, once loaded.
#[derive(Clone, PartialEq, Deserialize)]
struct Loaded {
    build: String,
    threads: u32,
    bundle_bytes: f64,
    load_ms: f64,
    // Every part came from this device's storage.
    from_device: bool,
}

impl Loaded {
    fn summary(&self) -> String {
        let threads = match self.threads {
            1 => "1 thread".to_string(),
            count => format!("{count} threads"),
        };
        let source = if self.from_device {
            "model loaded from this device"
        } else {
            "model downloaded"
        };
        format!(
            "{} · {threads} · {:.0} MB bundle · {source} in {:.1} s",
            self.build,
            self.bundle_bytes / 1e6,
            self.load_ms / 1000.0
        )
    }
}

/// A line of the page as read: its text and confidence.
#[derive(Clone, PartialEq, Deserialize)]
struct Line {
    text: String,
    conf: f64,
}

/// A line as the page lists it.
#[derive(Clone, PartialEq)]
struct Listed {
    index: usize,
    text: String,
    confidence: String,
}

/// The page as the engine read it (the reference's JSON), what the page draws of it.
#[derive(Clone, PartialEq, Deserialize)]
struct Page {
    rotation: u32,
    skew: f64,
    lines: Vec<Line>,
    blocks: Vec<serde_json::Value>,
    markdown: String,
}

/// What `web/app.js` measures while a page is read: the time from Read to the page, the batches
/// of lines read of those planned, and the network requests made since the model loaded.
#[derive(Clone, Default, PartialEq, Deserialize)]
struct Metrics {
    time_ms: Option<f64>,
    batches: u32,
    planned: u32,
    requests: u32,
}

impl Metrics {
    fn time(&self) -> String {
        self.time_ms
            .map_or("–".into(), |ms| format!("{:.2} s", ms / 1000.0))
    }

    fn batches(&self) -> String {
        match self.planned {
            0 => "–".into(),
            planned => format!("{} / {planned}", self.batches),
        }
    }

    fn requests(&self) -> String {
        self.requests.to_string()
    }
}

// What `web/app.js` reports, one JSON object per `engine` event.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Report {
    Loading {
        fraction: f64,
        detail: String,
    },
    Ready {
        loaded: Loaded,
    },
    // An image was chosen and decoded.
    Picture {
        picture: Picture,
    },
    // A stage done, or a batch of lines read.
    Progress {
        status: String,
        metrics: Metrics,
    },
    // The page read, or the reading stopped (no page).
    Done {
        page: Option<Page>,
        status: String,
        metrics: Metrics,
    },
    // Something failed; the engine can take the next page.
    Error {
        message: String,
    },
    // The model could not load, or the engine died: nothing more can run in this page.
    Broken {
        message: String,
    },
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Idle,
    Reading,
    Broken,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Reading => "reading",
            Self::Broken => "broken",
        }
    }
}

#[derive(JsInputs)]
struct App {
    math: Signal<bool>,
    // `web/app.js` acts on every new count: `reads` (Read) sends the chosen image and `options`
    // to the engine, `stops` stops the reading, `copies` copies the Markdown.
    #[js]
    options: Signal<String>,
    #[js]
    reads: Signal<u32>,
    #[js]
    stops: Signal<u32>,
    #[js]
    copies: Signal<u32>,
    phase: Signal<Phase>,
    fraction: Signal<f64>,
    detail: Signal<String>,
    status: Signal<String>,
    picture: Signal<Option<Picture>>,
    page: Signal<Option<Page>>,
    metrics: Signal<Metrics>,
    measured: Signal<bool>,
    loaded: Signal<Option<Loaded>>,
}

impl App {
    fn new() -> Self {
        Self {
            math: signal(false),
            options: signal(String::new()),
            reads: signal(0),
            stops: signal(0),
            copies: signal(0),
            phase: signal(Phase::Idle),
            fraction: signal(0.0),
            detail: signal("Starting…".into()),
            status: signal("Drop, paste or choose an image of a page.".into()),
            picture: signal(None),
            page: signal(None),
            metrics: signal(Metrics::default()),
            measured: signal(false),
            loaded: signal(None),
        }
    }

    fn read(&self) {
        let options = Options {
            math: self.math.get(),
        };
        self.options
            .set(serde_json::to_string(&options).expect("plain data serializes"));
        self.page.set(None);
        self.metrics.set(Metrics::default());
        self.measured.set(true);
        self.phase.set(Phase::Reading);
        let status = if self.loaded.get().is_some() {
            "Straightening the page…"
        } else {
            "Read once the model is ready."
        };
        self.status.set(status.into());
        self.reads.update(|count| *count += 1);
    }

    fn stop(&self) {
        self.stops.update(|count| *count += 1);
    }

    fn copy(&self) {
        self.copies.update(|count| *count += 1);
    }

    fn receive(&self, event: web_sys::Event) {
        let json = fusor::js::event_detail::<String>(&event).expect("web/app.js sends strings");
        let report = serde_json::from_str(&json).expect("web/app.js sends engine reports");
        match report {
            Report::Loading { fraction, detail } => {
                self.fraction.set(fraction);
                self.detail.set(detail);
            }
            Report::Ready { loaded } => self.loaded.set(Some(loaded)),
            Report::Picture { picture } => {
                self.status.set(format!("{}: read it.", picture.name));
                self.picture.set(Some(picture));
            }
            Report::Progress { status, metrics } => {
                self.status.set(status);
                self.metrics.set(metrics);
            }
            Report::Done {
                page,
                status,
                metrics,
            } => {
                self.page.set(page);
                self.status.set(status);
                self.metrics.set(metrics);
                self.phase.set(Phase::Idle);
            }
            Report::Error { message } => {
                self.status.set(format!("error: {message}"));
                self.phase.set(Phase::Idle);
            }
            Report::Broken { message } => {
                self.detail.set(message);
                self.phase.set(Phase::Broken);
            }
        }
    }

    fn summary(&self) -> String {
        self.loaded
            .get()
            .map_or(String::new(), |loaded| loaded.summary())
    }

    fn picture_summary(&self) -> String {
        self.picture
            .get()
            .map_or(String::new(), |picture| picture.summary())
    }

    fn loading(&self) -> bool {
        self.loaded.get().is_none() && self.phase.get() != Phase::Broken
    }

    // The model never loaded: likely a bundle that does not match the runtime.
    fn failed_to_load(&self) -> bool {
        self.loaded.get().is_none()
    }

    fn idle(&self) -> bool {
        self.phase.get() == Phase::Idle
    }

    fn can_read(&self) -> bool {
        self.idle() && self.picture.get().is_some()
    }

    fn read_label(&self) -> &'static str {
        match self.phase.get() {
            Phase::Reading => "Reading…",
            _ if self.loaded.get().is_none() => "Read now, starts when ready",
            _ => "Read",
        }
    }

    fn empty(&self) -> bool {
        self.page.get().is_none()
    }

    fn markdown(&self) -> String {
        self.page.get().map_or(String::new(), |page| page.markdown)
    }

    // The Markdown as the page draws it: its headings, text, formulas and tables in order.
    fn blocks(&self) -> Vec<Block> {
        blocks(&self.markdown())
    }

    fn lines(&self) -> Vec<Listed> {
        let lines = self.page.get().map_or(Vec::new(), |page| page.lines);
        lines
            .into_iter()
            .enumerate()
            .map(|(index, line)| Listed {
                index,
                text: line.text,
                confidence: format!("{:.0}%", line.conf * 100.0),
            })
            .collect()
    }

    fn line_count(&self) -> String {
        self.page
            .get()
            .map_or("–".into(), |page| page.lines.len().to_string())
    }

    fn block_count(&self) -> String {
        self.page
            .get()
            .map_or("–".into(), |page| page.blocks.len().to_string())
    }

    fn orientation(&self) -> String {
        self.page.get().map_or("–".into(), |page| {
            format!("{}° · {:+.1}°", page.rotation, page.skew)
        })
    }
}

fusor::template!("web/index.html");
