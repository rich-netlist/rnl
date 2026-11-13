# Roundtrip corpus (language 0.4)

Six deterministic solver-record cases mirror the coverage of the independent
[0.3 corpus](../roundtrip/README.md): SPICE and Spectre carriers, numbered points,
text and wires, keyed text offsets, generated usages, body/application/disabled
priorities, and replacement within a subcircuit.

The fixtures use local symbols without a standard package. The priority cases
select unconnected pins with `SELECT-PIN`; the lower pin is named `%bottom` so
generated usages can write back its exact reference. The distance interval has
an explicit zero lower bound. The other pins need no local names. Each 0.4 usage
has its own point, which the generated record preserves independently of its body.

The shared harness supplies positions and routes, checks numbering and input hashes,
keeps all authored text, verifies every route and priority, and requires identical
replacement on a second write. Each case runs with LF, CRLF and no final newline.
The reviewed `*.record.rnl` files hold exact written records and `*.expected.json`
files hold effective models and diagnostic locations.

Run both language corpora:

```sh
cargo test -p rnl-cli --test cli roundtrip_profile::
```

After an intentional, reviewed change, `RNL_BLESS_ROUNDTRIP_0_4=1` updates only
this corpus. `RNL_BLESS_ROUNDTRIP`, `RNL_BLESS`, and `RNL_BLESS_0_3` leave it alone.
The existing 0.3 fixtures and references remain in their own directory.
