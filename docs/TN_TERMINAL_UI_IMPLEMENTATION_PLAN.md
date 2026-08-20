# TN Terminal UI Implementation Plan

## Purpose

Run TOMS Navigator interactively inside a character terminal using
`actionc-vm`, without starting Atari800, Altirra, or another graphical Atari
emulator.

TN is a good fit for this frontend because its normal interface is a 40x24
text screen driven by keyboard input. The VM already boots MyDOS, loads the
standalone TN object, exposes the visible text screen, queues Atari key codes,
and services the disk operations used by TN. The missing work is primarily a
stable interactive-session API and a terminal presentation layer.

The implementation should be curses-like but use `crossterm` rather than a
native curses library. This keeps the executable portable across Windows,
macOS, and Linux and avoids adding a platform C-library dependency.

## Goals

- launch the audited standalone TN object directly after booting MyDOS;
- display TN's 40x24 text screen in an interactive terminal;
- preserve inverse-video cells so the active panel and selection are visible;
- translate ordinary terminal input, navigation keys, and function keys into
  Atari keyboard codes;
- keep the VM responsive without continuously busy-spinning while TN waits for
  input;
- use the existing ATR model and explicit copy-on-write output paths;
- restore terminal state reliably after normal exit, errors, and unwinding;
- keep the session engine independently testable without a real terminal.

## Non-goals

- ANTIC display-list rendering;
- GTIA colors, player/missile graphics, artifacting, or raster effects;
- POKEY audio or cycle-accurate keyboard emulation;
- support for arbitrary Atari graphical applications;
- mouse emulation;
- replacing the existing headless `run` command or TN integration tests.

The terminal frontend reads the text screen that TN has already constructed in
RAM. It does not turn `actionc-vm` into a general Atari emulator.

## Current Baseline

The repository already has the essential low-level pieces:

- `ExecutionProfile::DiskBoot` boots the bundled AltirraOS and MyDOS;
- ATR drives 1 through 8 support reads, copy-on-write updates, formatting, and
  the error paths exercised by TN;
- `CompilerVm::load_atari_object` loads the standalone TN object and exposes
  its `RUNAD`;
- `Bus::text_screen_snapshot(40, 24)` locates the screen through the display
  list or `SAVMSC` and decodes it into text;
- `Bus::queue_key_code` supplies Atari keyboard codes through `CH`;
- the test-only `TnHarness` characterizes boot, launch, keyboard readiness,
  panel navigation, file operations, executable launch, and visible errors;
- `tests/fixtures/tn-standalone.com` is an audited, reproducible TN object with
  GPL-compatible provenance recorded in `TN-NOTICE.md`.

Two current interfaces are insufficient for an interactive frontend:

1. `TextScreenSnapshot` contains only decoded strings. Its decoder masks bit 7,
   so inverse-video information is lost.
2. The TN test harness recognizes the bundled AltirraOS keyboard wait by its
   program counter. An interactive session should instead observe a public bus
   state that remains valid when the OS implementation changes.

## User Experience

Add a dedicated command:

```sh
actionc-vm tn \
  --disk 2:work.atr \
  --disk-writeback 2:work-updated.atr
```

If drive 1 is omitted, the command mounts the bundled MyDOS image as an
ephemeral copy-on-write boot drive. An explicit `--disk 1:...` replaces that
default. Existing `--disk` and `--disk-writeback` syntax and validation should
be reused.

The audited TN object is the default. An override is available for development
and compatibility testing:

```sh
actionc-vm tn --tn-object path/to/tn.com --disk 1:mydos.atr
```

The terminal shows:

- the 40x24 TN screen, preserving inverse video;
- a one-line host status area below it for drive/writeback state and errors;
- a short host-key hint, including the key used to leave the terminal session.

The 40x24 grid should be displayed at its natural size. A terminal smaller
than the required grid plus status area gets a clear resize message rather
than a clipped or corrupted screen. Resizing is handled dynamically.

