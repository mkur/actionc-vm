# actionc-vm

[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)

`actionc-vm` is a deterministic, headless 6502 VM for Action! compiler work.
It is used both as a Rust library and as a command-line diagnostic harness.

The VM has two main jobs:

- execute Atari load-format objects produced by `actionc` and let tests inspect
  their final memory state;
- boot and drive the original Action! cartridge compiler for compatibility
  probes, object capture, symbol inspection, and Action/TN diagnostics.

It is intentionally not a general Atari emulator. Display-list rendering,
graphics, sound, and cycle-accurate hardware emulation belong in Atari800,
Altirra, or another full emulator.

## Capabilities

- all legal NMOS 6502 opcodes, instruction stepping, cycle accounting, and
  bounded execution history;
- 64 KiB RAM, Atari OS ROM mapping, Action! cartridge mapping and banking, and
  extra RAM/ROM/cartridge images;
- Atari load-format object parsing, segment loading, `RUNAD` startup, and a
  headless program environment;
- the OS and CIO behavior needed by the current Action! workflows, including
  in-memory `H:`/`D:` host files and captured outputs;
- scheduled keyboard input, CIO input, and Action! source injection at direct
  or gated PC triggers;
- watchpoints, protected code ranges, traces, memory dumps, and structured stop
  reports;
- Action!-specific editor, screen, menu, call, and symbol-table diagnostics;
- a no-dependency Rust library API for running programs without spawning the
  CLI or exchanging 64 KiB memory-dump files.

## Execution profiles

Choose the narrowest profile that supplies the services used by the program:

| Profile | Cartridge and OS | Entry path | Intended use |
| --- | --- | --- | --- |
| `OriginalCompiler` | Bundled Action! 3.6 and AltirraOS | Cartridge boot | Drive and inspect the original Action! compiler |
| `CartridgeObject` | Bundled Action! 3.6 and AltirraOS | Object `RUNAD` | Run generated code that calls Action! or OS services |
| `StandaloneObject` | Not required | Object `RUNAD` | Run self-contained generated code |
| `SyntheticTest` | Not required | Caller-defined state | Small library-only CPU and bus tests |

The CLI spells the first three profiles as `original-compiler`,
`cartridge-object`, and `standalone-object`. `original-compiler` is the CLI
default; `synthetic-test` is library-only.

## Quick start

Build and run the test suite:

```sh
cargo test
cargo run -- --help
```

Inspect a cartridge without running it:

```sh
cargo run -- inspect --cart path/to/action.rom
```

Run a self-contained Atari object without ROM images:

```sh
cargo run -- run \
  --profile standalone-object \
  --load-object path/to/program.com \
  --max-steps 100000 \
  --dump-range-on-stop '$0600:$060F'
```

Run an object that uses Action! cartridge or Atari OS services:

```sh
cargo run -- run \
  --profile cartridge-object \
  --load-object path/to/program.com \
  --max-steps 100000
```

Boot the original compiler with instruction tracing:

```sh
cargo run -- run \
  --max-steps 1000 \
  --trace-pc
```

Cartridge-backed profiles use embedded Action! 3.6 and AltirraOS XL/XE 3.11
images. Pass `--cart path/to/custom-action.rom` or
`--os path/to/custom-os.rom` to override either default. The bundled images'
licenses and provenance are recorded in
[`roms/README.md`](roms/README.md).

Addresses accept decimal, `0x` hexadecimal, or `$` hexadecimal notation.
`--max-cycles` remains accepted as a compatibility alias for `--max-steps`;
the limit counts CPU steps, not hardware cycles.

## Library use

For local development, add a path dependency:

```toml
[dependencies]
actionc-vm = { path = "../actionc-vm" }
```

Repository consumers should pin the Git dependency to an exact revision. This
keeps VM changes intentional and reproducible.

A standalone object can be loaded, executed, and inspected entirely in memory:

