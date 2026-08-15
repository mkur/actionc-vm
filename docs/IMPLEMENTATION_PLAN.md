# Implementation Plan

The first practical target used both the Action! cartridge ROM and an Atari OS
ROM. Cartridge-backed profiles now default to bundled Action! 3.6 and
AltirraOS images; cartridge-only execution through fake OS services remains a
possible later optimization.

## Steps

1. ROM/image groundwork.
   - Initially require a caller-supplied Action! cartridge ROM.
   - Initially require a caller-supplied Atari OS ROM.
   - Add ROM metadata checks: size, mapped address range, checksum/hash display.
   - Add config presets for common mappings: Action! cartridge at `$A000`, OS
     ROM at `$C000`.
   - Status: implemented. Cartridge-backed `run` profiles use bundled Action!
     3.6 and AltirraOS unless `--cart` or `--os` overrides them. Images report
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
   - Status: bootstrap core implemented in-tree. It resets from `$FFFC`,
     steps only through the bus, exposes registers/PC/status, supports trace
     output, and currently implements enough opcodes to run deeply into the
     Atari OS reset path. It is intentionally incomplete and stops cleanly on
     unsupported opcodes.

3. Memory bus.
   - Replace raw memory access with a bus abstraction.
   - Support RAM writes and ROM/cartridge read-only regions.
   - Support watchpoints, vector reads/writes, and trace hooks for OS-range
     calls.
   - Status: initial draft implemented. The bus now has writable RAM,
     read-only OS ROM, cartridge mapping, watchpoints, and `.CAR` parsing for
     the Action! type `$0F` 16K banked cartridge. Exact OSS bank-switch control
     semantics still need tracing/confirmation.

4. Boot/entry experiment.
   - Load cartridge plus OS ROM.
   - Reset the CPU from the reset vector.
   - Step with tracing enabled.
   - Confirm whether execution reaches cartridge code.
   - Record the first missing hardware or OS assumption.
   - Status: started. With `action.rom` plus `rev02.rom`, reset starts at
     `$C2AA` and the bootstrap CPU runs through the OS reset path until it
     reaches a `BRK`/zero byte near `$5003`, suggesting the next work is to
     inspect required OS/hardware initialization state rather than just adding
     opcodes.
   - PORTB/self-test update: `$D301` is now modeled as an I/O latch that gates
     the hidden self-test ROM. When bit 7 is clear, the OS ROM slice normally
     hidden behind `$D000-$D7FF` appears at `$5000-$57FF`. The boot path now
     executes real self-test ROM bytes at `$5003` and returns to OS code.
   - Current boot update: added the next batch of 6502 opcodes discovered by
     the reset path plus minimal `VCOUNT` (`$D40B`) and `RTCLOK` low-byte
     (`$0014`) progress models. A queued keyboard-code option (`--key-code`)
     now lets the OS poll at `$F2FD-$F312` observe `$02FC` (`CH`) changing from
     `$FF`; with `--key-code $21`, execution reaches Action! cartridge code and
     currently spends the long run window in a cartridge loop around
     `$532A-$532F`.

5. Trace infrastructure.
   - Add CLI flags:
     - `--trace-pc`,
     - `--trace-range $C000:$FFFF`,
     - `--watch $000E`,
     - `--max-cycles`.
   - Emit compact trace logs that help discover Action!'s compiler path.
   - Status: initial recorder implemented. The runner supports PC trace
     ranges, trace-until stops, watchpoints/watch ranges, recent-instruction
     history, cartridge-bank reporting, and watched bus-event reports.
     Current boot investigation shows the `$5003` stop is reached by an
     explicit OS `JMP $5003` at `$C3C1`, after writing `$02F3/$02F4`; this
     looks like a missing DOS/device boot target rather than a random CPU bug.

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
actionc-vm compile \
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
source = "../actionc-public-release/surveys/probes/original-compiler/abi_calls.act"
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
