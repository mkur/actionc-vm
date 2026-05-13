# Action! Editor Source Model

This note records what the original Action! editor source implies for compiler
automation in the VM.

## Source Storage

The editor source is not stored as one contiguous text buffer. The editor uses a
private heap and stores source as a doubly linked list of separately allocated
line records.

Relevant original sources:

- `code/JAC/src/editor/EDIT.INI.asm`
- `code/JAC/src/editor/EDIT.MEM.asm`
- `code/JAC/src/compiler/LEXICON.asm`
- `code/JAC/src/ampl/AMPL.MON.asm`

`EDIT.INI.asm` initializes Action!'s heap from Atari `MEMLO` to `MEMTOP`, then
allocates a separate edit scratch buffer. The scratch buffer pointer is stored in
zero page `buf` at `$9B`. That buffer is used for the current line, command
input, file I/O, and compiler lexing, but it is not the whole source program.

`EDIT.MEM.asm` allocates and links source lines through `mem.instln`. A line
record has this shape:

```text
0  previous line pointer lo
1  previous line pointer hi
2  allocation size lo
3  allocation size hi
4  next line pointer lo
5  next line pointer hi
6  line length byte
7  line text bytes...
```

The allocation size includes the allocator header. `mem.instln` requests
`line_length + 3` bytes through `getmem`; `getmem` adds the allocator header, so
the resulting allocation covers the six-byte line header plus the length byte
and line payload.

The editor keeps the active window state in zero page:

```text
top     $90  pointer to first line
bot     $92  pointer to last line
cur     $94  pointer to current line
buf     $9B  pointer to scratch line buffer
dirtyf  $C3  current scratch line is dirty
```

Saved window state lives under `vars.w1` at `$0480` and, when a second window is
active, `vars.w2` at `$04B1`.

## Compiler Traversal

The compiler does not read a flat source buffer. `compiler.lexicon.nextline`
walks the editor line list:

1. If compiling editor text, `top+1` must be non-zero.
2. `mainio.ldbuf` copies the current linked-list line into `buf`.
3. `curnxt` records the source line pointer for diagnostics.
4. `mainmsc.editor.nextdwn` advances `cur` to the next linked-list node.
5. An internal EOL byte is appended in `buf`.
6. Lexing proceeds from the scratch buffer.

The monitor entry saves the current source presence via:

```text
lda top+1
sta vars.top1
```

The compiler later restores `top+1` from `vars.top1` before compiling editor
text. That means source injection must preserve a coherent editor/window state
before entering the monitor or must update `vars.top1` as part of the injection.

## Injection Strategy

For automation, prefer direct source injection over simulated typing, but inject
the editor data structure, not a contiguous text blob.

Minimum viable injection plan:

1. Wait until Action! has completed editor initialization and `buf` points at a
   valid scratch buffer.
2. Clear or replace the current source line list.
3. Build one line record per source line in free RAM, with correct prev/next
   links and length-prefixed ATASCII text.
4. Set `top`, `bot`, and `cur` to the injected list.
5. Update the saved current-window record (`vars.w1`, especially current-line
   fields) or invoke the editor's save/restore path after injection.
6. Ensure monitor/compiler state sees the program, either by entering monitor
   after `top` is valid or by setting `vars.top1` consistently.
7. Use keyboard simulation only for high-level monitor commands such as
   Shift+Control+M, `C` + Return, and `E` + Return.

Open questions for implementation:

- The safest address range for injected line allocations should be discovered
  from the live Action! heap after boot rather than hardcoded.
- The allocator free list should either be updated to exclude injected records,
  or injection should call/reproduce the allocator's allocation logic.
- Window save state should be verified against a live trace before relying on
  only `top`, `bot`, and `cur`.
