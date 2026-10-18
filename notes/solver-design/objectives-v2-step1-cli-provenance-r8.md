# R8: original input provenance across 0.2 upgrades

The real CLI exposed an empty input manifest after automatic 0.2→0.3 upgrading. The second pass reads upgraded texts from memory, bypassing the provider tracker. Existing code restored the first pass read list but accidentally retained only the second pass invocation manifest.

The correction preserves the first pass invocation, then appends dependencies first encountered by the second pass. A repeated path keeps the original provider-decoded text SHA; the internal version-upgrade overlay is not substituted for the original input. Solver input, options, candidate search, geometry, records and the existing read list are unchanged. An empty old manifest is not evidence that no external dependencies existed.

Independent source review accepted candidate02, manifest SHA256 `40085796320a3b746f195cb7eaf09247583cc36e6bf7a8582d359b30a4ca5773`. The real CLI regression uses a BOM-encoded 0.2 entry and an included file, checks both decoded hashes and confirms that the output actually upgraded to0.3. Targeted execution passed1/1 (compile10.21s/test1.33s), log `target/objectives-v2-validation/step1-gates/R8-T-cli-provenance-01.log`.

Final CLI source/binary rebinding and a real T1 old/new written-byte/scoring/work comparison remain pending. Earlier geometry/quality evidence retains its original source/binary labels; older timings are not relabelled as measurements of this metadata correction. The complete T14 prior-CLI run finished14/14 with84 successful command steps and18 nonempty drawing levels; its actual0b5afb tag and artifacts remain unchanged. No long T6 performance rerun is required solely for this metadata correction.
