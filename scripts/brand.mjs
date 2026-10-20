// Generate the committed brand assets with the existing CLI, Chromium and Tauri tools.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const viewerRequire = createRequire(join(root, "tests/viewer/package.json"));
export const { chromium } = viewerRequire("@playwright/test");
export const ICON_BACKGROUND = "#132033";
const rnl = process.env.RNL_BIN || join(root, "target/debug", process.platform === "win32" ? "rnl.exe" : "rnl");
const number = n => String(Number(n.toFixed(6)));

// Copied from apps/viewer-ui/background.js: importing that module also initializes the viewer DOM.
// brand.test.mjs checks this copy against the viewer's actual implementation.
export function darkColour(colour) {
  if (!/^#(?:[\da-f]{3,4}|[\da-f]{6}|[\da-f]{8})$/i.test(colour)) return colour;
  const hex = colour.slice(1);
  const full = hex.length <= 4 ? [...hex].map(c => c + c).join("") : hex;
  const rgb = [0, 2, 4].map(i => parseInt(full.slice(i, i + 2), 16) / 255);
  const lo = Math.min(...rgb), hi = Math.max(...rgb);
  const lightness = (lo + hi) / 2;
  const reflected = lightness < 0.25 ? 1 - lightness : lightness;
  if (reflected === lightness) return colour;
  const shift = reflected - lightness;
  return "#" + rgb.map(c => Math.round((c + shift) * 255).toString(16).padStart(2, "0")).join("") + full.slice(6);
}

/** Render afresh, remove the paper, and frame the painted bounds with 6% of their width per side. */
export async function renderLogo(page) {
  const temporary = mkdtempSync(join(tmpdir(), "rnl-brand-"));
  try {
    execFileSync(rnl, ["render", "brand/rnl-logo.sp", "-o", temporary], { cwd: root, stdio: "pipe" });
    let svg = readFileSync(join(temporary, "rnl-logo.svg"), "utf8").replace(/\r\n/g, "\n");
    if (!/<rect class="backdrop"[^>]*\/>/.test(svg)) throw new Error("The renderer's backdrop changed; review brand.mjs");
    svg = svg.replace(/\n<rect class="backdrop"[^>]*\/>/, "");
    await page.setContent(svg);
    const bounds = await page.evaluate(() => {
      const elements = [...document.querySelectorAll("svg path, svg circle, svg ellipse, svg line, svg polyline, svg polygon, svg rect, svg text")];
      if (!elements.length) throw new Error("The logo has no drawing");
      const boxes = elements.map(el => {
        const box = el.getBBox();
        const style = getComputedStyle(el);
        // This logo uses round caps and joins. getBBox() excludes strokes in Chromium.
        const half = style.stroke === "none" ? 0 : parseFloat(style.strokeWidth) / 2;
        return [box.x - half, box.y - half, box.x + box.width + half, box.y + box.height + half];
      });
      return [Math.min(...boxes.map(b => b[0])), Math.min(...boxes.map(b => b[1])),
        Math.max(...boxes.map(b => b[2])), Math.max(...boxes.map(b => b[3]))];
    });
    // Round the browser's float32 SVG geometry before doing layout arithmetic.
    const [x0, y0, x1, y1] = bounds.map(n => Number(n.toFixed(4)));
    const width = x1 - x0, height = y1 - y0, pad = width * 0.06;
    const box = [x0 - pad, y0 - pad, width + pad * 2, height + pad * 2].map(number).join(" ");
    svg = svg.replace(/^<svg\b[^>]*>/, tag => tag
      .replace(/ (?:width|height)="[^"]*"/g, "")
      .replace(/viewBox="[^"]*"/, `viewBox="${box}"`));
    return { svg, bounds: { x: x0, y: y0, width, height } };
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

export function darkLogo(svg) {
  return svg.replace(/\b(stroke|fill)="(#[\da-f]+)"/gi, (_, attribute, colour) => `${attribute}="${darkColour(colour)}"`);
}

// Tauri emits the same ICNS representations in a random order. Keep their payloads untouched,
// but sort the resource blocks by four-character type so repeated generation is byte-identical.
function icnsBlocks(file) {
  const data = readFileSync(file), blocks = [];
  if (data.toString("ascii", 0, 4) !== "icns" || data.readUInt32BE(4) !== data.length) throw new Error("Invalid ICNS header");
  for (let offset = 8; offset < data.length;) {
    const size = data.readUInt32BE(offset + 4);
    if (size < 8 || offset + size > data.length) throw new Error("Invalid ICNS resource");
    blocks.push(data.subarray(offset, offset + size));
    offset += size;
  }
  return new Map(blocks.map(block => [block.toString("ascii", 0, 4), block]));
}

function writeIcns(file, blocks) {
  const sorted = [...blocks.values()].sort((a, b) => Buffer.compare(a.subarray(0, 4), b.subarray(0, 4)));
  const header = Buffer.alloc(8);
  header.write("icns");
  header.writeUInt32BE(8 + sorted.reduce((size, block) => size + block.length, 0), 4);
  writeFileSync(file, Buffer.concat([header, ...sorted]));
}

// Windows can choose larger frames even for its taskbar at higher DPI. Keep the 32px logo's
// heavier, simpler strokes in those frames too, rendering its geometry at the requested size.
export async function windowsIco(file) {
  const data = readFileSync(file), count = data.readUInt16LE(4);
  const directory = Buffer.from(data.subarray(0, 6 + count * 16)), images = [];
  const icon = readFileSync(join(root, "brand/rnl-icon.svg"), "utf8").replace(/<style>.*?<\/style>/s,
    "<style>.icon-normal, .icon-small { display: none; } .icon-32 { display: inline; }</style>");
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    let offset = directory.length;
    for (let i = 0; i < count; i++) {
      const entry = 6 + i * 16, size = data[entry] || 256;
      const image = size <= 32 ? readFileSync(join(root, `brand/rnl-icon-${size}.png`)) :
        await png(page, icon, size);
      directory.writeUInt32LE(image.length, entry + 8);
      directory.writeUInt32LE(offset, entry + 12);
      images.push(image);
      offset += image.length;
    }
  } finally {
    await browser.close();
  }
  writeFileSync(file, Buffer.concat([directory, ...images]));
}

export function iconSvg(logo, bounds, { background = ICON_BACKGROUND } = {}) {
  const scale = 1024 * 0.72 / Math.max(bounds.width, bounds.height);
  const x = 512 - (bounds.x + bounds.width / 2) * scale;
  const y = 512 - (bounds.y + bounds.height / 2) * scale;
  const body = logo.replace(/^<svg[^>]*>\n/, "").replace(/\n<\/svg>\n$/, "");
  const stroke = Number(logo.match(/stroke-width="([\d.]+)"/)[1]);
  // Small circuits use hard edges. Fit orthogonal strokes to pixel centres and replace
  // the hollow ground with a solid stepped triangle, so its opening cannot collapse.
  const transform = [x, y, scale].map(n => Number(number(n)));
  function pixelHint(size) {
    const [tx, ty, s] = transform;
    const snap = (value, origin) => ((Math.round((value * s + origin) * size / 1024 - .5) + .5) * 1024 / size - origin) / s;
    const ground = body.replace(/<g data-id="%g0"><path\b[^>]*\/><\/g>/g, tag => {
      const d = tag.match(/\bd="([^"]+)"/)[1];
      if (!/Z/i.test(d)) return tag; // Keep the ground's horizontal lead.
      const values = d.match(/-?\d+(?:\.\d+)?/g).map(Number);
      const xs = values.filter((_, i) => i % 2 === 0), ys = values.filter((_, i) => i % 2);
      const left = Math.round((Math.min(...xs) * s + tx) * size / 1024);
      const centre = Math.floor(((Math.min(...ys) + Math.max(...ys)) / 2 * s + ty) * size / 1024);
      const width = Math.max(2, Math.round((Math.max(...xs) - Math.min(...xs)) * s * size / 1024));
      const point = (px, py) => `${number((px * 1024 / size - tx) / s)},${number((py * 1024 / size - ty) / s)}`;
      const rows = [];
      for (let dy = 1 - width; dy < width; dy++) {
        const right = left + width - Math.abs(dy), top = centre + dy;
        rows.push(`M${point(left, top)} L${point(right, top)} L${point(right, top + 1)} L${point(left, top + 1)} Z`);
      }
      const colour = tag.match(/\bstroke="([^"]+)"/)[1];
      return `<g data-id="%g0"><path d="${rows.join(" ")}" fill="${colour}" stroke="none"/></g>`;
    });
    return ground.replace(/<path\b[^>]*\/>/g, tag => {
      if (tag.includes('stroke="none"')) return tag;
      tag = tag.replace(/\bstroke-width="[\d.]+"/,
        `stroke-width="${number(Math.max(stroke * 1.35, 1024 / (s * size)))}"`);
      const d = tag.match(/\bd="([^"]+)"/)[1];
      if (/[^MLZ\d.,\s-]/i.test(d)) return tag;
      const values = d.match(/-?\d+(?:\.\d+)?/g).map(Number);
      const points = Array.from({ length: values.length / 2 }, (_, i) => values.slice(i * 2, i * 2 + 2));
      if (!points.every((p, i) => !i || p[0] === points[i - 1][0] || p[1] === points[i - 1][1])) return tag;
      return tag.replace(/\bd="[^"]+"/, () =>
        `d="${d.replace(/(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)/g, (_, px, py) =>
          `${number(snap(Number(px), tx))},${number(snap(Number(py), ty))}`)}"`);
    });
  }
  const group = (classes, drawing) => `<g class="${classes}" transform="translate(${number(x)} ${number(y)}) scale(${number(scale)})">\n${drawing}\n</g>\n`;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">\n` +
    `<style>\n.icon-small { display: none; }\n` +
    `@media (max-width: 32px) {\n` +
    `  .icon-small path, .icon-small circle { shape-rendering: crispEdges; }\n` +
    `  .icon-normal { display: none; } .icon-32 { display: inline; }\n}\n` +
    `@media (max-width: 28px) { .icon-32 { display: none; } .icon-24 { display: inline; } }\n` +
    `@media (max-width: 20px) { .icon-24 { display: none; } .icon-16 { display: inline; } }\n</style>\n` +
    (background ? `<rect width="1024" height="1024" rx="180" fill="${background}"/>\n` : "") +
    group("icon-normal", body) + [16, 24, 32].map(size => group(`icon-small icon-${size}`, pixelHint(size))).join("") + "</svg>\n";
}