Use a host-only key such as `Ctrl+]` to exit. `Escape`, `Q`, and TN's normal
keys must continue to reach TN. On exit, write only outputs explicitly named by
`--disk-writeback`; never overwrite an input ATR implicitly.

## Architecture

Keep the layers separate:

```text
CompilerVm / Bus
    raw text cells, keyboard state, key queue, ATR state
             |
             v
TnSession
    boot, launch, step scheduling, screen snapshots, input dispatch
             |
             v
TerminalFrontend
    crossterm events, cell rendering, resize, status, terminal guard
             |
             v
CLI `tn` command
    paths, option parsing, asset selection, writeback, diagnostics
```

The VM library must not print terminal control sequences or read host events.
The terminal frontend must not reach into private RAM fields or depend on
AltirraOS program-counter addresses.

## Library Primitives

### Attribute-preserving text cells

Add an attribute-preserving screen API while keeping
`text_screen_snapshot()` compatible for diagnostics and existing tests:

```rust
pub struct TextScreenCell {
    pub screen_code: u8,
    pub character: char,
    pub inverse: bool,
}

pub struct TextScreenGrid {
    pub base: u16,
    pub columns: usize,
    pub rows: usize,
    pub cells: Vec<TextScreenCell>,
}

impl Bus {
    pub fn text_screen_grid(
        &self,
        columns: usize,
        rows: usize,
    ) -> TextScreenGrid;
}
```

`screen_code` retains the original byte. `character` decodes the lower seven
bits using the existing text mapping. `inverse` reflects bit 7. The first
frontend does not need Atari colors, so no color model should be added.

Keep the cell vector flat and row-major. This makes screen comparison and
dirty-cell rendering straightforward and avoids allocating one string per row
on every refresh.

### Keyboard readiness

Make native keyboard waiting observable without a ROM address:

```rust
impl Bus {
    pub fn keyboard_input_is_idle(&self) -> bool;
}
```

When code reads `CH` while the latch is `$FF` and no queued key is available,
the bus records that the guest is waiting. Queueing or delivering a key clears
the state. The predicate is true only when:

- an empty keyboard read has been observed;
- `CH` is `$FF`;
- the pending key queue is empty.

Refactor `TnHarness::wait_for_keyboard` to use this public predicate. This both
proves the abstraction against TN and removes the current bundled-OS PC
assumption from the tests.

## Reusable TN Session

Move the reusable behavior of the test harness into a small production module,
without moving test assertions or TN-specific workflow helpers:

```rust
pub struct TnSession { /* CompilerVm plus scheduling state */ }

pub enum TnSessionState {
    Booting,
    Starting,
    WaitingForInput,
    Running,
    Exited,
    Failed(String),
}
```

The session owns:

- mounting or accepting prepared drive images;
- preparing `DiskBoot` and waiting for `dos_boot_is_ready()`;
- loading TN, selecting its `RUNAD`, and running to the first input wait;
- stepping in bounded batches;
- returning screen grids and structured state;
- accepting already translated Atari key codes;
- exposing changed ATR images through existing VM APIs.

The session does not own filesystem paths, terminal events, rendering, or
writeback. It accepts bytes and structured configuration.

Use two stepping policies:

- while TN is running, execute a bounded batch and then yield to the frontend;
- while TN is waiting with no queued input, execute no guest instructions and
  block briefly in the terminal event poll.

This prevents a host CPU busy loop while still allowing progress screens and
long disk operations to redraw. Step and wall-clock limits remain safety
guards, not normal completion signals.

## Terminal Frontend

Add `crossterm` as the only terminal dependency. A full widget framework is not
needed for a fixed text grid.

### Terminal lifetime

An RAII guard must:

1. enable raw mode;
2. enter the alternate screen;
3. hide the host cursor;
4. restore the cursor, leave the alternate screen, and disable raw mode in
   `Drop`.

Installation failures unwind already completed steps. Normal errors return
through the guard rather than calling `process::exit` while raw mode is active.
The CLI converts the final result into an exit code only after restoration.

