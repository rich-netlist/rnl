// Serialize this self-contained function into WebKit/WebView2. Inspect the captured PNG,
// not a re-rendered SVG or the DOM alone. A grey grid/border must not count as schematic ink.
export async function screenshotEvidence(png) {
  const image = new Image();
  image.src = `data:image/png;base64,${png}`;
  await image.decode();
  const canvas = document.createElement("canvas");
  canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
  const context = canvas.getContext("2d", { willReadFrequently:true });
  context.drawImage(image, 0, 0);
  const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
  const sx = canvas.width / innerWidth, sy = canvas.height / innerHeight;
  const inspect = (selector, inset, minimum) => {
    const el = document.querySelector(selector);
    if (!el) return {selector, minimum, ink:0, missing:true};
    const rect = el.getBoundingClientRect(), style = getComputedStyle(el);
    let background = [255,255,255];
    for (let p = el; p; p = p.parentElement) {
      const css = getComputedStyle(p).backgroundColor;
      const rgb = css.match(/[\d.]+/g)?.map(Number);
      if (rgb && (rgb.length === 3 || rgb[3] === 1)) { background = rgb.slice(0,3); break; }
    }
    const area = {x:rect.x+inset[0], y:rect.y+inset[1],
      width:Math.max(0,rect.width-inset[0]-inset[2]), height:Math.max(0,rect.height-inset[1]-inset[3])};
    let ink = 0, sampled = 0;
    for (let y=Math.max(0,Math.ceil(area.y*sy)); y<Math.min(canvas.height,Math.floor((area.y+area.height)*sy)); y++) {
      for (let x=Math.max(0,Math.ceil(area.x*sx)); x<Math.min(canvas.width,Math.floor((area.x+area.width)*sx)); x++) {
        const i=(y*canvas.width+x)*4;
        sampled++;
        if (pixels[i+3] > 200 && Math.max(...background.map((c,k)=>Math.abs(pixels[i+k]-c))) > 100) ink++;
      }
    }
    let visible = rect.width > 0 && rect.height > 0;
    for (let p = el; p; p = p.parentElement) {
      const s = getComputedStyle(p);
      visible &&= s.visibility === 'visible' && s.display !== 'none' && Number(s.opacity) > 0;
    }
    return {selector, area, minimum, ink:ink/(sx*sy), density:sampled ? ink/sampled : 0, visible, background,
      visibility:style.visibility, display:style.display, opacity:style.opacity};
  };
  return {width:canvas.width, height:canvas.height, viewport:{width:innerWidth,height:innerHeight},
    starting:document.body.classList.contains("starting"), startup:!!document.getElementById("startup"),
    loading:!!document.querySelector('#loading:not([hidden])'),
    regions:[inspect('#tab-netlist',[5,5,5,5],30), inspect('#source',[70,5,18,18],150),
      inspect('#canvas',[20,20,20,20],200)]};
}

export function assertPainted(evidence) {
  if (evidence.starting || evidence.startup || evidence.loading) throw new Error("A startup/loading overlay remains.");
  for (const region of evidence.regions) {
    // A missing white layer over a dark parent (or a solid error screen) is not text/ink.
    if (!region.visible || !(region.ink >= region.minimum) || region.density > 0.6) {
      throw new Error(`Screenshot has no painted content in ${region.selector}: ${JSON.stringify(evidence)}`);
    }
  }
}
