# ROADMAP

**MVP complete — TASK-001 through TASK-021 are all done.** This document
originally planned every remaining step from TASK-009 to a fully working
MVP, as defined by [README.md](README.md)'s product description: capture
screenshots, screen video, microphone audio, notes, and markers during a
live working session; use cloud AI to turn that raw capture into a
structured, editable business process; export it as a Word functional
specification. It's kept here, unchanged in shape, as the historical
record of how that plan was scoped and sequenced; PROJECT_STATE.md
remains the authoritative record of what's actually built. Any future
work continues the same `TASK-0NN` sequence as a new entry, not by
reopening anything below.

## How to use this document

- **[PROJECT_STATE.md](PROJECT_STATE.md) is always the authoritative
  record of what is actually built.** This file is the forward-looking
  plan — what comes next and why, not a claim about what already exists.
  Before starting any step below, read PROJECT_STATE.md in full; it plus
  the step's own description here should be enough to write a complete
  task brief (in the level of detail TASK-008/TASK-009 were specified —
  objective, explicit in/out of scope, deliverables) and implement it,
  with no other context required.
- Each step is scoped to be independently implementable and testable —
  roughly the size of TASK-005 through TASK-009, not larger. A step that
  turns out bigger once inspected should be split further, not
  shrink-wrapped to fit.
- Task numbering continues the existing `TASK-0NN` sequence.
  [TASK_INDEX.md](TASK_INDEX.md) is a short status table in the same
  spirit as this file but has drifted out of date (still shows TASK-008
  as "TODO"); reconcile it against this file and PROJECT_STATE.md the
  next time it's touched, rather than trusting it on its own.
- Milestones group related steps; they are a planning aid, not a
  contract — PROJECT_STATE.md's "Current milestone" field is updated as
  steps within one complete.
- "Depends on" lists what a step needs to already exist. Steps are meant
  to be done in order; skipping ahead isn't supported by this plan.
- Every step inherits the standing rules already established and
  repeated in every task brief so far: don't redesign existing
  architecture, don't add unnecessary dependencies, don't implement a
  later step's scope early, run the full existing test suite plus new
  tests for the step, keep `cargo check` warning-free, keep
  `npm run tauri build` passing, and don't touch previous database
  migrations.

---

## M0 — Foundation (done)

TASK-001 through TASK-009. Application shell, SQLite persistence,
Project → Process → Capture domain hierarchy, and real screenshot
capture with safe media storage. See PROJECT_STATE.md for the full
record.

---

## M1 — Live Capture ✅ DONE (TASK-010 through TASK-015)

Everything needed for a consultant to capture a process passively, from
outside the main window, in every modality the product promises
(screenshot, recording, audio, quick markers) — before any AI or export
work begins. Capture quality/breadth is the input AI structuring (M2)
depends on, so it comes first.

### TASK-010 — Background persistence, system tray, and the active-process store ✅ DONE

**Depends on:** TASK-006 (Project Workspace), TASK-007 (Process
selection).
**Goal:** Let GoLive keep running when its main window is closed, add a
system tray icon as the real way to quit, and introduce the first piece
of genuinely cross-feature client state — "which Process is currently
active" — so a future floating widget (TASK-011) can read it without
the main window being open or focused.

**In scope:**
- Tauri system tray icon: menu with at least "Open GoLive" and "Quit".
  Closing the main window hides it instead of exiting the process;
  Quit from the tray is the only real exit (plus normal window-manager
  force-close).
- `src/stores/` gets its first real occupant: an "active process" store
  (id + the minimal denormalized fields a tray/widget label needs — name,
  parent project name). This is a client-side notion only; no backend
  schema change.
