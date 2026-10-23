//! A page's Markdown as the page draws it, block by block. Indra writes its blocks apart, a blank
//! line between them: a section header after `## `, a table as HTML (`<table>`, `<tr>`, `<th>` or
//! `<td>` with `colspan`), display formulas each a `$$…$$` line, and paragraphs and lists with
//! formulas inline between dollar signs. A table's cells are read as text, never as markup: the
//! page never injects HTML.

const HEADING: &str = "## ";
const TABLE_START: &str = "<table";
const ROW_START: &str = "<tr";
const ROW_END: &str = "</tr>";
const CELL_STARTS: [&str; 4] = ["<td>", "<td ", "<th>", "<th "];
const DISPLAY_MATH: &str = "$$";
const BLOCK_SEPARATOR: &str = "\n\n";

/// A cell the page draws: its text, and the columns it spans.
#[derive(Clone, PartialEq)]
pub(crate) struct Cell {
    pub(crate) column: usize,
    pub(crate) text: String,
    pub(crate) columns: usize,
}

#[derive(Clone, PartialEq)]
pub(crate) struct Row {
    pub(crate) index: usize,
    pub(crate) cells: Vec<Cell>,
}

#[derive(Clone, PartialEq)]
pub(crate) enum Kind {
    Heading,
    Text,
    /// A displayed formula: its LaTeX, delimiters dropped.
    Formula,
    Table(Vec<Row>),
}

/// A piece of a text block: text as written, or a formula inline (its LaTeX).
#[derive(Clone, PartialEq)]
pub(crate) struct Segment {
    pub(crate) index: usize,
    pub(crate) text: String,
    pub(crate) math: bool,
}

/// One block of the page: a heading, a paragraph or list, a displayed formula or a table.
#[derive(Clone, PartialEq)]
pub(crate) struct Block {
    pub(crate) index: usize,
    pub(crate) kind: Kind,
    pub(crate) text: String,
}

impl Block {
    pub(crate) fn is_heading(&self) -> bool {
        self.kind == Kind::Heading
    }

    pub(crate) fn is_table(&self) -> bool {
        matches!(self.kind, Kind::Table(_))
    }

    pub(crate) fn is_formula(&self) -> bool {
        self.kind == Kind::Formula
    }

    pub(crate) fn is_text(&self) -> bool {
        self.kind == Kind::Text
    }

    /// A text block's text and inline formulas, in order.
    pub(crate) fn segments(&self) -> Vec<Segment> {
        let mut segments = Vec::new();
        let mut rest = self.text.as_str();
        while let Some((start, end)) = inline_math(rest) {
            segment(&mut segments, &rest[..start], false);
            segment(&mut segments, &rest[start + 1..end], true);
            rest = &rest[end + 1..];
        }
        segment(&mut segments, rest, false);
        segments
    }

    pub(crate) fn rows(&self) -> Vec<Row> {
        match &self.kind {
            Kind::Table(rows) => rows.clone(),
            Kind::Heading | Kind::Text | Kind::Formula => Vec::new(),
        }
    }
}

/// The blocks of a page's Markdown, in order.
pub(crate) fn blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    for part in markdown
        .split(BLOCK_SEPARATOR)
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some(heading) = part.strip_prefix(HEADING) {
            push(&mut blocks, Kind::Heading, heading);
        } else if part.starts_with(TABLE_START) {
            let rows = rows(part);
            let kind = if rows.is_empty() {
                Kind::Text
            } else {
                Kind::Table(rows)
            };
            push(&mut blocks, kind, part);
        } else if part.lines().all(|line| displayed(line).is_some()) {
            for formula in part.lines().filter_map(displayed) {
                push(&mut blocks, Kind::Formula, formula);
            }
        } else {
            push(&mut blocks, Kind::Text, part);
        }
    }
    blocks
}

fn push(blocks: &mut Vec<Block>, kind: Kind, text: &str) {
    let index = blocks.len();
    blocks.push(Block {
        index,
        kind,
        text: text.into(),
    });
}

fn segment(segments: &mut Vec<Segment>, text: &str, math: bool) {
    if text.is_empty() {
        return;
    }
    segments.push(Segment {
        index: segments.len(),
        text: text.into(),
        math,
    });
}

/// The byte offsets of the dollar signs around the first inline formula, by Pandoc's rule: the
/// opening one is followed by a non-space, the closing one preceded by a non-space and not
/// followed by a digit (so that "$5 and $10" stays text). A formula does not span lines.
fn inline_math(text: &str) -> Option<(usize, usize)> {
    let dollars: Vec<usize> = text.match_indices('$').map(|(at, _)| at).collect();
    let after = |at: usize| text[at + 1..].chars().next();
    let before = |at: usize| text[..at].chars().next_back();
    let opens = |at: usize| after(at).is_some_and(|next| !next.is_whitespace() && next != '$');
    let closes = |at: usize| {
        before(at).is_some_and(|previous| !previous.is_whitespace())
            && !after(at).is_some_and(|next| next.is_ascii_digit())
    };
    dollars.iter().enumerate().find_map(|(index, &start)| {
        if !opens(start) {
            return None;
        }
        let end = dollars[index + 1..]
            .iter()
            .copied()
            .find(|&end| end > start + 1 && closes(end))?;
        (!text[start..end].contains('\n')).then_some((start, end))
    })
}

/// The LaTeX of a line that is one displayed formula.
fn displayed(line: &str) -> Option<&str> {
    let tex = line
        .trim()
        .strip_prefix(DISPLAY_MATH)?
        .strip_suffix(DISPLAY_MATH)?;
    (!tex.is_empty() && !tex.contains(DISPLAY_MATH)).then_some(tex)
}

/// The rows of an HTML table, each with its cells' text and spans.
fn rows(html: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find(ROW_START) {
        let row = &rest[start..];
        let end = row
            .find(ROW_END)
            .map_or(row.len(), |end| end + ROW_END.len());
        let cells = cells(&row[..end]);
        if !cells.is_empty() {
            rows.push(Row {
                index: rows.len(),
                cells,
            });
        }
        rest = &row[end..];
    }
    rows
}

fn cells(row: &str) -> Vec<Cell> {
    let mut cells = Vec::new();
    let mut rest = row;
    while let Some(start) = CELL_STARTS.iter().filter_map(|tag| rest.find(tag)).min() {
        let cell = &rest[start..];
        let Some(open_end) = cell.find('>') else {
            break;
        };
        let content = &cell[open_end + 1..];
        let close = content.find("</t").unwrap_or(content.len());
        cells.push(Cell {
            column: cells.len(),
            text: plain(&content[..close]),
            columns: colspan(&cell[..open_end]),
        });
        rest = &content[close..];
        rest = rest.find('>').map_or("", |end| &rest[end + 1..]);
    }
    cells
}

/// The columns an opening cell tag spans: its `colspan`, 1 without one.
fn colspan(tag: &str) -> usize {
    let Some(at) = tag.find("colspan=\"") else {
        return 1;
    };
    let digits: String = tag[at + "colspan=\"".len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    // Indra writes a number; anything else spans one cell, as a browser reads it.
    digits.parse().unwrap_or(1).max(1)
}

/// A cell's text, the entities Indra escapes (`&`, `<`, `>`) decoded.
fn plain(content: &str) -> String {
    content
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .trim()
        .into()
}
