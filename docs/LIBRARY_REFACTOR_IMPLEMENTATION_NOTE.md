# VM Library Refactor Implementation Note

## Purpose

`action-compiler-vm` already exposes its CPU, bus, object loader, CIO harness,
and Action!-specific memory decoders from `src/lib.rs`.  The reusable boundary
is incomplete, however: the execution loop, stop policy, scheduled actions,
tracing, and result collection still live in the CLI's `src/main.rs`.

This refactor makes the VM usable as a deterministic library while preserving
the existing command-line interface and the original-compiler capture scripts.
The VM remains a separate crate and repository.  It must not become a normal
runtime dependency of `actionc`.

## Current Baseline

At the start of the refactor:

- `src/lib.rs` is approximately 6,600 lines;
- `src/main.rs` is approximately 3,100 lines;
- 85 library tests pass;
- 28 CLI tests pass;
- `actionc` has 19 `run-*-vm.sh` runtime scripts and 18 ignored integration
  gates that invoke this crate as a subprocess;
- every `run` currently requires an Action! cartridge and Atari OS image, even
  when `--load-object` starts a generated program directly at `RUNAD`.

The passing CLI behavior, exit status, trace ordering, and capture artifacts are
compatibility contracts until their consumers have migrated.

## Ownership Boundary

### Library owns

- CPU, bus, memory, cartridge, ROM, and Atari load-object models;
- headless environment initialization;
- CIO host devices and captured output;
- execution limits and stop conditions;
- pre-step actions and post-step observers;
- structured instruction, call, CIO, symbol, screen, and memory observations;
- final VM state and structured stop reports;
- Action!-specific editor and symbol-table memory decoding.

### CLI owns

- command-line parsing and help text;
- filesystem paths and file I/O;
- loading ROM, source, object, listing, and map bytes;
- parsing `actionc` listing/map text into structured metadata;
- human-readable formatting;
- JSON, memory-dump, host-output, and capture-file writing;
- process exit codes.

The core library must not print, write files, terminate the process, or depend
on `actionc`.

## Target Execution API

The public execution surface should converge on these concepts:

```rust
pub struct VmRunner { /* VM plus scheduled actions and observers */ }

pub struct RunRequest {
    pub max_steps: u64,
    pub history_len: usize,
    pub stop_conditions: Vec<StopCondition>,
}

pub struct RunOutcome {
    pub stop: StopReason,
    pub steps: u64,
    pub registers: CpuRegisters,
    pub history: Vec<CpuStep>,
    pub vm: CompilerVm,
}

pub enum StopReason {
    StepLimit,
    PcReached { pc: u16 },
    Halted,
    UnsupportedOpcode { pc: u16, opcode: u8 },
    ProtectedCodeWrite { /* address and instruction facts */ },
}
```

Exact ownership may evolve to avoid unnecessary memory copies, but callers must
be able to inspect final RAM and CIO outputs without requesting a 64 KiB dump.
Expected VM stops are structured outcomes; invalid configuration, malformed
objects, and host-side setup failures are errors.

## Execution Ordering Contract

The library runner must preserve the current loop ordering:

1. Load images and initialize the selected environment.
2. Load Atari objects and choose `RUNAD`, when requested.
3. Apply explicit RAM pokes.
4. Install protected and allowed code-write ranges.
5. At each PC, execute scheduled pre-step actions.
6. Step the CPU exactly once.
7. Feed the completed step to observers and history.
8. Evaluate stop conditions and CPU stop reasons.
9. Return the final VM and structured outcome.

This order matters for PC-triggered source injection, deferred keyboard/CIO
input, symbol snapshots, call tracing, and protected-write diagnostics.

## Execution Profiles

Image validation must eventually depend on an explicit profile:

- `OriginalCompiler`: requires the Action! cartridge and Atari OS;
- `CartridgeObject`: runs an emitted object with cartridge runtime services;
- `StandaloneObject`: runs a standalone emitted object without Action! ROM;
- `SyntheticTest`: executes small in-memory programs without external ROMs.

