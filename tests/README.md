# tests

Test corpora shared by the implementations.

| Path | Content |
| --- | --- |
| [`conformance/`](conformance/README.md) | Netlists with RNL, the effective models they must give, and SPICE/Spectre pairs that must give the same model. |
| [`roundtrip/`](roundtrip/README.md) | Future dedicated read, modify and write-back cases. The writeback checks run over the conformance cases for now. |
| [`integration/`](integration/README.md) | Future solver-to-display and editing checks. |

Unit tests of each crate live with the crate under [`crates/`](../crates/README.md).