- Selecting a Process in the existing Processes UI sets it as active
  (exact trigger — implicit on select vs. an explicit "Set active"
  action — is this task's call; document whichever is chosen).
  Deleting the active Process/its Project clears it.
- Tray tooltip/menu reflects the active Process (or an explicit "no
  active process" state).

**Out of scope:**
- The floating widget itself, any hotkey, any capture action from the
  tray (TASK-011).
- Multi-window management beyond the tray + existing main window.

**Definition of done:** closing the window leaves GoLive running
(confirmed via `tasklist`); the tray menu can reopen the window or quit
for real; selecting a Process updates the store and the tray reflects
it; existing Project/Process/Capture functionality is unaffected.

### TASK-011 — Global hotkey and floating capture widget (screenshot only) ✅ DONE

**Depends on:** TASK-010 (active-process store, background persistence),
TASK-009 (real screenshot capture pipeline).
**Goal:** Let the user capture a screenshot into the active Process from
anywhere in Windows, without switching focus to GoLive — the core "while
you're doing your actual work" promise, for the one capture type that
already has a real, verified pipeline.

**In scope:**
- Add `tauri-plugin-global-shortcut` (official first-party Tauri
  plugin), one configurable-later-but-hardcoded-for-now default
  shortcut.
- A small always-on-top floating widget window: shows the active
  Process (from TASK-010's store) or a clear "no active process, pick
  one" state, and a "Capture screenshot" button.
- The hotkey and the widget's button both call the exact same
  `createScreenshotCapture` flow TASK-009 built — no backend capture
  changes expected.
- Widget shows a brief success/failure acknowledgment (it can't rely on
  the main window being visible to show the result).
- A way to show/hide the widget (tray menu entry is sufficient).

**Out of scope:**
- Recording, audio, markers (TASK-012+).
- Hotkey customization UI, multiple/configurable shortcuts.
- Picking a different active Process *from* the widget beyond the
  minimum needed for the "no active process" state to be resolvable
  (full switching UX can stay in the main window).

**Definition of done:** with GoLive's main window closed/hidden,
pressing the hotkey captures a real screenshot into the active Process
and it shows up in that Process's Captures list next time the main
window is opened; verified manually against the compiled app per the
same standard TASK-009 used (real display content, not a mock).

### TASK-012 — Quick markers ✅ DONE

**Depends on:** TASK-011 (floating widget).
**Goal:** A near-zero-friction way to flag a moment during live work
without filling in a title/description — the "markers" the product
description names as distinct from full notes.

**In scope:**
- One button/hotkey action that creates a Capture immediately (type
  `note`, an auto-generated title such as a timestamp, empty
  description) with no dialog and no required input — inspect whether
  this needs any backend change at all before assuming it does; the
  existing generic `create_capture` metadata path may already be
  sufficient.
- Surfaced from the floating widget (and optionally its own hotkey).
- Markers appear in the existing Captures list/detail like any other
  Note capture — no new CaptureType, no new UI beyond the quick-create
  entry point.

**Out of scope:**
- Editing markers differently from any other Note capture — they use
  the existing Edit flow unchanged.
- Any backend schema change (this step should very likely need none).

**Definition of done:** one click/hotkey from the widget creates a
visible Capture with no dialog interaction required.

### TASK-013 — Screen recording: capture engine and storage (no UI polish) ✅ DONE

**Depends on:** TASK-009 (native screenshot engine, MediaStorage — this
step extends both rather than replacing them).
**Goal:** Give Recording Captures real video media, mirroring what
TASK-009 did for Screenshot — backend and native layer first, UI
minimal, matching the risk-reducing incremental approach PROJECT_STATE.md
already flagged for this specific feature ("highest risk — flagged for
incremental proof-of-concept development").

**In scope:**
- Extend `native::screenshot`'s sibling area (or a new `native::
  recording` module) with a start/stop recording engine for the primary
  display — inspect whether `xcap`'s own video-capture surface covers
  this before adding any new dependency.
- Extend `MediaStorage` (or add a parallel storage path) to persist
  video files under `captures/<id>.<ext>` — decide the container/codec
  during this task, favoring whatever keeps the dependency footprint
  smallest and is natively playable.
- Two-phase capture flow (unlike screenshot's one-shot):
  `start_recording_capture(process_id, title, description)` returns a
  handle/in-progress marker; `stop_recording_capture(...)` finalizes the
  file and the Capture metadata row. Apply the same orphan-cleanup
  discipline TASK-009 established (a failed stop must not leave a
  half-written file referenced by a Capture row, or vice versa).
- Minimal command-level UI is acceptable (e.g. a Start/Stop pair with no
  polish) — full UX is TASK-014.

**Out of scope:**
- Recording UI polish, in-progress indicator, video playback in
  CaptureDetail (TASK-014).
- Microphone audio (TASK-015).
- Pause/resume, multi-monitor, area selection — same exclusions TASK-009
  already established for screenshots, still true here.

**Definition of done:** a Recording Capture created through this flow
has a real, playable video file on disk, keyed by Capture id, persisted
across restart, deleted alongside its metadata — same guarantees
TASK-009 proved for PNGs, now proved for video via equivalent automated
+ manual verification.

### TASK-014 — Recording UI and playback ✅ DONE

**Depends on:** TASK-013.
**Goal:** Make screen recording actually usable end to end.

**In scope:**
- Start/Stop recording controls in the Captures section and the
  floating widget, with a visible in-progress indicator (at minimum,
  elapsed time) reachable even while the main window isn't focused.
- `CaptureDetail` plays Recording captures' video (bounded player,
  consistent with how the screenshot preview is bounded/non-overflowing)
  via the same safe backend-media-access pattern `get_capture_media`
  established — extended for potentially large files if a single
  in-memory byte transfer stops being appropriate (evaluate; a
  streaming/seek-friendly approach may be needed for video where it
  wasn't for a single PNG).
- Delete behavior, loading/error states — same conventions as
  screenshots.

**Out of scope:** editing/trimming video, thumbnails, audio (TASK-015).

**Definition of done:** starting a recording from the widget while the
main window is hidden, then stopping it, produces a Capture whose video
plays back correctly in `CaptureDetail`.

### TASK-015 — Microphone audio capture ✅ DONE

**Depends on:** TASK-013/014 (recording pipeline).
**Goal:** Let a recording optionally include microphone audio, closing
the last capture-modality gap in the product description.

**In scope:**
- An opt-in audio toggle on the Start Recording action.
- Native microphone capture, muxed with (or stored alongside and synced
  with) the video from TASK-013 — inspect the simplest reliable
  approach before choosing a new dependency; document why alternatives
  were rejected, matching this project's existing dependency-decision
  discipline.
- Playback includes audio (browser/OS-native `<video>` playback should
  cover this for free once muxed correctly).

**Out of scope:** standalone audio-only captures (a "voice memo" type is
not part of the current CaptureType set and isn't being added); noise
suppression/processing.

**Definition of done:** a recording made with the audio toggle on has
synchronized audio and video in playback, verified manually with real
speech against real screen content.

---

## M2 — AI Structuring

Turns the raw capture (screenshots, recordings, notes, markers) collected
in M1 into an editable, structured business process. Deliberately
sequenced after M1 completes, since AI's input quality depends on having
every capture modality available first.

### TASK-016 ✅ DONE — Windows Credential Manager integration and AI settings

**Depends on:** none beyond M0 (Settings page already exists as a
placeholder).
**Goal:** Securely store the user's OpenAI API key, with no code or
future task ever storing it in SQLite, plain JSON, or source control —
the constraint docs/architecture.md §12 already commits to.

**In scope:**
- A small native credential-storage abstraction (mirroring
  `media::MediaStorage`'s "own one filesystem/OS resource, testable in
  isolation" shape) wrapping the Windows Credential Manager.
- Settings UI: enter/save/clear the API key, a "test connection" action
  that calls OpenAI with the stored key and reports success/failure
  without ever displaying the key back in plaintext once saved.
- No feature in the app calls OpenAI yet — this step is storage +
  settings only.

**Out of scope:** the AI service abstraction and any actual process
generation (TASK-017).

**Definition of done:** the key is never found in SQLite, app logs, or
any file under the app-data directory; Settings can save/clear/test it;
the key survives an application restart (stored by Windows, not GoLive).

### TASK-017 ✅ DONE (pending final live-key check) — AI service abstraction and raw process-generation pipeline

**Depends on:** TASK-016 (stored API key).
**Goal:** Prove the round trip — send a Process's captures to OpenAI,
get back a structured result — before building any editing UI around it.
This is docs/architecture.md §8's first real occupant.

**In scope:**
- A Rust trait (`AiService` or similar) + an OpenAI implementation
  behind it, per §8's existing planned shape — nothing above the trait
  ever references OpenAI-specific types.
- One command, e.g. `generate_process_draft(process_id)`: gathers a
  Process's Captures (titles, descriptions, and screenshot images where
  present), sends them to OpenAI, and returns the raw structured result.
- Minimal UI: a "Generate" action and a plain read-only view of whatever
  came back (not yet the polished editor — that's TASK-019). Persistence
  of the result is TASK-018's job; this step can hold it only in memory/
  return it directly if that keeps the step small.

**Out of scope:** persisting/versioning the generated content
(TASK-018), the real editor UI (TASK-019), Word export (TASK-020).

**Definition of done:** given a Process with a handful of real Captures,
the command returns a genuinely AI-structured result (not a canned
string) through the full React → Tauri → Rust → OpenAI → back chain,
verified with a real API key.

### TASK-018 ✅ DONE — Structured process content domain and versioning

**Depends on:** TASK-017.
**Goal:** Persist AI-generated process structure properly, and never let
a regeneration silently destroy a previous version — a rule
docs/architecture.md §5 already commits to.

**In scope:**
- A new persisted entity for a Process's structured content (e.g.
  `ProcessDocument`/`ProcessVersion` — name and exact shape are this
  task's call), 1:N with Process, following the same
  model/repository/service/command layering every prior domain used.
  This likely needs a new migration (`0005_...sql`) — the first since
  `0004_captures.sql`; every existing migration stays untouched, as
  always.
- Regenerating creates a new version rather than overwriting; versions
  are listable and retrievable.
- `generate_process_draft` (TASK-017) is wired to persist through this
  layer instead of returning ephemeral data.

**Out of scope:** the editor UI itself (TASK-019); deleting/pruning old
versions (not required for MVP; can remain unbounded for now).

**Definition of done:** generating twice for the same Process produces
two retrievable versions, neither overwriting the other; full repository/
service test coverage matching the rigor already established for
Project/Process/Capture.

### TASK-019 ✅ DONE — Process editor UI

**Depends on:** TASK-018.
**Goal:** Let the user actually read and edit the AI-structured process
before it's considered ready to export — the "editable" half of the
product's "structured, editable business process" promise.

**In scope:**
- A view (likely a new Project Workspace area, or a Process-level view —
  this task's call, follow the simplest fit with the existing
  Workspace/tab conventions) showing a Process's latest structured
  version: sections/steps, editable text.
- Saving edits persists as an update to that version (or a new version —
  decide and document which, consistent with TASK-018's versioning
  model).
- A way to see/switch between versions and to trigger regeneration
  (calling back into TASK-017/018's pipeline).

**Out of scope:** rich formatting, drag-to-reorder steps, collaborative
editing — a working, honest text editor over the structured content is
enough for MVP.

**Definition of done:** a user can generate, read, edit, save, and
regenerate a Process's structured content entirely through the UI.

---

## M3 — Export and MVP Completion

### TASK-020 ✅ DONE — Word (.docx) functional specification export

**Depends on:** TASK-019 (edited, finalized structured content).
**Goal:** Deliver the product's final artifact — a Word document a
consultant can hand to a client.

**In scope:**
- Generate a `.docx` functional specification from a Process's
  (finalized/selected-version) structured content, with embedded
  screenshots referenced by the relevant steps where applicable.
- A native "Save As" flow (Tauri's file-save dialog) — the first place
  GoLive writes a file to a location the user chooses, scoped narrowly
  to this one export action; still no arbitrary frontend-supplied path
  reaches native code unvalidated.
- Inspect and choose a minimal, well-maintained Rust `.docx` generation
  approach before adding a dependency; document the choice and rejected
  alternatives per this project's existing dependency-decision
  discipline.

**Out of scope:** export templates/branding customization, PDF or other
formats, editing the exported file from within GoLive.

**Definition of done:** exporting a real, AI-structured, user-edited
Process produces a `.docx` file that opens correctly in Microsoft Word
and contains the expected structure and images.

### TASK-021 ✅ DONE — MVP end-to-end hardening and documentation

**Depends on:** TASK-020 (every prior M1/M2/M3 step, in effect).
**Goal:** Close the loop: walk the entire consultant workflow start to
finish, fix whatever friction/inconsistency turns up across steps built
somewhat independently, and update the project's own docs to describe a
complete MVP rather than a foundation.

**In scope:**
- One full manual pass: create a project → document a process live via
  the hotkey/widget (screenshot + recording + audio + a marker) → notes
  → generate the AI structure → edit it → export to Word — fixing
  real issues found along the way (empty/error/loading state
  consistency, obviously rough UX edges), not adding new features.
- Update README.md's "Current status" section, PROJECT_STATE.md, and
  docs/architecture.md to reflect the completed MVP.
- Full existing automated test suite still passes; no regressions in
  anything built since TASK-001.

**Out of scope:** any net-new feature. If this pass surfaces something
that's genuinely a new feature rather than a fix, it becomes its own
future task instead of being folded in here.

**Definition of done:** the workflow above works, unassisted, on a real
Windows machine, and the project's own documentation says so accurately.

---

## Explicitly deferred beyond this MVP roadmap

Not required for "the MVP fully working" as scoped above; revisit only
after TASK-021:

- Full-text search (FTS5) across Projects/Processes/Captures/generated
  content.
- ZIP import/export of a Project.
- Monitor selection / area selection for screenshots and recordings.
- Hotkey customization UI.
- Recording pause/resume.
- Multi-user / cloud sync of any kind.
- Any AI provider other than OpenAI.
