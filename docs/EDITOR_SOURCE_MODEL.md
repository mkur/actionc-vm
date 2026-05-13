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

## Implemented Injection Strategy

For automation, prefer direct source injection over simulated typing, but inject
the editor data structure, not a contiguous text blob.

The VM now exposes `Bus::inject_action_source`, and the CLI can trigger it with
`--inject-source-at-pc <pc:path>`. A useful editor-idle trigger is currently
`$A2E0`, after the Action! editor has initialized its heap and scratch buffer.

The implemented flow:

1. Wait until Action! has completed editor initialization and `buf` points at a
   valid scratch buffer.
2. Free the current source line list through a small reproduction of Action!'s
   free-list behavior.
3. Allocate one live Action! heap block per host source line.
4. Build each line record with correct prev/next links, allocation size, line
   length, and ATASCII text bytes.
5. Set `top`, `bot`, `cur`, `vars.w1` top/bottom/current fields, and
   `vars.top1`.
6. Copy the first source line into the editor scratch buffer and clear dirty
   state.
7. Use keyboard simulation only for high-level monitor commands such as
   Shift+Control+M, `C` + Return, and `E` + Return.

The companion `Bus::action_editor_lines` API and
`--dump-editor-lines-at-pc <pc>` CLI option walk the live linked list and are
intended for sanity checks while automation evolves.

The compiler's completion signal is only a speaker beep. Both success and error
paths beep; errors additionally print an `Error:` line with a numeric code. The
VM therefore records speaker writes and can dump/decode the Atari text screen
with `--dump-screen-at-pc <pc>` or `--dump-screen-on-stop`.

Remaining questions:

- The current injector replaces the editor text directly; it does not call the
  original editor insertion routines.
- Compile success should be treated as "compile command completed, a speaker
  write occurred, and no visible `Error:` line was detected"; watching
  code-size/output ranges can make that stronger later.
- Window save state has been updated from observed fields, but should still be
  checked against more live editor navigation traces.
