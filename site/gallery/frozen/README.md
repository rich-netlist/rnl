# Frozen groups of the gallery of the standard packages

A file here is the data of one group of `packages.html`, exactly as `site/gallery/build.mjs` writes it into
`site/dist/gallery/` (`standard-0.3.js`): the packages of the series, their statements, and every symbol as an SVG drawn
by the core. The build copies it into the site as it is and does not read or draw `libraries/standard/<series>/` for it.

They are the standard series 0.2 and 0.3, which are archived: the core after them no longer reads those languages, and
their package files leave `libraries/`, so the pages of both series can no longer be drawn from them. The data is the
page as it was when it was frozen, not a snapshot to keep up to date; a change of the core or of the page's script does
not touch it. Statements carry the paths `libraries/standard/<series>/…` that they had in the repository then.

To draw a series from its package files again (to compare, or to freeze one more), point the build at an empty
directory and keep what it writes:

```bash
mkdir -p /tmp/no-frozen
node site/gallery/build.mjs --libraries libraries --frozen /tmp/no-frozen --out /tmp/gallery-out
cp /tmp/gallery-out/gallery/standard-0.3.js site/gallery/frozen/
```

These files are generated; do not edit them by hand.
