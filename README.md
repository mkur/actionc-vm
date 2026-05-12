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
- a CLI that can load and inspect cartridge/ROM/source files,
- tests around basic memory mapping.

CPU execution and Action! entry-point discovery are intentionally not
implemented yet.

## Commands

```sh
cargo test
cargo run -- inspect --cart path/to/action.rom
cargo run -- run --cart path/to/action.rom --os path/to/atari-os.rom --source probe.act
```

`run` currently loads inputs and reports that execution is not implemented.
For phase 1, `run` requires both the Action! cartridge ROM and an Atari OS ROM.

## Design Constraint

Prefer fake OS/compiler services over device emulation. The intended order is:

1. identify Action! compiler entry points and memory contracts,
2. load source directly into the expected buffer if possible,
3. implement or intercept only the OS/CIO calls the compiler actually uses,
4. extract object code directly from memory.