### Rendering

Maintain the previous `TextScreenGrid` and update only changed cells. Render
inverse cells using terminal reverse-video attributes. Clear attributes after
every changed run so styles cannot leak into adjacent cells or the status row.

Redraw when:

- the session reaches keyboard idle;
- a running batch changes the screen;
- the terminal is resized;
- host status or an error changes.

Do not derive TN selection state from private TN variables. The visible screen
bytes are the rendering contract.

### Input mapping

Translate `crossterm::event::KeyEvent` into Atari key codes in a pure function.
Cover at least:

- printable ASCII and Space;
- Return and Escape;
- cursor Up, Down, Left, and Right;
- Backspace/Delete and Tab where supported by TN;
- F1 through F9;
- modified keys used by documented TN commands;
- the host-only exit chord.

Keep the key-code table in one module and give public constants semantic names.
Unknown terminal events are ignored and optionally reported in the host status
area; they must not inject guessed key codes.

Key press events are sufficient initially. Ignore release and repeat events
unless characterization shows that a terminal backend requires explicit
filtering.

## Bundled TN Asset

Promote the audited TN object from a test-only fixture to a clearly named
bundled asset used by both tests and the `tn` command. Keep its Action! source,
build command, compiler revision, checksum, and license notice adjacent to the
binary.

The CLI selects the bundled bytes by default and passes them through the normal
Atari object loader. The VM core does not gain TN-specific loading semantics.
`--tn-object` reads replacement bytes in the CLI and uses the same path.

## Disk and Writeback Semantics

Reuse the existing ATR implementation and CLI option syntax:

- mounted images are read-only unless copy-on-write is explicitly requested;
- the implicit bundled D1 boot image is always ephemeral;
- `--disk-writeback unit:path` enables copy-on-write for that unit and writes
  the resulting ATR to the named output;
- no input path is overwritten unless the user explicitly supplies the same
  path as the writeback target and existing CLI validation permits it;
- writeback occurs after terminal restoration;
- a writeback failure is reported and produces a non-zero exit status;
- the host status row marks dirty drives but does not claim persistence until
  writeback succeeds.

Use the existing serializer so 128-byte and 256-byte ATR geometry is preserved.

## Implementation Slices

Each slice should be independently committed and leave the full suite green.

### Slice 1: screen cells and keyboard-idle contract

- add `TextScreenCell`, `TextScreenGrid`, and `Bus::text_screen_grid`;
- preserve the current string snapshot API using the new decoder internally;
- record native empty-`CH` reads and expose `keyboard_input_is_idle`;
- refactor `TnHarness` away from the AltirraOS keyboard-wait PC;
- add focused tests for inverse cells, ordinary cells, readiness transitions,
  and queued-key transitions.

Suggested commit:

```text
feat: expose terminal-ready text and keyboard state
```

### Slice 2: reusable TN session engine

- add a production `TnSession` module with byte-oriented configuration;
- boot MyDOS, load TN, and run to structured input readiness;
- add bounded batch stepping and state transitions;
- convert existing TN tests to use the session where practical;
- retain workflow-specific assertions in tests.

Suggested commit:

```text
feat: add reusable interactive TN session
```

### Slice 3: terminal renderer and key mapping

- add `crossterm`;
- implement the RAII terminal guard;
- implement dirty-cell rendering and reverse video;
- implement pure terminal-to-Atari key translation;
- test rendering through an in-memory command sink rather than a real TTY;
- unit-test every supported key and the host exit chord.

Suggested commit:

```text
feat: add terminal frontend for text sessions
```

### Slice 4: `tn` CLI command and bundled asset

- promote the TN fixture and provenance into a shared bundled asset;
- add `actionc-vm tn` option parsing and help;
- reuse disk and writeback parsing from `run`;
- supply the bundled MyDOS D1 default when drive 1 is absent;
- support `--tn-object` override;
- connect the session engine, terminal frontend, status row, and writeback;
- add parser and non-TTY error tests.

