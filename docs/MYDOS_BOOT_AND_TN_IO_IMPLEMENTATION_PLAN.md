# MyDOS Boot and TN I/O Implementation Plan

## Purpose

TN is a MyDOS-aware file manager. Its normal operation depends on more than
the VM's current in-memory `D:` host-file bridge provides: directory searches,
buffered file transfers, file creation and mutation, subdirectories, drive
selection, binary loading, and formatting.

The VM should not reproduce these MyDOS policies itself. Instead, it should
let the real Atari OS boot MyDOS and intercept the operating system's `SIOV`
entry at `$E459` as a logical sector-device boundary. MyDOS will then install
and run its own `D:` handler, while the VM supplies deterministic ATR-backed
disk sectors without emulating POKEY serial timing or a physical drive.

The target path is:

```text
TN -> CIOV -> MyDOS D: handler -> SIOV -> VM ATR sector service
```

This work must preserve the VM's existing cartridge compiler, standalone
object, host-file, and runtime-library workflows.

## Implementation Status

Slices 0 through 6 and Milestones A and B are complete as of 2026-08-19.
The implementation now provides:

- ownership-aware native CIO fallback and buffered host-file reads;
- validated 128-byte and 256-byte ATR images;
- read, status, and copy-on-write sector service at `SIOV`;
- deterministic boot of the bundled MyDOS 4.53/3 image without a cartridge;
- native MyDOS directory and file CIO integration tests;
- explicit CLI ATR writeback to a different path;
- a reproducible standalone TN 1.25 fixture with source and compiler
  provenance;
- a scripted TN test that browses D1, views a known text file, assigns D2 to
  the second panel, and copies the file between drives;
- byte-for-byte verification of a file larger than TN's available transfer
  buffer, proving the repeated `Bget`/`Bput` path.

TN's View output is asserted through the VM's structured channel-zero CIO
capture. Directory state is asserted through the decoded TN screen, while the
copy result is read back through the real MyDOS handler. All observed disk SIO
requests in this workflow are handled by the mounted ATR service.

Slices 7 through 9 remain: broader MyDOS mutation coverage, format-specific
SIO, and final public-surface stabilization.

The native-CIO foundation for Slice 7 is also complete. A focused test now
proves MyDOS commands 32 through 36 and 41 for rename, delete, mkdir,
lock/unlock, and current-directory changes; wildcard deletion; and command 39
loading and executing a known load-format object. The remaining Slice 7 work
is to drive representative mutations through TN's own UI rather than call the
same DOS interfaces from a test trampoline.

## Current Baseline

The VM already has useful prerequisites:

- a complete legal NMOS 6502 instruction core;
- Atari OS ROM and Action! cartridge mapping;
- execution from the Atari reset vector;
- a deterministic runner with PC, step, input-idle, and error stops;
- a high-level `CIOV` interceptor for explicitly registered host devices;
- scheduled keyboard and CIO input;
- enough headless OS state for Action! and generated-object tests.

The missing disk path is explicit. During the present cartridge boot, writes
to POKEY serial output schedule an artificial SIO timeout. The timeout redirects
the failed disk boot to the Action! cartridge cold-start vector. There is no ATR
model, mounted disk, `SIOV` interception, or writable sector service.

The current CIO host bridge also requires care before native DOS operation:

- `D:` opens are captured only when their normalized name exactly matches a
  registered host file;
- unknown CIO commands already fall through to the Atari OS;
- `CLOSE` is currently handled even when no harness device owns the IOCB;
- host `GETCHR` handles a single character in A, not a nonzero buffer/length
  transfer such as TN's `Bget`;
- host outputs must be declared before opening and do not form a directory.

## Architectural Boundary

### VM library owns

- parsing and validating ATR bytes;
- logical sector addressing and geometry;
- mounted drive state for units 1 through 8;
- read-only and copy-on-write sector data;
- decoding a Disk Control Block request at `SIOV`;
- deterministic SIO command results and status bytes;
- structured SIO observations;
- returning modified ATR bytes to the caller;
- execution policy for disk-boot profiles.

### CLI owns

- disk-image paths and host filesystem access;
- selecting drive number and mount policy;
- writing a modified image when explicitly requested;
- human-readable SIO traces and diagnostics;
- choosing an output path for copy-on-write images.

