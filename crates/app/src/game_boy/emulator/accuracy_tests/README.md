# Mooneye regression harness

Run `scripts/mooneye-tests.sh` from any directory. It downloads the pinned
`mts-20240926-1737-443f6e1` acceptance fixtures when necessary, then runs the
harness unit tests and ROM checks with per-ROM output and a summary. Ordinary
`cargo test` runs the harness unit tests but skips the ROM suite. With fixtures
already cached, use `cargo test -p app mooneye --offline -- --include-ignored --nocapture`.

The core runs headlessly, without bypassing peripheral behavior to accelerate
the tests. Each case has a limit of 2,000,000 emulation steps, including HALT,
DMA stalls, and interrupt entries (these are not all CPU instructions).
Only the documented Fibonacci pass signature or six `$42` failure bytes at an
executed `LD B,B` reporting breakpoint terminate a case as pass/fail. Other
uses of the instruction are ignored. Timeout, STOP, and invalid opcode are
separate outcomes.

## Models and coverage

[Upstream naming rules](https://github.com/Gekkio/mooneye-test-suite#test-naming)
define `G` as DMG/MGB, `S` as SGB/SGB2, and `C` as CGB/AGB/AGS. The harness
selects hardware independently of cartridge flags and runs every applicable
model it implements. Unsuffixed acceptance ROMs run on DMG and SGB; upstream
explicitly warns that not all acceptance tests apply to CGB. CGB runs require
an explicit `C`/`cgb` suffix. This snapshot therefore does not establish CGB
coverage. Unknown suffixes fail the harness rather than silently choosing a
model.

The core's DMG configuration represents the supported DMG A/B/C group. DMG0,
MGB, SGB2, and AGB/AGS-specific ROMs are reported as unsupported skips, not
emulation failures. The core does not model separate CGB revisions.

This harness currently covers the 75 pinned acceptance ROMs, not Mooneye's
`emulator-only`, `misc`, or manual suites. Cartridge headers select the mapper
and RAM size (the acceptance `sources-GS` DMA test requires MBC5 and RAM).

## Baseline maintenance

`baseline.tsv` records the exact outcome class for each ROM/model pair. Any
change fails the regression check, including a failure becoming a timeout,
an improvement, an unlisted case, or a baseline case no longer being run.
A successful harness run means the baseline matched; consult the printed
pass/fail/timeout totals for actual emulation accuracy.

To produce a candidate for review, set
`SHINING_MOONEYE_REPORT=/tmp/mooneye-candidate.tsv` when running the script.
The report is written even when outcomes differ. Review the changes before
updating `baseline.tsv`; the harness never updates it automatically. Keep
the fixture version pinned and review inventory/model changes when upgrading.
