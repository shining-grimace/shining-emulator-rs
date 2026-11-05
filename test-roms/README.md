# Validation ROMs

`catalog.json` pins curated headless and in-app selections from c-sp's compiled
test collection v7.0: 330 hardware cases using 222 binaries. The in-app subset
contains 34 hardware entries using 26 Mooneye/SameSuite binaries (832 KiB).
Each case declares its SHA-256 and hardware target. Reporting protocol,
emulated time budget and in-app eligibility use defaults where omitted.

## Prepare and run

```sh
python3 scripts/test-roms.py run                    # your website + upstream Blargg, then run all suites
python3 scripts/test-roms.py run --source upstream  # bootstrap until the real domain is configured
python3 scripts/test-roms.py run --offline audio    # cached fixtures; sound tests only
python3 scripts/test-roms.py run --offline samesuite
python3 scripts/test-roms.py run --offline mooneye
python3 scripts/test-roms.py run --offline blargg
```

Python 3 and the project's normal Rust build dependencies are required. No
assembler, audio device, display server, or running Bevy app is needed. Binaries
are downloaded from a pinned release rather than built locally. Ordinary
`cargo test -p app --offline` runs the harness/provider tests but ignores the ROM
regressions and never downloads fixtures.

`SHINING_TEST_ROM_ROOT` overrides `.test-roms/validation`. Preparation verifies
the website directory/catalog and each downloaded ROM. Blargg stays in a
separate upstream archive download whose SHA-256 is pinned. The upstream-only
bootstrap mode extracts all selected fixtures from that archive. The Rust runner
also verifies fixtures before execution.
`SHINING_TEST_ROM_FILTER` filters the model/path within a selected suite:

```sh
SHINING_TEST_ROM_FILTER=channel_3 python3 scripts/test-roms.py run --offline samesuite
SHINING_TEST_ROM_REPORT_DIR=/tmp/rom-results python3 scripts/test-roms.py run --offline
```

Candidate reports never update the checked-in baseline. Review each change,
including improvements, before editing
`test-roms/baselines/validation.tsv`.
A successful regression run means the baseline matched, **not** that every ROM
passed. Pass, fail, timeout, STOP, invalid opcode and unsupported-hardware skip
remain distinct. Fixture failures and an empty filter fail the harness.

Initial observed baseline (before audio accuracy fixes):

| Suite | Pass | Fail | Timeout | Unsupported |
| --- | ---: | ---: | ---: | ---: |
| Blargg | 35 | 8 | 14 | 0 |
| Mooneye | 144 | 45 | 4 | 8 |
| SameSuite APU | 2 | 63 | 0 | 7 |

## Coverage and verification

- Mooneye: acceptance, MBC1/MBC2/MBC5 emulator-only tests, and applicable CGB
  cases in `misc`. Unsupported hardware is explicitly recorded.
- Blargg: 24 individual DMG/CGB sound cases; individual CPU instruction and
  mem_timing-2 cases; instr_timing, interrupt_time (CGB), and halt_bug.
- SameSuite: all 70 APU binaries, plus DMG cases for the two documented
  monochrome DIV tests. Early-CGB/AGB variants have explicit unsupported skips;
  other CGB cases target CGB-E. This is a test target, not a claim that the
  emulator currently reproduces every CGB-E quirk.

Mooneye and SameSuite use their executed `LD B,B` register signature. SameSuite
requires functioning PPU timing and serial completion even without a display.
Blargg uses its signed cartridge-RAM status protocol where available and
completed serial output otherwise. Its halt_bug and interrupt_time cases use
the collection's expected screenshots after three emulated seconds, comparing
exact pixel structure under a consistent palette remapping; this checks result
content, not color accuracy. No peripheral behavior is bypassed or patched.
The runner permits CGB speed-switch STOPs and drains unused playback events.

The existing `scripts/mooneye-tests.sh` acceptance baseline remains available
at its original 2024 pin. The expanded catalog intentionally has its own
baseline and versioned binaries; neither silently upgrades the other.

These tests check emulated hardware behavior. They do not establish MIDI Graph
event ordering, output resampling quality, analog DAC/mixer fidelity, or HPF
response. Those require separate output tests.

## In-app selection

Enable **Game Boy validation ROMs** in Settings → ROM Providers. It starts
disabled and contributes no library entries while disabled. The permitted
selection comes from the versioned static web catalog configured in
`site-config.json`. Catalog sync and the first ROM load need network access;
ROMs and license notices are cached and verified for later offline use. Each
entry includes its hardware target in its name.
The loader selects that hardware and starts from reset without restoring SRAM
or automatic save states. Result screens are displayed by the ROMs themselves.