### MyDOS owns

- the `D:` CIO handler;
- Atari filename and wildcard semantics;
- directory record layout;
- allocation, VTOC, and subdirectory policy;
- file creation, rename, delete, lock, and unlock;
- current-directory and drive state used by TN;
- binary loading and DOS-specific XIO commands;
- filesystem error codes.

### Explicit non-goals

- POKEY serial or physical SIO timing;
- drive rotation, sector skew, baud rates, or command-frame waveforms;
- copy-protected or timing-sensitive disks;
- a general-purpose Atari emulator;
- implementing a second MyDOS filesystem in Rust;
- transparent host-directory mounting in the first implementation;
- guaranteeing that every arbitrary bootable ATR works.

## Public Model

The library API should be byte-oriented and independent of host paths. Exact
names may evolve, but the concepts should resemble:

```rust
pub struct AtrImage { /* validated header, geometry, sector data */ }

pub enum DiskWritePolicy {
    ReadOnly,
    CopyOnWrite,
}

pub struct MountedDisk {
    pub unit: u8,
    pub image: AtrImage,
    pub write_policy: DiskWritePolicy,
}

impl CompilerVm {
    pub fn mount_atr_bytes(
        &mut self,
        unit: u8,
        bytes: Vec<u8>,
        policy: DiskWritePolicy,
    ) -> Result<(), String>;

    pub fn mounted_atr_bytes(&self, unit: u8) -> Option<Vec<u8>>;
    pub fn disk_is_dirty(&self, unit: u8) -> bool;
}
```

The CLI should initially expose an explicit form such as:

```text
--disk 1:path/to/mydos.atr
--disk-writeback 1:path/to/result.atr
```

The default must be non-destructive. A mounted input image is read-only or
copy-on-write unless the user explicitly requests a writeback destination.

## ATR Model

The first ATR implementation should remain small and internal rather than add
a dependency merely for container parsing. It must:

- validate the 16-byte ATR header and magic;
- validate the declared paragraph count against available bytes;
- support 128-byte and 256-byte logical sector sizes;
- handle the first three 128-byte boot sectors correctly on 256-byte images;
- reject sector zero and out-of-range sectors;
- expose one-based logical sector reads and writes;
- retain the original geometry when serializing;
- track dirty state without changing the caller's input bytes;
- report malformed and unsupported images explicitly.

Support for less common ATR variants should be added only when a real MyDOS or
TN fixture demonstrates the need.

## High-Level SIOV Contract

When the CPU reaches `$E459` and a disk is mounted, the VM should inspect the
OS Disk Control Block at `$0300-$030B`:

- device and unit;
- command;
- direction/status flags;
- buffer address;
- timeout;
- requested transfer length;
- auxiliary sector/command values.

The interceptor should service logical drive requests directly and return as
though the OS SIO routine had completed. It must update the same DCB status,
CPU registers, flags, stack, and memory buffer that real callers observe.
These return details must be characterized with small synthetic programs before
MyDOS boot is used as the oracle.

The initial command set is deliberately narrow:

1. drive status;
2. read sector;
3. write/put sector for copy-on-write disks;
4. format commands required by the selected MyDOS version.

Command byte values and status payloads must be derived from traces and focused
tests rather than guessed. Unsupported commands must fail deterministically
with a structured observation; they must never silently succeed.

Every handled request should optionally record:

```rust
pub struct SioObservation {
    pub unit: u8,
    pub command: u8,
    pub sector: Option<u16>,
    pub buffer: u16,
    pub length: u16,
    pub handled: bool,
    pub status: u8,
    pub bytes_transferred: u16,
}
```

## Execution Profile and Boot Policy

Disk boot must be explicit. Add a disk-boot execution policy/profile only after
the read-only spike succeeds. It should:

- require an Atari OS ROM, using bundled AltirraOS by default;
- not require the Action! cartridge;
- require at least drive 1 to be mounted;
- reset through the normal Atari OS vector;
- disable the artificial disk-timeout-to-cartridge redirect;
- leave the mounted ATR installed across the run;
- expose SIO observations and modified disk bytes in the outcome.

Existing `OriginalCompiler` behavior must retain its current cartridge fallback
unless a mounted-disk mode was explicitly selected.

