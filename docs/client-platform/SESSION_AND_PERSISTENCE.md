# The session — recording operations, and committing them in batches

The specification of `mjx-session` (unit A7). Every mutation is recorded as an operation; operations
are coalesced; and the document is **committed on a schedule rather than on every operation**, so a
burst of editing does not turn into a burst of serialisation.

---

## 1 · Three layers that must not be conflated

The instruction — *record all operations, commit at a frequency* — is right, and the way it goes
wrong in practice is by batching one layer too many. There are three, and only the third is batched.

| Layer | When | Why |
|---|---|---|
| **1 · Record** — append the operation to the journal | **immediately**, synchronously | the journal is undo/redo and crash recovery; an operation that is not recorded the instant it happens can be lost |
| **2 · Apply** — mutate the in-memory model | **immediately** | the keystroke-to-repaint budget is 30 ms; a pending queue means the user types and nothing appears |
| **3 · Commit** — serialise dirty parts to XML and write bytes | **batched, on a schedule** | this is the expensive layer, and the only one worth deferring |

Batching layer 1 loses work. Batching layer 2 makes the editor feel broken. Batching layer 3 is pure
win, and it is where all the cost actually is: serialising a worksheet part costs orders of magnitude
more than mutating one cell in the model.

---

## 2 · Coalescing is the real saving, not the timer

A timer alone still serialises a part once per interval whether one character changed or ten
thousand. The saving comes from **collapsing operations before the commit ever runs**.

- **Typing** — 20 keystrokes into one run produce 20 journal entries and **one** dirty run.
- **Dragging** — 100 pointer moves produce one final transform; the intermediate positions were never
  document state, only render state.
- **Sliders and steppers** — continuous adjustment collapses to its settled value.
- **Repeated writes to the same address** — last-write-wins within a batch window.

So the commit does **not replay the journal**. It walks a **dirty set** and serialises each dirty part
exactly once, no matter how many operations touched it. That granularity falls out of the existing
architecture for free: parts are already the unit of copy-on-write, and untouched parts already
re-emit verbatim.

### One refinement to the documented copy-on-write rule

`CLAUDE.md` states the contract as: *on first edit, serialize from the model and drop raw bytes.*
Under batched commit that becomes two separate moments:

> On first edit, **drop the raw bytes and mark the part dirty** — the model is now authoritative.
> **Serialise at commit**, once, however many edits have accumulated.

The round-trip guarantee is unaffected: an untouched part still re-emits verbatim, byte for byte,
because it was never marked dirty. Only the *timing* of the serialisation moves. `PLAN.md` and
`CLAUDE.md` should carry this refinement when A7 lands, so the documented contract and the
implementation do not diverge.

---

## 3 · When a commit fires

A pure interval is the wrong policy on its own — it is either too slow to be safe or too frequent to
be worth doing. Commit when **any** of these becomes true:

| Trigger | Rationale |
|---|---|
| **Idle for ~2 s** | the cheapest possible moment; the user has paused |
| **Max age ~30 s since last commit** | bounds worst-case exposure during continuous editing |
| **Dirty-byte threshold** | a bulk operation (paste 10,000 rows) should not wait for a clock |
| **Explicit save** | always immediate and synchronous-to-completion |
| **Focus loss / app backgrounded** | **mandatory** — iOS terminates backgrounded apps without warning, and this is the single most likely way to lose work on the mobile target |
| **Before any consistency-requiring operation** | export, print, external hand-off, close |

**Idle-first, and never mid-interaction.** A commit that lands during a drag or a fling costs a
dropped frame, which is exactly the thing the whole render architecture is built to avoid. The
scheduler defers a due commit while a gesture is in flight and takes the first idle moment after it.

**Off the frame path.** Serialisation runs on a worker thread natively. On single-threaded `wasm32`
it is chunked across frames with a time budget per slice, because there is no thread to move it to
and blocking the frame is not an option.

---

## 4 · Batching must not mean losing work

The obvious objection to deferring commits is that a crash loses everything since the last one. The
answer is the standard one, and it is why layer 1 is never batched:

**Two artefacts, two frequencies.**

- **The journal** — append-only, small, sequential writes, flushed **frequently** (sub-second, or on
  every operation where the platform makes that cheap). Writing an operation record is orders of
  magnitude cheaper than serialising a part, so this costs almost nothing.
- **The document** — the full committed state, written **infrequently**, per §3.

**Recovery is then: the last committed document, plus a replay of the journal tail after it.** The
journal is truncated at each successful commit. Nothing is exposed beyond the last flushed operation,
while the expensive work still happens on a schedule — which is precisely the outcome asked for.

An unclean prior shutdown is detected on open, and recovery is offered rather than performed
silently.

---

## 5 · Undo granularity is a separate concern

Easy and common bug: using the persistence coalescing window as the undo unit, so undo jumps back by
however much happened to be batched.

They are independent. **Undo units are semantic** — a word of typing, one drag, one formatting
command — determined by operation kind, target and a short idle boundary. **Persistence batches are
economic** — whatever accumulated since the last commit. A single undo unit may span several commits;
a single commit may contain many undo units. Neither constrains the other, and the journal carries
both markings.

---

## 6 · Interaction with the rest of the architecture

- **Invalidation is emitted at apply time (layer 2), not at commit.** Layout, scene and paint caches
  react to the edit immediately; the commit is invisible to the render pipeline.
- **External change detection.** The source is checked for modification since the last commit before
  writing, so a file changed underneath the session is a detected conflict rather than a silent
  overwrite.
- **`DocumentSource` capability-aware.** A read-only or high-latency source (HTTP, a future
  collaboration source) reports its capabilities and the commit policy adapts — longer intervals,
  larger batches — without the editor above it knowing.
- **The journal is the collaboration seam.** It is not in scope, but an operation log with semantic
  units is exactly what a future collaborative transport would carry. Keeping operations
  self-describing and address-based costs nothing now and keeps that door open.

---

## 7 · Budgets

| Budget | Target |
|---|---|
| Record an operation | < 50 µs, allocation-free in the common case |
| Journal flush | < 2 ms, off the frame path |
| Commit — 50-page document, one dirty part | < 50 ms on the worker |
| Commit — worst case, all parts dirty | must not block a frame; chunked on `wasm32` |
| Exposure window (unflushed work) | < 1 s of operations |
| Memory held by an uncommitted journal | bounded; forces a commit when exceeded |

`mjx-allocation-counter` already exists in the workspace and is what turns the memory bound into an
asserted test rather than a claim.
