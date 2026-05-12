# Implementation Plan

The first practical target is a headless Action! compiler runner that uses both
the Action! cartridge ROM and an Atari OS ROM. Cartridge-only execution through
fake OS services remains a later optimization after we know the real call
surface.

## Steps

1. ROM/image groundwork.
   - Keep the Action! cartridge ROM required.
   - Treat the Atari OS ROM as required for early execution.
   - Add ROM metadata checks: size, mapped address range, checksum/hash display.
   - Add config presets for common mappings: Action! cartridge at `$A000`, OS
     ROM at `$C000`.
   - Status: implemented. `run` requires `--cart` and `--os`, images report
     mapped range, checksum16, and CRC32, and the `action-os` preset captures
     the common `$A000`/`$C000` mapping.

2. CPU core decision.
   - Evaluate using an existing Rust 6502 core first.
   - Requirements:
     - deterministic stepping,
     - memory bus hooks,
     - PC/register inspection,
     - no heavy emulator dependency.
   - Implement a small 6502 core only if no suitable crate fits.

3. Memory bus.
   - Replace raw memory access with a bus abstraction.
   - Support RAM writes and ROM/cartridge read-only regions.
   - Support watchpoints, vector reads/writes, and trace hooks for OS-range
     calls.

4. Boot/entry experiment.
   - Load cartridge plus OS ROM.
   - Reset the CPU from the reset vector.
   - Step with tracing enabled.
   - Confirm whether execution reaches cartridge code.
   - Record the first missing hardware or OS assumption.

5. Trace infrastructure.
   - Add CLI flags:
     - `--trace-pc`,
     - `--trace-range $C000:$FFFF`,
     - `--watch $000E`,
     - `--max-cycles`.
   - Emit compact trace logs that help discover Action!'s compiler path.

6. Minimal Atari OS surface.
   - Initially run the real OS ROM.
   - Intercept or log OS vectors and CIO calls.
   - Identify which calls are needed for compile-only operation.
   - Replace calls only when they are well understood.

7. Source injection.
   - First version may still drive Action!'s normal memory expectations.
   - Discover the source/editor buffer layout.
   - Add a loader that places `.ACT` text into the right memory region.
   - Track APPMHI/CODE (`$0E`) and CODEBASE (`$491`).

8. Compile invocation.
   - Find a way to invoke compilation.
   - Preferred path: jump directly to a compiler routine.
   - Fallback path: simulate enough keyboard/editor command flow.
   - Save the exact entry-point contract in docs.

9. Object extraction.
   - After compile, read object code from the configured origin/code-pointer
     range.
   - Emit raw bytes, an Atari load segment, and eventually optional
     listing/disassembly.
   - Compare with existing original `.COM` probes.

10. Harness integration.
    - Add a command shaped like:

```sh
action-compiler-vm compile \
  --cart ACTION.ROM \
  --os ATARIOS.ROM \
  --source probe.act \
  --origin '$3000' \
  --out probe.com
```

    - Return structured status:
      - compile succeeded,
      - Action! error code,
      - timeout/cycle limit,
      - unsupported OS/device call.

11. Probe automation.
    - Add a small manifest format:

```toml
[[probe]]
name = "abi_calls"
source = "../actionc/experiments/original-compiler-probes/abi_calls.act"
origin = "3000"
```

    - Batch compile probes with original Action!.
    - Store outputs under `outputs/original`.

12. Cartridge-only spike.
    - Once OS calls are known, try replacing the OS ROM with fake services.
    - Keep this as a later milestone, not a blocker.

## Suggested First Slice

Start with steps 1 through 3:

- ROM metadata validation,
- CPU-core selection,
- memory bus and watchpoint scaffolding.

That gives the project enough instrumentation to learn from the cartridge and OS
instead of guessing.