async function png(page, svg, size, path) {
  await page.setViewportSize({ width: size, height: size });
  await page.setContent(`<style>html,body{margin:0;width:100%;height:100%}svg{display:block;width:100%;height:100%}</style>${svg}`);
  return page.screenshot({ path, omitBackground: true });
}

async function lightLogoPngs(page, svg) {
  for (const size of [16, 24, 32, 48, 64, 128, 256, 512, 1024]) {
    await png(page, svg, size, join(root, `brand/rnl-logo-${size}.png`));
  }
}

async function generate(lightPngsOnly = false) {
  if (!lightPngsOnly) execFileSync(rnl, ["check", "brand/rnl-logo.sp"], { cwd: root, stdio: "inherit" });
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ deviceScaleFactor: 1 });
    if (lightPngsOnly) {
      await lightLogoPngs(page, readFileSync(join(root, "brand/rnl-logo-small.svg"), "utf8"));
      console.log("Light logo PNGs regenerated from the committed transparent SVG.");
      return;
    }
    const { svg, bounds } = await renderLogo(page);
    const dark = darkLogo(svg), icon = iconSvg(dark, bounds);
    const light = iconSvg(svg, bounds, { background: null });
    const write = (path, text) => writeFileSync(join(root, path), text);
    write("brand/rnl-logo.svg", svg);
    write("brand/rnl-logo-dark.svg", dark);
    write("brand/rnl-icon.svg", icon);
    write("brand/rnl-logo-small.svg", light);
    write("brand/rnl-logo-small-dark.svg", iconSvg(dark, bounds, { background: null }));
    await lightLogoPngs(page, light);
    for (const size of [16, 24, 32, 1024]) await png(page, icon, size, join(root, `brand/rnl-icon-${size}.png`));

    const assets = join(root, "site/src/assets");
    mkdirSync(join(assets, "brand"), { recursive: true });
    for (const name of ["rnl-logo.svg", "rnl-logo-dark.svg", "rnl-logo-small.svg", "rnl-logo-small-dark.svg"]) {
      copyFileSync(join(root, "brand", name), join(assets, "brand", name));
    }
    writeFileSync(join(assets, "favicon.svg"), icon);
    copyFileSync(join(root, "brand/rnl-icon-32.png"), join(assets, "favicon-32.png"));
    await png(page, icon, 180, join(assets, "apple-touch-icon.png"));
    const viewerIcons = join(root, "apps/viewer-ui/icons");
    mkdirSync(viewerIcons, { recursive: true });
    for (const name of ["favicon.svg", "favicon-32.png", "apple-touch-icon.png"]) {
      copyFileSync(join(assets, name), join(viewerIcons, name));
    }
  } finally {
    await browser.close();
  }

  // The installed Tauri entry point is the same command as `npx tauri icon`, without a download.
  // Generate in a temporary folder and retain only the bundle's configured icons.
  const temporary = mkdtempSync(join(tmpdir(), "rnl-tauri-icons-"));
  try {
    const require = createRequire(join(root, "apps/desktop/package.json"));
    execFileSync(process.execPath, [require.resolve("@tauri-apps/cli/tauri.js"), "icon",
      join(root, "brand/rnl-icon-1024.png"), "-o", temporary], { cwd: root, stdio: "inherit" });
    const config = JSON.parse(readFileSync(join(root, "apps/desktop/tauri.conf.json"), "utf8"));
    for (const icon of config.bundle.icon) copyFileSync(join(temporary, icon.replace(/^icons\//, "")), join(root, "apps/desktop", icon));
    copyFileSync(join(root, "brand/rnl-icon-32.png"), join(root, "apps/desktop/icons/32x32.png"));
    await windowsIco(join(root, "apps/desktop/icons/icon.ico"));
    const blocks = icnsBlocks(join(temporary, "icon.icns"));
    // Let Tauri encode legacy ICNS RGB/mask pairs; retain just the requested physical sizes.
    // Resource types follow tauri-cli/src/helpers/icns.json; masks accompany is32 and il32.
    for (const [size, types] of [[16, ["is32", "s8mk"]], [32, ["il32", "l8mk", "ic11"]]]) {
      const out = join(temporary, `small-${size}`);
      execFileSync(process.execPath, [require.resolve("@tauri-apps/cli/tauri.js"), "icon",
        join(root, `brand/rnl-icon-${size}.png`), "-o", out], { cwd: root, stdio: "pipe" });
      const small = icnsBlocks(join(out, "icon.icns"));
      for (const type of types) {
        if (!small.has(type)) throw new Error(`Tauri did not generate ICNS resource ${type}`);
        blocks.set(type, small.get(type));
      }
    }
    writeIcns(join(root, "apps/desktop/icons/icon.icns"), blocks);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
  console.log("Brand assets regenerated. Review the SVG and small-size icons before committing.");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  if (args.length > 1 || (args.length === 1 && args[0] !== "--light-pngs")) {
    throw new Error("Usage: node scripts/brand.mjs [--light-pngs]");
  }
  await generate(args[0] === "--light-pngs");
}
