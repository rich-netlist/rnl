# Roundtrip corpus (language 0.3)

Dedicated read → write → read cases for the solver-record interface, exercised by
[`roundtrip_profile.rs`](../../crates/rnl-cli/tests/roundtrip_profile.rs).
They use small local symbols with the standard package disabled, so changes to bundled
artwork do not change the writer checks. The other writeback tests still cover the
[conformance corpus](../conformance/README.md).

| Case | What it covers |
| --- | --- |
| `01-numbers-wires-text.sp` | Anonymous point, text and wire numbers; quoted wire name; keyed text keeps `dx` and writes only its free y; Unicode/markup; replacement of an old generated record. |
| `02-numbers-wires-text.scs` | The same cases through the Spectre carrier and host adapter. |
| `03-priority-body.sp` | A rule under a high-priority record head writes its usage without a priority; its low/high/normal body facts survive (10.5, 14.8). |
| `04-priority-usage.sp` | An explicit high application priority is written on the usage and still overrides its body. |
| `05-priority-disabled.sp` | A disabled rule record writes a disabled usage; its body stays disabled. |
| `06-subcircuit.sp` | The result belongs inside the subcircuit, before `.ends`, leaving its caller intact. |

Each source has a reviewed `*.record.rnl` (the exact record emitted) and
`*.expected.json` (the conformance-format effective model and diagnostic locations).
A deterministic `Solution` supplies positions and routes: these tests exercise the
writer independently of solver search, optimization and time limits. They do not claim
that the current built-in solver chooses every route the interface can represent.

For every case, the harness checks that all supplied decisions are writable, numbering
and input hashes read back as current, routes read back exactly, usage body priorities
are unchanged, and stripping the result recovers all authored text. It then reads the
written file as a solver (ignoring generated records), writes the result again, and
requires byte-identical replacement and an identical effective model. The cycle runs
with LF, CRLF and no final newline. The expected JSON is checked on the LF form.

Run from the repository root:

```sh
cargo test -p rnl-cli --test cli roundtrip_profile::
```

To update only this corpus after an intentional change, set `RNL_BLESS_ROUNDTRIP=1`
and run the same command, then review every changed record and JSON file. `RNL_BLESS`
and `RNL_BLESS_0_3` do not update this corpus. On the shared Windows build host, first
check that no other cargo/rustc is running and at least 8 GiB is free; set
`$env:CARGO_BUILD_JOBS = "4"` before running cargo.

The independent [0.4 corpus](../roundtrip-0.4/README.md) runs through the same harness.
`RNL_BLESS_ROUNDTRIP` never updates its references; use `RNL_BLESS_ROUNDTRIP_0_4` for those.
