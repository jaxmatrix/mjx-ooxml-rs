//! What the suites share: a residency that has never heard of OOXML, file-backed sinks, and the
//! address probes the OOXML suites use.
//!
//! Nothing here is production code. The two allowances at the top are what a `#[path]`-included
//! helper module always needs: each test binary compiles the whole file and uses part of it, so
//! every unused helper would warn, and `pub` here reaches nothing outside the binary, so every
//! declaration would warn again. `clippy --all-targets -- -D warnings` turns both into failures.
#![allow(dead_code, unreachable_pub)]

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use mjx_layout::{LayoutRect, PartId, SourcePath, SourceRef};
use mjx_session::{
    Applied, Committed, Invalidation, Operation, OperationKind, ResidentDocument, SessionError,
    SinkError, Value,
};

// -------------------------------------------------------------------------------------------------
// A residency that is not OOXML.
// -------------------------------------------------------------------------------------------------

/// A document of numbered parts, each holding numbered nodes with a string and a box.
///
/// **This is the point of the whole seam**, and it is why it lives in the test tree rather than in
/// the crate: `ResidentDocument` with one implementation would be a trait nothing had ever been
/// swapped for, and a claim that the machinery is document-agnostic that no build had checked. This
/// one names no package, no part name and no format crate, and every suite that measures batching,
/// undo granularity or the schedule drives it as well as the three real ones.
///
/// Its addressing is its own, as a box model's always is: part `p`, path `[node]`.
#[derive(Clone, Debug)]
pub struct PlainDocument {
    parts: Vec<PlainPart>,
    /// Every part this document has ever serialised, in order — so a suite can name *which*, not
    /// only how many.
    pub serialised: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
struct PlainPart {
    nodes: Vec<String>,
    boxes: Vec<LayoutRect>,
    dirty: bool,
}

impl PlainDocument {
    /// A document of `parts` parts, each with `nodes` empty nodes.
    #[must_use]
    pub fn new(parts: usize, nodes: usize) -> Self {
        Self {
            parts: (0..parts)
                .map(|_| PlainPart {
                    nodes: vec![String::new(); nodes],
                    boxes: vec![LayoutRect::ZERO; nodes],
                    dirty: false,
                })
                .collect(),
            serialised: Vec::new(),
        }
    }

    /// The address of node `node` in part `part`.
    #[must_use]
    pub fn address(part: u32, node: u32) -> SourceRef {
        SourceRef::node(PartId::new(part), SourcePath::new(&[node]))
    }

    /// What a node holds.
    #[must_use]
    pub fn text(&self, part: usize, node: usize) -> Option<&str> {
        self.parts.get(part)?.nodes.get(node).map(String::as_str)
    }

    /// Where a node is.
    #[must_use]
    pub fn boxed(&self, part: usize, node: usize) -> Option<LayoutRect> {
        self.parts.get(part)?.boxes.get(node).copied()
    }

    /// How many parts are waiting to be serialised.
    #[must_use]
    pub fn dirty_parts(&self) -> usize {
        self.parts.iter().filter(|part| part.dirty).count()
    }

    fn locate(&mut self, address: &SourceRef) -> Result<(usize, usize), SessionError> {
        let part = address.part().number() as usize;
        let [node] = address.path().segments() else {
            return Err(SessionError::no_such_node(format!(
                "path {:?} — a plain document addresses one node per part path",
                address.path().segments()
            )));
        };
        let node = *node as usize;
        let held = self
            .parts
            .get(part)
            .ok_or_else(|| SessionError::no_such_node(format!("part {part}")))?;
        if node >= held.nodes.len() {
            return Err(SessionError::no_such_node(format!(
                "part {part} node {node}"
            )));
        }
        Ok((part, node))
    }
}

impl ResidentDocument for PlainDocument {
    fn apply(&mut self, operation: &Operation) -> Result<Applied, SessionError> {
        let address = operation.address();
        let (part, node) = self.locate(address)?;
        let inverse = match operation.kind() {
            OperationKind::SetValue(Value::Text(text)) => {
                let was = std::mem::replace(&mut self.parts[part].nodes[node], text.to_string());
                Operation::set_value(address.clone(), Value::text(was))
            }
            OperationKind::SetValue(Value::Empty) => {
                let was = std::mem::take(&mut self.parts[part].nodes[node]);
                Operation::set_value(address.clone(), Value::text(was))
            }
            OperationKind::SetValue(other) => {
                return Err(SessionError::unsupported(format!(
                    "hold {other:?} — a plain document holds text"
                )))
            }
            OperationKind::SetBounds(rect) => {
                let was = std::mem::replace(&mut self.parts[part].boxes[node], *rect);
                Operation::set_bounds(address.clone(), was)
            }
        };
        self.parts[part].dirty = true;
        Ok(Applied {
            inverse,
            invalidation: Invalidation::at(address),
        })
    }

