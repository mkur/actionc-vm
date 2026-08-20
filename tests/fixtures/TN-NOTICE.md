# TN standalone integration fixture

`tn-standalone.com` is a standalone Atari load-format build of TOMS Navigator
1.25. It is used only by the MyDOS integration tests.

The fixture was built from the GPL-3.0-or-later Action! sources in the public
`actionc` repository at commit:

```text
ca5c2ed663a4752bb61145f11cfae624877a41c2
```

Source inputs:

```text
samples/tn/modern/TN.ACT
  SHA-256 c69b3e89106d9b2de57490b51c8b0395ecdc878989679de4330a3688196a5cc2
samples/tn/modern/LIB.ACT
  SHA-256 e01566f0edc024d1accfbf9113dba72b98f52cd09e15ae35f8470aff4dd16e3d
```

Build command, run from the `actionc` repository root:

```sh
actionc --profile modern --backend mir6502 --runtime standalone \
  --output TN.COM samples/tn/modern/TN.ACT
```

The compiler identified itself as:

```text
actionc 0.1.0 (vfs=165b51d70f9829b8eff5c90138eb091b10ca7d7afeffdda1544e54ac33ce6231)
```

Fixture checksum:

```text
SHA-256 2baaf8e1dd6810349f0366fd8b0b37632a149d5ba58ac201b1e124b7bc3a2578
```

TOMS Navigator and the linked standalone Action! runtime are distributed under
GPL-3.0-or-later, matching this repository's `LICENSE`.

## Binary-launch marker

`tn-launch-marker.com` is generated from the adjacent
`tn-launch-marker.act`. TN copies the object to a MyDOS disk and launches it;
successful execution writes `$A5` to `$4FFF` and enters a stable loop.

The fixture was built with `actionc` commit:

```text
241ad760ddff2b36cbf28e96bd157bbccb25ce10
```

Build command, run from the `actionc` repository root:

```sh
actionc --profile modern --backend mir6502 --runtime standalone \
  --output ../actionc-vm/tests/fixtures/tn-launch-marker.com \
  ../actionc-vm/tests/fixtures/tn-launch-marker.act
```

The compiler identified itself as:

```text
actionc 0.1.0 (vfs=165b51d70f9829b8eff5c90138eb091b10ca7d7afeffdda1544e54ac33ce6231)
```

Fixture checksums:

```text
tn-launch-marker.act
  SHA-256 73d32316d950bf17d9781e813e63cda6846775d24302c92a659838402e61c751
tn-launch-marker.com
  SHA-256 3e57ee98f65f06bec2f96e8bc683df6a1871d73cdd8c47500bf292176de06a94
```