## CIO Ownership and Coexistence

The `CIOV` interceptor needs an ownership rule before MyDOS can operate:

- calls on an IOCB owned by `Q:`, `E:`, `S:`, or a registered host file may be
  handled by the harness;
- calls on an IOCB not owned by the harness must fall through to the OS;
- in native disk-boot mode, `D:` belongs to the installed DOS handler;
- closing an unowned IOCB must not be converted into harness success;
- legacy headless profiles may retain their existing empty-close convenience
  through an explicit policy rather than an unconditional special case;
- if native DOS and host files coexist, `H:` should be the unambiguous host
  device and `D:` should be reserved for DOS.

This routing change needs characterization tests because it affects resident
runtime tests as well as MyDOS.

## Implementation Slices

### Slice 0: characterize and repair current CIO behavior

- add a buffered host-file `GETCHR` test with nonzero buffer and length;
- implement the full buffered transfer and update IOCB length/status;
- characterize single-character `GETCHR` and EOF behavior unchanged;
- introduce explicit IOCB ownership checks;
- retain empty-close behavior only for profiles that require it;
- add a test proving an unowned IOCB reaches native OS CIO.

This slice is independently useful and prevents the host bridge from obscuring
later MyDOS failures.

### Slice 1: read-only ATR model

- add an internal `atr` module;
- parse 128-byte and 256-byte images;
- implement sector-to-byte-range translation;
- add malformed-header, truncated-image, first-three-sector, and boundary tests;
- mount ATR bytes on a numbered drive;
- expose read-only sector access and disk metadata;
- do not alter boot behavior yet.

### Slice 2: synthetic read-only SIOV service

- define the DCB constants and structured request decoder;
- intercept `$E459` only when a matching disk unit is mounted;
- implement drive status and sector reads;
- model success, missing drive, invalid sector, wrong length, and read-only
  failures;
- test return registers, flags, stack unwinding, DCB status, and buffer bytes;
- add structured SIO traces to the library and CLI formatter.

### Slice 3: read-only MyDOS boot spike

- add an experimental disk-boot path;
- suppress the current artificial disk failure only in this path;
- mount a known MyDOS boot ATR as drive 1;
- run from reset under a generous safety limit;
- stop on an audited MyDOS-ready memory/PC condition;
- confirm that DOS vectors and the `D:` handler are installed;
- record all SIO commands observed during boot.

This is the first decision gate. If MyDOS cannot reach a stable prompt without
broad hardware emulation, stop and reassess instead of adding guessed devices.

### Milestone A: deterministic read-only MyDOS boot

Milestone A is complete when:

- the same MyDOS ATR reaches the same ready condition repeatedly;
- boot uses logical SIOV sector reads, not POKEY timing;
- no cartridge fallback is involved;
- all existing VM and `actionc` runtime tests remain green;
- unsupported SIO requests are visible in the report.

### Slice 4: native DOS CIO handoff

- enable the ownership-based CIO routing in disk-boot mode;
- prove `D:` open/read/close calls reach the MyDOS-installed handler;
- keep explicit harness devices functional on their owned IOCBs;
- verify that MyDOS error/status values reach TN unchanged;
- add a small program that lists a directory through CIO rather than reading
  ATR sectors directly from the test.

### Slice 5: writable copy-on-write sectors

- add sector writes without modifying the original mounted bytes;
- implement dirty-sector tracking and ATR serialization;
- service the write variants actually emitted by MyDOS;
- test write/read persistence within one run;
- test read-only rejection;
- test caller retrieval of the modified ATR;
- add CLI writeback only to an explicit destination.

### Slice 6: TN browse, view, and copy

- build or install TN on a reproducible MyDOS ATR fixture;
- script TN startup and directory navigation;
- verify directory entries through structured screen or memory state;
- view a known text file;
- copy a binary file and compare its bytes in the resulting ATR;
- exercise files larger than TN's available transfer buffer so repeated
  `Bget`/`Bput` paths are covered;
- test EOF `$88` and disk-full/error propagation where practical.

### Milestone B: useful TN file operation

Milestone B is complete when TN can boot under MyDOS, list a disk, view a file,
and copy a file on a copy-on-write image without unhandled CIO or SIO requests.