    fn dirty_bytes(&self) -> usize {
        self.parts
            .iter()
            .filter(|part| part.dirty)
            .map(|part| part.nodes.iter().map(String::len).sum::<usize>())
            .sum()
    }

    fn commit(&mut self) -> Result<Committed, SessionError> {
        let mut bytes = Vec::new();
        let mut serialised = 0;
        for (index, part) in self.parts.iter_mut().enumerate() {
            if part.dirty {
                serialised += 1;
                self.serialised.push(index);
                part.dirty = false;
            }
            for node in &part.nodes {
                bytes.extend_from_slice(node.as_bytes());
                bytes.push(b'\n');
            }
        }
        Ok(Committed {
            parts_serialised: serialised,
            bytes,
        })
    }
}

// -------------------------------------------------------------------------------------------------
// Sinks that reach a real filesystem, so a kill can be recovered from.
// -------------------------------------------------------------------------------------------------

/// A journal appended to a file, flushed with `sync_all`.
///
/// The library ships in-memory sinks only, because `wasm32` has no files and nothing below the
/// facade has ever touched one. This is what a desktop host would write, and it is what lets the
/// recovery suite kill a process and read back what actually reached the disk.
#[derive(Debug)]
pub struct FileJournal {
    path: PathBuf,
    handle: File,
}

impl FileJournal {
    /// Opens (creating) the journal at `path`, truncating whatever was there.
    ///
    /// # Panics
    /// If the file cannot be created. This is a test helper; a host would return an error.
    #[must_use]
    pub fn create(path: &Path) -> Self {
        let handle = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .unwrap_or_else(|error| panic!("creating {}: {error}", path.display()));
        Self {
            path: path.to_path_buf(),
            handle,
        }
    }
}

impl mjx_session::JournalSink for FileJournal {
    fn append(&mut self, record_bytes: &[u8]) -> Result<(), SinkError> {
        self.handle.write_all(record_bytes).map_err(SinkError::new)
    }

    fn flush(&mut self) -> Result<(), SinkError> {
        self.handle.flush().map_err(SinkError::new)?;
        self.handle.sync_all().map_err(SinkError::new)
    }

    fn truncate(&mut self) -> Result<(), SinkError> {
        self.handle = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(SinkError::new)?;
        Ok(())
    }
}

/// The committed document, written to a file.
#[derive(Debug)]
pub struct FileDocument {
    path: PathBuf,
}

impl FileDocument {
    /// Commits will land at `path`.
    #[must_use]
    pub fn at(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }
}

impl mjx_session::DocumentSink for FileDocument {
    fn write(&mut self, container_bytes: &[u8]) -> Result<(), SinkError> {
        std::fs::write(&self.path, container_bytes).map_err(SinkError::new)
    }
}

/// A directory this process owns, under the system temporary directory.
///
/// # Panics
/// If it cannot be created.
#[must_use]
pub fn scratch_directory(name: &str) -> PathBuf {
    let unique = format!(
        "mjx-session-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|it| it.as_nanos())
            .unwrap_or_default()
    );
    let path = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&path)
        .unwrap_or_else(|error| panic!("creating {}: {error}", path.display()));
    path
}

