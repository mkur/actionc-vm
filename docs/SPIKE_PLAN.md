# Spike Plan

This project should answer one question before it becomes "an emulator":

Can we run the original Action! compiler in a small, deterministic Rust process
and extract object code automatically?

## Phase 1: Image Loader

- Load cartridge and OS ROM bytes into a 64K address space.
- Support explicit extra image mappings for experiments.
- Keep image origins visible in CLI output.

Done in the initial scaffold.

## Phase 2: Discovery Harness

- Add a CPU core boundary.
- Prefer adopting an existing 6502 core if licensing and API are acceptable.
- Add tracing hooks for:
  - program counter,
  - OS vector calls,
  - writes to APPMHI/CODE (`$0E`) and CODEBASE (`$491`),
  - writes to the compiler output region.

## Phase 3: Action! Memory Contract

- Determine how source text is represented in the editor buffer.
- Determine whether compilation can be invoked directly without keyboard/editor
  scripting.
- Record the minimum initial memory locations required by Action!.

## Phase 4: Fake Services

- Intercept or implement only the services the compiler calls.
- Start with CIO/open/read/write-like paths if direct source-buffer loading is
  not enough.
- Avoid graphics, sound, display-list, and timing work unless proven necessary.

## Phase 5: Object Extraction

- Compile a fixed-origin probe using:

```action
SET $E=$3000
SET $491=$3000
```

- Extract object bytes from the Action! code pointer range.
- Emit a `.COM` or raw segment compatible with the existing `actionc` probe
  notes.

## Stop Conditions

Stop this mini-emulator path and automate Atari800 instead if we need:

- display hardware behavior,
- real keyboard/editor timing,
- SIO timing,
- broad Atari OS emulation beyond a small set of calls,
- cycle-level accuracy.