The selection contains SameSuite DIV, pulse, wave and LFSR diagnostics, and
Mooneye DIV, timer, DMA, boot-I/O and unused-bit checks. `in_app: true` is the
exact list. It contains no Blargg ROMs, unsupported revision cases, utilities
or manual-only tests.

Generate the complete website upload tree and archive with:

```sh
python3 scripts/test-roms.py export --offline
```

See [HOSTING.md](HOSTING.md) for the directory schema, exact upload paths,
server setup, domain replacement and verification commands. The public
headless catalog contains 273 permitted cases using 181 ROMs; Blargg remains
in the private developer fixture cache.

## Provenance and licensing

Compiled collection: https://github.com/c-sp/game-boy-test-roms/releases/tag/v7.0

Its changelog identifies Mooneye `8d742b9d55` and SameSuite `f71b4b3c37`.
Upstream source remains the reference for each test's expectations. The
collection's own MIT license does not relicense third-party suites.

Mooneye is MIT-licensed and SameSuite is X11-licensed. Preserve the copyright
and permission notices in `licenses/` and the generated website license directory when
distributing the app or a permitted ROM pack. Both allow commercial use and
redistribution with those notices. SameSuite's hardware definitions carry CC0.

**Blargg:** no explicit redistribution license was found in the reviewed
gb-test-roms sound distribution. Its binaries are fetched only into the ignored
developer fixture cache for automated headless testing; they are excluded from
the app provider and website upload pack. Resolve redistribution permission
before publishing them. This is an unresolved license status, not a claim that
developer downloading or testing has an explicit license grant.

## Catalog defaults, licenses and the test fixture

Catalog integrity uses **SHA-256 only**. An omitted headless `protocol` means
`breakpoint`; omitted `seconds` means a 12-second emulated budget. Non-default
protocols and budgets remain explicit in the developer manifest. The in-app
catalog omits all execution settings and membership flags: its role already
identifies the selection. Author, license, hardware and name are retained as
useful identifying metadata. `directory.json` needs only paths and SHA-256;
there is no separate checksum listing or ownership marker in the upload.

The app's header-normalized ROM IDs also use SHA-256. Normalization still
zeroes cartridge header bytes, so these IDs differ from exact-file hashes.
No settings or cache migration is provided for the previous identifier scheme.

`Collection-MIT.txt` is the upstream compiled collection's `LICENSE`, renamed
locally to distinguish it from the original ROM licenses. It credits
Christoph Sprenger (2020), maintainer of `c-sp/game-boy-test-roms`. It is kept
as the notice/provenance for the collection we downloaded, not as a replacement
license for Mooneye, SameSuite, or Blargg. Original file:
https://github.com/c-sp/game-boy-test-roms/blob/v7.0/LICENSE

The download/cache round-trip test uses Mooneye's `acceptance/div_timing.gb`
from the prepared collection, respecting `SHINING_TEST_ROM_ROOT`. It is ignored
by ordinary `cargo test` and included by `python3 scripts/test-roms.py run`.
There are no ROM binaries committed solely as unit-test fixtures.

Both reviewed baselines are under `baselines/`: `mooneye-acceptance.tsv` for
the original 2024 acceptance pin and `validation.tsv` for the expanded catalog.
They remain separate because their versions and coverage differ.

## One operational command

The four new scripts have been consolidated into `scripts/test-roms.py`:

| Subcommand | Operation |
| --- | --- |
| `prepare` | Fetch/verify fixtures without running Rust tests |
| `run` | Prepare fixtures and execute selected headless regressions |
| `export` | Prepare permitted files and generate the website tree/ZIP |
| `check-site` | Verify the export and test HTTP import/corruption handling |

The pre-existing `scripts/mooneye-tests.sh` remains a separate entry point for
its original pinned baseline; it was not one of the four new scripts.

## Runtime fetching in the app

1. Home-screen synchronization filters out disabled providers, then runs provider
   work on the existing I/O task pool. The validation provider delegates to
   `storage/validation_roms/site.rs`.
2. That adapter uses the existing `http_get_bytes` client to fetch `directory.json`
   and the selected release's `in-app.json`. It verifies their schema, pinned
   release, catalog hash and exact known selection before returning library rows.
3. Selecting a ROM starts the existing asynchronous loader. Its validation branch
   delegates to `storage/validation_roms/download.rs`, which fetches the relevant
   license notice and just that ROM, checks SHA-256 and caches both.
4. Later loads reuse valid cached files without fetching them again. The loader
   applies the catalog's hardware target and bypasses saved-state/SRAM restoration.

The app does not download an entire suite archive or extract ZIPs at runtime.
It intentionally pins the curated release and reuses the normal HTTP client,
background tasks, library metadata, cache and emulator-loading path.