// -------------------------------------------------------------------------------------------------
// Probes for the OOXML suites.
// -------------------------------------------------------------------------------------------------

/// The address of the first non-empty run in a deck, as `[slide, shape…, paragraph, run]`.
///
/// Probed rather than hard-coded, because a fixture is a real file and a suite that assumed slide
/// 0's shape 0 had a run would be a suite that broke the day the corpus grew a better sample.
#[cfg(feature = "ooxml")]
#[must_use]
pub fn first_run_in_deck(deck: &mut mjx_pptx::Presentation) -> Option<SourceRef> {
    for slide in 0..deck.slide_count() {
        let shapes = deck.shape_count(slide).ok()?;
        for shape in 0..shapes {
            let Ok(paragraphs) = deck.paragraph_count(slide, shape) else {
                continue;
            };
            for paragraph in 0..paragraphs {
                let Ok(runs) = deck.run_count(slide, shape, paragraph) else {
                    continue;
                };
                for run in 0..runs {
                    if deck
                        .run_text(slide, shape, paragraph, run)
                        .is_ok_and(|text| !text.is_empty())
                    {
                        return Some(SourceRef::node(
                            PartId::new(0),
                            SourcePath::new(&[
                                slide as u32,
                                shape as u32,
                                paragraph as u32,
                                run as u32,
                            ]),
                        ));
                    }
                }
            }
        }
    }
    None
}

/// The address of the first run of a **second or later paragraph** in a deck.
///
/// It exists because of the one number in `PresentationSession` a first-paragraph address can never
/// exercise: `mjx-pptx` counts runs *per paragraph* for reading and *flattened over the whole shape*
/// for writing, so a residency addressed by `[slide, shape..., paragraph, run]` has to convert
/// between the two — and for paragraph zero that conversion is the identity, so every gate that
/// used the first run with text in `sample.pptx` ran the loop zero times and would have passed with
/// the conversion deleted.
#[cfg(feature = "ooxml")]
#[must_use]
pub fn first_run_after_the_first_paragraph(deck: &mut mjx_pptx::Presentation) -> Option<SourceRef> {
    for slide in 0..deck.slide_count() {
        let shapes = deck.shape_count(slide).ok()?;
        for shape in 0..shapes {
            let Ok(paragraphs) = deck.paragraph_count(slide, shape) else {
                continue;
            };
            for paragraph in 1..paragraphs {
                let Ok(runs) = deck.run_count(slide, shape, paragraph) else {
                    continue;
                };
                for run in 0..runs {
                    if deck
                        .run_text(slide, shape, paragraph, run)
                        .is_ok_and(|text| !text.is_empty())
                    {
                        return Some(SourceRef::node(
                            PartId::new(0),
                            SourcePath::new(&[
                                slide as u32,
                                shape as u32,
                                paragraph as u32,
                                run as u32,
                            ]),
                        ));
                    }
                }
            }
        }
    }
    None
}

/// The address of the first run in a document's main story, as `[block, run]`.
#[cfg(feature = "ooxml")]
#[must_use]
pub fn first_run_in_document(document: &mut mjx_docx::Document) -> Option<SourceRef> {
    let blocks = document.paragraph_count().ok()?;
    for block in 0..blocks {
        let Ok(runs) = document.run_count(block) else {
            continue;
        };
        for run in 0..runs {
            if document
                .run_text(block, run)
                .is_ok_and(|text| !text.is_empty())
            {
                return Some(SourceRef::node(
                    PartId::PRIMARY,
                    SourcePath::new(&[block as u32, run as u32]),
                ));
            }
        }
    }
    None
}

/// The address of one cell: sheet `sheet`, row `row`, column `column`.
#[cfg(feature = "ooxml")]
#[must_use]
pub fn cell_address(sheet: u32, row: u32, column: u32) -> SourceRef {
    SourceRef::node(PartId::new(sheet), SourcePath::new(&[row, column]))
}

/// A fixture's bytes.
#[must_use]
pub fn fixture(name: &str) -> Vec<u8> {
    mjx_fixtures::fixture(name)
}
