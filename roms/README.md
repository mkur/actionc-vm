# Bundled ROM image

| File | Size | SHA-256 | Use |
| --- | ---: | --- | --- |
| `action.rom` | 16400 | `b4a3a399f4f1e8c20f4b1cbc3f6e2fbcef342c36d2c252f903938e93a502c166` | Default Action! 3.6 cartridge for cartridge-backed VM profiles |
| `altirraos-xl.rom` | 16384 | `9de5a313fe3946f04fe236a8d3ceacb471fbed4ec5fc5db009732e1169946ccf` | Default Atari XL/XE OS for cartridge-backed VM profiles |

`action.rom` is the Action! 3.6 OSS 16K cartridge, licensed under GPL version
3 or any later version. Its exact source revision, build path, hashes, and
byte-for-byte comparison with the upstream reference ROM are recorded in
`ACTION-ROM-NOTICE.md`. A pinned corresponding-source snapshot is preserved
under `source/`.

`altirraos-xl.rom` is AltirraOS XL/XE 3.11. It was extracted byte-for-byte
from `ROM_altirraos_xl` in Atari800's checked-in
`src/roms/altirraos_xl.c` at commit
`bbe287d6d2c233bc8bad92ed2b2637f6a3859eb6`:

https://github.com/atari800/atari800/blob/bbe287d6d2c233bc8bad92ed2b2637f6a3859eb6/src/roms/altirraos_xl.c

Its copyright and redistribution notice is preserved in
`ALTIRRAOS-LICENSE`. The corresponding source is available in Atari800's
`emuos` directory:

https://github.com/atari800/atari800/tree/bbe287d6d2c233bc8bad92ed2b2637f6a3859eb6/emuos