```rust
use actionc_vm::{CompilerVm, ExecutionProfile, RunRequest, VmRunner};

fn run_object(object: &[u8]) -> Result<u8, String> {
    let mut vm = CompilerVm::default();
    vm.load_atari_object_for_execution(ExecutionProfile::StandaloneObject, object)?;

    let outcome = VmRunner::new(vm).run(RunRequest {
        max_steps: 10_000,
        ..RunRequest::default()
    });

    Ok(outcome.memory().read(0x0600))
}
```

Use `CompilerVm::load_bundled_action_cartridge` and
`CompilerVm::load_bundled_altirra_os` to install the defaults explicitly, or
`CompilerVm::load_image_bytes` for custom cartridge, OS, and other images.
`add_host_file_bytes`, `add_host_output`, and `host_file_bytes` provide
in-memory host I/O. `ScheduledActions` supplies PC-triggered keys, CIO data,
and source injection. `RunOutcome` retains the final VM together with a typed
stop reason, step and cycle counts, registers, and recent instructions.

Path-oriented `VmConfig` remains available for CLI-style callers, but reusable
test harnesses should prefer the byte-oriented API.

## ActionC integration

The `actionc` repository keeps its VM runtime gates in the isolated
`tools/vm-runtime-tests` crate. That crate pins `actionc-vm` by exact Git
revision and is deliberately outside the main workspace dependency graph, so a
normal compiler build does not fetch or build the VM.

The current runtime suite executes seven self-contained fixtures with
`StandaloneObject` and twelve fixtures that need runtime services with
`CartridgeObject`. The tests inspect VM memory directly rather than invoking
the CLI, writing a full memory dump, and parsing it in shell.

## Original compiler probes

`scripts/run-probe` drives the original Action! cartridge compiler against a
named set of probe sources. It supplies monitor input equivalent to:

```text
C "H:FUNCTIONS.ACT"
W "H:FUNC.COM"
```

List or run probes with:

```sh
scripts/run-probe --list
scripts/run-probe functions
scripts/run-probe all
```

The script's default path targets the current sibling `actionc-public-release`
checkout layout. Override it when the compiler repository lives elsewhere:

```sh
ACTION_PROBES_DIR=/path/to/actionc/surveys/probes/original-compiler \
scripts/run-probe functions
```

Set `ACTION_VM_CART` or `ACTION_VM_OS` only when comparing against different
images; the probe runner otherwise uses the VM's bundled Action! cartridge and
AltirraOS.

The runner stops normally after its scripted `Q:` input has been consumed and
Action! returns to an idle keyboard read. `ACTION_VM_MAX_STEPS` remains a safety
limit for cartridge or harness failures.

VM-generated objects and symbol JSON files are written below the probe output
directory and compared with matching original-compiler captures when present.
The output paths and step limit can be overridden with
`ACTION_VM_OUTPUT_DIR`, `ACTION_VM_SYMBOL_OUTPUT_DIR`,
`ACTION_ORIGINAL_OUTPUT_DIR`, and `ACTION_VM_MAX_STEPS`.

## Design boundary

Prefer a small, explicit host-side model of the services exercised by the
compiler over broader device emulation. The reusable library owns CPU, memory,
bus, object loading, execution policy, and structured results. The CLI owns
argument parsing, filesystem I/O, human-readable traces, and capture-file
formatting.

The VM core must not depend on `actionc` or bundle ROM images without clear
redistribution and source terms. Its Action! and AltirraOS defaults retain
their own notices. The VM must not grow into ANTIC, GTIA, POKEY, display-list,
audio, or cycle-accurate video emulation.

See [the library refactor implementation note](docs/LIBRARY_REFACTOR_IMPLEMENTATION_NOTE.md)
for the ownership boundary, migration status, and remaining decomposition
work.

## License

Copyright (C) 2026 Michal Kurcewicz

`actionc-vm` and its original supporting code are free software licensed under
the [GNU General Public License, version 3 or any later version](LICENSE), like
the main `actionc` project.

The bundled Action! 3.6 cartridge is available under
[GPL version 3 or later](roms/ACTION-ROM-NOTICE.md), with its corresponding
source preserved under `roms/source/`. The bundled AltirraOS image retains its
[file-specific permissive license](roms/ALTIRRAOS-LICENSE). Both images'
provenance is recorded in [roms/README.md](roms/README.md).
