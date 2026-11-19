<!-- timeline-summary
{
  "en": "Describe the main released changes in one sentence."
}
-->

# Release name and version

Describe the resulting behavior and any migration steps. Replace the summary sentence
before release. For a standard release, include the same summary block in the annotated
spec tag's message. For Viewer/core, keep it in that version's Markdown release notes.

A Viewer release ends with its downloads, which `viewer-release.yml` attaches. Copy the table of
the last Viewer notes, with the version changed: the desktop viewer's twelve files, then the local
viewer's two packages for older enterprise Linux, then the checksums:

| File | Use |
| --- | --- |
| … | (the desktop viewer's twelve files, as in the last notes) |
| `rnl-viewer-local-X.Y.Z-linux-x86_64.tar.gz` | The local viewer for older enterprise Linux (CentOS 7, RHEL 8 and 9, any Linux x86_64 of kernel 3.2 or newer): one static program that serves this viewer on this computer and shows it in a browser; with `get-firefox.sh` for a computer whose browser is too old. |
| `rnl-viewer-local-X.Y.Z-linux-x86_64-with-firefox.tar.gz` | The same with Mozilla's official Firefox ESR, unmodified, beside the program. |
| `SHA256SUMS.txt` | SHA-256 checksums for the fourteen downloads. |
