# tests

Test corpora shared by the implementations, and browser tests of the viewers' page.

| Path | Content |
| --- | --- |
| [`conformance/`](conformance/README.md) | Netlists with RNL, the effective models they must give, and SPICE/Spectre pairs that must give the same model. |
| [`roundtrip-0.4/`](roundtrip-0.4/README.md) | Solver-record read/write/read cases, exact written records and effective models; conformance writeback checks remain too. |
| [`integration/`](integration/README.md) | Cases for the solver: the netlists of the brainstorm on its algorithms, whose problems the runtime tests check. Future solver-to-display and editing checks. |
| [`viewer/`](viewer/README.md) | Browser tests of the viewers' shared page in Chromium, Firefox and WebKit, against a fake shell that serves Documents built natively from the corpus. |

Unit tests of each crate live with the crate under [`crates/`](../crates/README.md).