### Slice 7: MyDOS mutation coverage

Through TN and the real MyDOS handler, test:

- create destination files on open mode 8;
- rename;
- delete;
- lock and unlock;
- create and enter a subdirectory;
- return to the parent directory;
- wildcard operations on tagged/all files;
- execute a known load-format binary through TN's command `$27` path.

No Rust implementation of these commands should be added: failures here
should identify missing sector/SIO behavior or CIO routing defects.

### Slice 8: format and multiple drives

- mount independently addressable drives 1 through 8;
- characterize and implement the SIO format command(s) used by MyDOS;
- operate only on a disposable copy-on-write image;
- verify the formatted image can be remounted and listed;
- test a TN copy between two mounted drives;
- preserve unit identity in every SIO observation and error.

### Milestone C: TN-required disk coverage

Milestone C is complete when all disk-facing TN commands operate through real
MyDOS on disposable images:

- browse and change directory;
- view and copy;
- rename and delete;
- lock and unlock;
- create subdirectories;
- load a binary;
- format;
- switch and copy between drives.

### Slice 9: stabilization and public surface

- remove the experimental marker from the disk-boot profile;
- document the library and CLI APIs;
- document supported ATR geometries and SIO commands;
- add compact SIO summaries to failure reports;
- verify deterministic copy-on-write output;
- keep MyDOS fixture provenance and redistribution terms explicit;
- update the VM README's scope without claiming general Atari emulation.

## Test Assets and Licensing

A deterministic boot test needs a known MyDOS ATR. Before embedding any image:

- record its exact version and SHA-256;
- establish redistribution terms;
- preserve a notice and corresponding source when required;
- prefer generating a small test ATR reproducibly when suitable tooling and
  licensed boot files are available;
- allow an external image for exploratory spikes, but do not make CI depend on
  an undocumented local path.

TN test disks should be generated from repository sources and disposable input
files. Mutation and format tests must never modify the checked-in baseline.

## Verification per Slice

Every slice runs in `actionc-vm`:

```sh
cargo fmt --check
cargo test
```

Slices that change the public VM revision also update the isolated
`actionc/tools/vm-runtime-tests` pin and run:

```sh
cargo test --locked
```

from that harness, followed by the main `actionc` test suite when CIO behavior
or execution profiles changed.

Disk-specific regression layers are:

1. pure ATR geometry tests;
2. synthetic DCB/SIOV tests;
3. Atari OS plus MyDOS boot tests;
4. small native CIO programs;
5. scripted TN workflows.

Failures should identify the lowest failing layer before an interactive TN run
is debugged.

## Risks and Mitigations

### Boot depends on unmodeled hardware

Mitigation: use the read-only boot spike as an explicit decision gate. Add only
small deterministic OS-visible state models; do not implement video or serial
timing to force progress.

### CIO interception hides native DOS behavior

Mitigation: make channel ownership explicit and trace every handled versus
passthrough call.

### ATR geometry corrupts writes

Mitigation: default to copy-on-write, test every sector boundary, and serialize
to a new byte vector. Never overwrite the input path implicitly.

### MyDOS version-specific assumptions leak into the VM

Mitigation: keep the VM at the SIO sector boundary. Version-specific ready
markers belong in tests, not production disk code.

### Interactive TN tests become fragile

Mitigation: assert stable memory, DOS-vector, directory, file-content, and CIO
facts. Use screen text only when it is the user-visible contract, and keep step
limits as safety bounds rather than success conditions.

### Existing runtime tests regress

Mitigation: preserve the current host-CIO policy outside the explicit disk-boot
profile and migrate routing in a characterized slice.

## Recommended Commit Boundaries

Use one commit per major slice:

1. buffered host reads and CIO ownership;
2. ATR parsing and read-only mounts;
3. synthetic SIOV read/status service;
4. experimental MyDOS boot;
5. native DOS CIO handoff;
6. copy-on-write sector writes;
7. TN browse/view/copy tests;
8. TN mutation tests;
9. format and multidrive support;
10. stabilization and documentation.

Do not combine ATR parsing, boot policy, CIO routing, and writable disk behavior
in one patch. Each boundary should remain independently reviewable and
bisectable.
