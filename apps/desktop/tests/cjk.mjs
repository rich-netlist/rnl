// Runs in the real WebView or browser. Compare rasterized glyphs, rather than accepting
// document.fonts.check (which also succeeds when a font has no Chinese glyphs).
export async function cjkEvidence() {
  const text = "中文网表电路";
  await document.fonts.load('32px "Noto Sans SC"', text);
  await document.fonts.ready;
  const raster = font => {
    const canvas = document.createElement('canvas');
    canvas.width = 256; canvas.height = 64;
    const ctx = canvas.getContext('2d', {willReadFrequently:true});
    ctx.font = font; ctx.fillText(text, 4, 44);
    return ctx.getImageData(0,0,256,64).data;
  };
  const results = [];
  for (const selector of ['#tab-netlist', '#source', 'dialog .dialog-title', '.symbol-detail h2', '.menu-item']) {
    const el = document.querySelector(selector);
    if (!el) continue;
    const s = getComputedStyle(el);
    const prefix = `${s.fontStyle} ${s.fontWeight} 32px `;
    const actual = raster(prefix + s.fontFamily);
    const reference = raster(prefix + '"Noto Sans SC"');
    const missing = raster(prefix + '"RNL nonexistent font", monospace');
    let ink=0, referenceDifference=0, missingDifference=0;
    for (let i=3; i<actual.length; i+=4) {
      if (actual[i]) ink++;
      referenceDifference += Math.abs(actual[i]-reference[i]);
      missingDifference += Math.abs(actual[i]-missing[i]);
    }
    results.push({selector, family:s.fontFamily, ink, referenceDifference, missingDifference});
  }
  return results;
}

export function assertCjk(evidence, noSystemCjk = false) {
  if (evidence.length < 2) throw new Error('No source and UI font evidence.');
  for (const result of evidence) {
    if (result.ink < 500 || (noSystemCjk && (result.referenceDifference !== 0 || result.missingDifference < 1000))) {
      throw new Error(`Chinese glyphs were not painted with the bundled font: ${JSON.stringify(result)}`);
    }
  }
}