Suggested commit:

```text
feat: run TN in an interactive terminal
```

### Slice 5: platform verification and documentation

- document the command and key mappings in `README.md`;
- verify macOS, Linux, and Windows terminal behavior;
- add a CI-safe end-to-end session test using the in-memory frontend;
- manually verify D1/D2 navigation, tagging, copying, renaming, formatting,
  executable launch, read-only errors, disk-full errors, resize, and exit;
- confirm writeback ATRs reopen and contain the expected changes.

Suggested commit:

```text
docs: document interactive TN terminal mode
```

## Test Strategy

### Library tests

- all 128 screen codes decode consistently with the existing snapshot;
- bit 7 changes only `inverse`, not the decoded character;
- screen base discovery still works through both LMS and `SAVMSC`;
- keyboard idle becomes true only after an empty guest read;
- queueing, delivering, and consuming keys produce the expected transitions;
- TN reaches input idle without checking a ROM PC.

### Session tests

- boot and launch reach `WaitingForInput` with the expected title visible;
- a key moves the session through `Running` and back to `WaitingForInput`;
- the screen updates after panel switches and directory navigation;
- long copy and format operations yield intermediate batches without timing
  out;
- VM failures become `Failed` states with useful context;
- dirty drive state remains available for writeback.

### Frontend tests

- a synthetic grid emits the expected cursor moves, characters, and reverse
  attributes;
- an unchanged grid emits no cell redraws;
- resize and too-small-terminal states render deterministically;
- every supported host key maps to the characterized Atari code;
- the exit chord is consumed by the host and never queued into the VM;
- cleanup operations run when rendering or session stepping returns an error.

Do not require a real terminal or PTY in the normal test suite. A manual PTY
smoke test may be supplied as an ignored test or development script.

## Required Verification

After every slice:

```sh
cargo fmt --check
cargo test
```

Before declaring the feature complete, also run ActionC's pinned VM consumer:

```sh
cd ../actionc-public-release/tools/vm-runtime-tests
cargo test --locked
```

Then update ActionC's exact `actionc-vm` revision only after the VM commits are
pushed.

## Risks and Mitigations

### Screen attributes are currently discarded

Without raw cells, TN is readable but its selection is ambiguous. Add the
attribute-preserving API before terminal work and keep the old API compatible.

### Keyboard waiting is coupled to one OS address in tests

Do not carry that address into production. Record empty `CH` reads in the bus
and expose a state predicate with transition tests.

### Host terminal state can be left corrupted

Centralize all terminal setup in an RAII guard. Avoid early process exits while
the guard is alive and test cleanup with injected writer failures.

### Terminal key reporting varies by platform

Use `crossterm`, normalize events in one pure mapping function, ignore unknown
events, and manually verify all three supported host platforms.

### A wait-only loop can hide progress screens

Step in bounded batches and compare screens between batches. Stop stepping only
after the public keyboard-idle predicate becomes true.

### Disk changes can be lost or overwrite user data

Keep the existing explicit copy-on-write contract. Show dirty state, restore
the terminal before writing files, and never add implicit in-place saves.

### Bundling TN can obscure provenance

Keep source, reproducible build instructions, checksums, and GPL notice next to
the promoted asset. Preserve `--tn-object` for independently built versions.

## Completion Criteria

The feature is complete when:

- `actionc-vm tn` starts TN in a standard terminal on macOS, Linux, and Windows;
- both panels, selection, inverse text, and error messages are legible;
- keyboard navigation and TN function shortcuts work without dropped or
  duplicated keys;
- idle TN consumes negligible host CPU;
- copy and format operations can update a COW disk and save it to an explicit
  output ATR;
- terminal state is restored after success, failure, and host exit;
- existing VM, TN, CLI, and ActionC pinned-consumer tests remain green;
- the VM still contains no ANTIC/GTIA renderer or cycle-accurate video model.
