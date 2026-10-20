# RNL brand assets

The source is [rnl-logo.sp](rnl-logo.sp), copied unchanged from the original RNL 0.3 logo.
It defines its own symbols and styles and contains no text. Keep its circuit and line weights intact.

| File | Use |
| --- | --- |
| `rnl-logo.sp` | Editable source; render with the RNL CLI |
| `rnl-logo.svg` | Transparent original colours for light backgrounds |
| `rnl-logo-dark.svg` | Transparent colours for dark backgrounds, using the viewer's `darkColour` rule |
| `rnl-logo-small.svg`, `rnl-logo-small-dark.svg` | Transparent, pixel-fitted light/dark logos for the site's 24px navigation mark |
| `rnl-logo-{16,24,32,48,64,128,256,512,1024}.png` | Transparent square PNGs in the original black/orange colours for light backgrounds |
| `rnl-logo-white-{16,24,32,48,64,128,256,512,1024}.png` | The same square artwork on a fully opaque white (`#ffffff`) background |
| `rnl-icon.svg` | Responsive 1024-unit icon, dark logo on `#132033`, corner radius 180 |
| `rnl-icon-1024.png` | Chromium rasterization used as Tauri's input |
| `rnl-icon-16.png`, `rnl-icon-24.png`, `rnl-icon-32.png` | Small icons with stronger strokes, also packed into the desktop icon containers |

The transparent logos have 6% of the painted width as padding on every side, including strokes.
The icon centres the painted bounds and makes their longer side 72% of the canvas.
At 32px and below, dedicated 16/24/32px variants fit horizontal and vertical strokes to pixel
centres, with at least a 1px stroke and a 35% weight increase. All circuit paths and orange
junctions use `crispEdges` rendering. The hollow ground becomes a solid stepped triangle:
3 pixels high at 16/24px, 5 pixels high at 32px. The rounded backdrop retains its smooth edge.
SVG media queries select the matching small variant for browser favicons (16px up to 20px,
24px up to 28px, and 32px up to 32px). Raster icons are rendered directly at their target size.
Larger icons and the transparent homepage logos retain their original line weights; the source circuit stays unchanged.

Both light-background PNG sets use the same square framing and 72% painted extent as the icon.
The plain filenames have no backdrop; the `white` filenames have a solid white background,
including the corners. Sizes 16/24/32 use the approved hard-edge small artwork and solid stepped
ground; sizes 48 and above use the original smooth artwork. Each PNG is rasterized directly
at its named pixel dimensions rather than resized from a larger image. The portrait SVG
`rnl-logo.svg` keeps its original framing for the homepage.

## Regeneration

From the repository root, install the existing tools once:

```sh
cargo build -p rnl-cli
npm ci --prefix tests/viewer
node tests/viewer/node_modules/@playwright/test/cli.js install chromium
npm ci --prefix apps/desktop
```

Then regenerate everything with one command:

```sh
node scripts/brand.mjs
```

To export both the transparent and white-background PNG sets from the committed `rnl-logo-small.svg`,
without rebuilding the RNL CLI or invoking Tauri:

```sh
node scripts/brand.mjs --light-pngs
```

This shorter command needs only the existing Playwright/Chromium installation in `tests/viewer`.

The script uses `target/debug/rnl` (`rnl.exe` on Windows); `RNL_BIN` can select another freshly built CLI.
It invokes the installed Tauri CLI's `icon` command, equivalent to `npx tauri icon`, in a temporary
folder and copies only the five icons listed in `apps/desktop/tauri.conf.json`. Unused Appx,
Android and iOS outputs are discarded; the bundle configuration stays unchanged.
The 32px desktop PNG and ICO's 16/24/32px frames use the small Chromium PNGs. Larger Windows ICO
frames render the 32px variant's geometry at their own size, so high-DPI taskbars and Explorer use
the same small-logo strokes without shrinking the 1024px logo. The Windows viewer also explicitly
uses the 32px PNG for its window icon. Tauri also encodes
the small ICNS RGB/mask resources, including the 16pt@2x (32 physical pixels) representation;
larger ICNS resources retain their normal strokes. ICNS blocks are sorted by type for reproducibility.

The same command also updates:

- `site/src/assets/favicon.svg`, `favicon-32.png` and `apple-touch-icon.png` (180px).
- `apps/viewer-ui/icons/`: copies of those favicons, carried into the web viewer by its existing build.
- `site/src/assets/brand/`: copies of the transparent homepage and small navigation logos.

The site keeps these generated copies in its source assets so its Python-only build and downloadable
zip work without Rust or Chromium. `site/build.py` publishes them under both `zh/assets/` and
`en/assets/`, and embeds them as data URLs in the standalone pages, which still work offline.
Normal site and viewer pages use relative icon paths, including when hosted under `/rnl/`.
The standard page template uses the public site's English asset URLs so its downloadable HTML,
version pages and comparison pages share the same icons without changing the standard publisher.

The homepage's `.hero-logo` and the top bar's 24px `.brand-logo` are decorative (`alt=""`).
Both follow the site's `html[data-theme]` and system preference rules. The top bar mark is
part of the existing home link; the hero has no click action. Its `cap`-based image height
accounts for the SVG padding, matching the painted height to the wordmark's capital letters
(about `.70em` in the site's display font, with an `em` fallback). The SVG's own side padding
provides the spacing before RNL, with no additional flex gap, so the mark reads like a prefix.

## Renderer regression check

```sh
node --test scripts/brand.test.mjs
```

Pages CI runs this after building the current CLI and installing its existing Playwright Chromium.
It rerenders and crops the source, compares `rnl-logo.svg` byte for byte, and also checks the dark
derivative. Renderer output changes fail with `run node scripts/brand.mjs and review the logo`.
Regenerate when that check fails, then inspect the new logos and small icons before committing.
The colour test compares the copied helper against `apps/viewer-ui/background.js`, including
threshold boundaries, short hex values and alpha. PNG, ICO and ICNS files are not compared in CI.

The site's Chromium and WebKit checks cover the homepage themes, narrow layout and favicon URLs.
For visual review, compare 16, 24, 32, 48 and 256px icons on light and dark backgrounds.

`rnl check brand/rnl-logo.sp`: **0 errors, 0 warnings, 5 hints**. The hints identify intentional
overrides of `$base`, `$signal`, `$NMOS`, `$GROUND_SYMBOL` and `$JUNCTION_SYMBOL` from the standard
package. The source is preserved rather than changing it to silence those hints.
