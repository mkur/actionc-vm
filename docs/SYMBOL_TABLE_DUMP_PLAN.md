# Action! Symbol Table Dump Plan

Goal: after the original Action! compiler VM compiles a source file, dump the
compiler's user symbol tables in a structured form. `actionc` comparison tools
can then align original compiler addresses by symbol/routine instead of only by
raw byte offset.

## 1. Use `ST.ACT` As The Format Spec

`ST.ACT` from the official runtime disk is the primary reference:

- `$B1/$B2` points to the global symbol table index.
- `$B3/$B4` points to the local symbol table index.
- Each index has two 256-byte pages:
  - high bytes at `base^`
  - low bytes at `base^ + 256`
- A nonzero high byte means the slot points to an Action string name.
- The entry immediately follows the string name:
  - `vtype`
  - `adr`
  - `numargs`
  - argument type bytes

## 2. Add A VM-Side Decoder

Add a reusable decoder to `actionc-vm` that reads the live VM memory and
returns:

- global index root
- local index root
- global entries
- local entries
- raw `vtype`, `adr`, `numargs`, raw argument type bytes
- best-effort decoded type/class strings

The first pass should match `ST.ACT` behavior: skip `vtype=$88`, sort entries by
name, and preserve raw fields even where type decoding is incomplete.

## 3. Add Stop-Time CLI Output

Add:

```sh
--dump-symbols-on-stop <path>
```

When execution stops, write the decoded symbol dump to the requested path.

## 4. Wire Probe Runner

Update `scripts/run-probe` to write a sibling symbol dump for each generated
probe, for example:

```text
outputs/vm/POINTERS.COM
outputs/vm/POINTERS.symbols.json
```

## 5. Capture Local Symbol Snapshots

`ST.ACT` also reveals a useful compiler hook: `Segvec` at `$04C6`. Action!
initializes this RAM vector as an `RTS`, and the compiler reaches it at
routine/segment boundaries. The VM can observe `PC=$04C6` directly instead of
hotpatching the vector to a custom routine.

Add:

```sh
--dump-symbol-snapshots-on-stop <path>
--action-symbol-hooks
```

With `--action-symbol-hooks`, the VM captures non-empty local symbol tables at
`Segvec` and also captures the final live local table when execution stops. The
final stop snapshot matters because the last compiled routine has no following
routine boundary to trigger `Segvec`.

Probe outputs now include:

```text
outputs/vm/POINTERS.symbol-snapshots.json
```

## 6. Consume Later In `actionc-compare`

After dumps exist, teach `actionc-compare` to accept original symbols and report
routine-relative gaps:

```text
original symbol Main $316C
compat   routine Main $316C
first diff at Main +$00FC
compat source: ip^ = -1
```

The original dump supplies symbols and addresses; the `actionc` map supplies
source ranges.
