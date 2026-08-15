# action-compiler-vm

Headless experiment for running the original Action! compiler without driving a
full emulator UI.

The goal is a small "compiler VM", not a general Atari emulator. The useful
finish line is:

1. load the Action! cartridge and any required OS/runtime images,
2. place an `.ACT` source program into the compiler's expected memory state,
3. run just enough 6502/OS/CIO behavior to invoke compilation,
4. extract the generated object bytes for comparison with `actionc`.

If this grows into ANTIC/GTIA/POKEY/display-list or timing emulation, the spike
should stop and fall back to Atari800 automation.

## Current Status

This repo currently contains a no-dependency Rust scaffold:

- a 64K memory image,
- a bus with writable RAM, read-only OS ROM, cartridge mapping, and watchpoints,
- `.CAR` container detection for cartridge images,
- ROM metadata reporting with mapped range, checksum, and CRC32,
- an `action-os` mapping preset for cartridge at `$A000` and OS ROM at `$C000`,
- a bootstrap 6502 CPU core with reset, stepping, tracing, and a growing opcode
  subset,
- execution recorder support: PC trace ranges, trace-until, watchpoints, and
  recent-instruction stop reports,
- a CLI that can load and inspect cartridge/ROM/source files,
- tests around basic memory mapping.

CPU execution and Action! entry-point discovery are intentionally not
implemented yet.

## Commands

```sh
cargo test
cargo run -- inspect --cart path/to/action.rom
cargo run -- run --cart path/to/action.rom --os path/to/atari-os.rom --max-cycles 1000 --trace-pc
cargo run -- run --profile standalone-object --load-object path/to/program.com --max-steps 1000
scripts/run-probe functions
scripts/run-probe all
```

`run` currently resets and steps the bootstrap CPU through the mapped bus.
For phase 1, `run` requires both the Action! cartridge ROM and an Atari OS ROM.

## Library use

The CPU loop and stop policy are also available without spawning the CLI. Image,
source, object, and host-file data can be supplied as bytes, so an embedding
test harness does not need temporary files:

```rust
use action_compiler_vm::{CompilerVm, ExecutionProfile, RunRequest, VmRunner};

let mut vm = CompilerVm::default();
vm.load_atari_object_for_execution(ExecutionProfile::StandaloneObject, &object_bytes)?;

let outcome = VmRunner::new(vm).run(RunRequest {
    max_steps: 10_000,
    ..RunRequest::default()
});
let result = outcome.memory().read(0x0600);
```

ROM and cartridge callers can use `CompilerVm::load_image_bytes`; host inputs
and captured outputs use `add_host_file_bytes`, `add_host_output`, and
`host_file_bytes`. The path-based `VmConfig` remains available for CLI-style
callers. `OriginalCompiler` and `CartridgeObject` profiles validate that the
cartridge and OS are present; `StandaloneObject` and `SyntheticTest` support
ROM-free execution.

PC-triggered key codes, scripted CIO input, and in-memory Action! source
injection can be supplied through `ScheduledActions`. Direct and gated
`PcTrigger` values make the same scheduling behavior available to library
clients and the CLI, while `ScheduledActionObservation` records what was
delivered.

`scripts/run-probe` runs the original Action! compiler in the VM against probe
sources from `../actionc/experiments/original-compiler-probes`. It feeds monitor
commands equivalent to:

```text
C "H:FUNCTIONS.ACT"
W "H:FUNC.COM"
```

By default it writes VM-generated load files to
`../actionc/experiments/original-compiler-probes/outputs/vm` and compares them
with matching files in `outputs/original` when present. Override paths with
`ACTION_PROBES_DIR`, `ACTION_VM_OUTPUT_DIR`, `ACTION_ORIGINAL_OUTPUT_DIR`,
`ACTION_VM_CART`, `ACTION_VM_OS`, or `ACTION_VM_MAX_STEPS`.

## Design Constraint

Prefer fake OS/compiler services over device emulation. The intended order is:

1. identify Action! compiler entry points and memory contracts,
2. load source directly into the expected buffer if possible,
3. implement or intercept only the OS/CIO calls the compiler actually uses,
4. extract object code directly from memory.