The existing CLI keeps `OriginalCompiler` behavior as its default during the
refactor.  Cartridge-free object execution is a later, explicit behavior slice.

## Migration Slices

### Slice 1: structured runner nucleus

- add `RunRequest`, `StopReason`, and `RunOutcome` to the library;
- move the simple step-limit/PC/CPU-error loop into `VmRunner`;
- retain final `CompilerVm` state in the outcome;
- add ROM-free synthetic tests for every stop reason;
- do not migrate CLI triggers or trace formatting yet.

### Slice 2: CLI adopts the runner

- add runner hooks needed by the current pre-step and post-step behavior;
- route `run_vm` through the library loop;
- keep CLI flags, output text, exit status, and stop artifacts unchanged;
- add CLI characterization tests before deleting the old loop.

### Slice 3: triggers and observers

- model key/CIO/source injection and symbol snapshots as pre-step actions;
- model history, range tracing, Action call tracing, fixup tracing, and code
  pointer tracing as observers;
- return structured observations and keep formatting in the CLI.

The library accepts structured routine metadata and code ranges.  It does not
parse `actionc` listing or map syntax.

### Slice 4: host I/O boundary

- accept ROM, object, source, and host-file contents as bytes;
- return captured host outputs as bytes;
- leave path resolution and writes in the CLI;
- keep compatibility constructors for the current path-based configuration
  until all CLI call sites migrate.

### Slice 5: execution profiles

- split image loading from execution policy;
- introduce profile-specific validation;
- preserve cartridge-backed CLI defaults;
- add standalone and synthetic execution paths without fake cartridge inputs.

### Slice 6: source decomposition

After behavior is library-backed, split the monolithic sources mechanically:

```text
src/cpu.rs
src/bus.rs
src/images.rs
src/object.rs
src/cio.rs
src/action.rs
src/runner.rs
```

Keep public re-exports stable.  Do not combine module moves with semantic
changes.

### Slice 7: first `actionc` consumer

Migrate the `initialized_arrays` runtime gate first:

- compile classic and MIR6502 objects;
- invoke `VmRunner` directly;
- inspect `$0600-$0605` in final RAM;
- remove the temporary memory dump and `od` parsing from the new path;
- keep the existing shell gate until both paths agree.

The dependency must be non-default and pinned.  Normal `actionc` builds must not
compile the VM or require ROM files.  Do not encode a permanent sibling path in
`actionc`'s `Cargo.toml`; use a versioned Git dependency behind a dedicated
`vm-tests` feature, or vendor the crate only if offline builds require it.

### Slice 8: gradual runtime-gate migration

Migrate remaining gates in this order:

1. pure memory-result fixtures;
2. ABI and runtime-helper fixtures;
3. CIO and host-file fixtures;
4. TN and toolkit diagnostics.

Original-compiler probe and toolkit capture scripts may remain CLI consumers;
batch capture is an appropriate process boundary.

## Non-Goals

- no ANTIC, GTIA, POKEY, display-list, audio, or cycle-accurate video work;
- no replacement for Atari800, Altirra, or AltirraBridge;
- no dependency from the VM core to `actionc`;
- no ROM bundling into the library;
- no removal of the CLI or current capture scripts;
- no broad module rename before the execution contract is covered by tests.

## Verification

Every slice runs:

```sh
cargo test
```

Major boundaries also run:

- one classic and one MIR6502 direct-object runtime fixture;
- one original-compiler probe with byte-identical output;
- protected-code-write and host-CIO smoke tests;
- the corresponding `actionc` tests when integration changes.

## Completion Criteria

- the CLI and existing capture scripts remain functional;
- the execution loop is library-owned and produces structured outcomes;
- final RAM and CIO state are directly inspectable by callers;
- at least one `actionc` runtime gate uses the library API;
- standalone objects can run without an Action! cartridge;
- normal `actionc` builds remain independent of the VM;
- hardware-timing work remains explicitly outside the VM boundary.
