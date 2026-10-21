import { append, m, setAttr } from "./i18n.js";
// An independent viewport for SVG produced by the RNL renderer. Like the main viewer, pan and
// zoom change drawing coordinates, never CSS scale; each embedded view owns its input and lifetime.
import { svgElement } from './util.js';

export function svgViewer(container, svg, scale) {
  const extent = svg.getAttribute('viewBox').split(/[\s,]+/).map(Number);
  let view, size = {w: 0, h: 0}, press;
  container.tabIndex = 0;
  setAttr(container, 'aria-label', m('Symbol viewer'));
  svg.setAttribute('preserveAspectRatio', 'xMinYMin meet');
  const paint = () => {
    if (!view || !size.w || !size.h) return;
    svg.setAttribute('viewBox', `${view.x} ${view.y} ${size.w/view.k} ${size.h/view.k}`);
    let step = scale * view.k;
    while (step < 14) step *= 2;
    while (step > 56) step /= 2;
    container.style.backgroundSize = `${step}px ${step}px`;
    container.style.backgroundPosition = `${-view.x*view.k}px ${-view.y*view.k}px`;
  };
  const fit = () => {
    if (!size.w || !size.h) return;
    const [x,y,w,h] = extent;
    const k = Math.min((size.w-24)/Math.max(1,w), (size.h-24)/Math.max(1,h));
    view = {x: x+w/2-size.w/(2*k), y: y+h/2-size.h/(2*k), k};
    paint();
  };
  const zoom = (factor, x = size.w/2, y = size.h/2) => {
    if (!view) return;
    const k = Math.min(50, Math.max(0.02, view.k*factor));
    view = {x: view.x+x/view.k-x/k, y: view.y+y/view.k-y/k, k};
    paint();
  };
  const observer = new ResizeObserver(() => {
    const w = container.clientWidth, h = container.clientHeight;
    if (!w || !h) return;
    if (view) { view.x += (size.w-w)/(2*view.k); view.y += (size.h-h)/(2*view.k); }
    size = {w,h};
    if (view) paint(); else fit();
  });
  observer.observe(container);
  const controller = new AbortController(), signal = controller.signal;
  const toolbar = document.createElement('div');
  toolbar.className = 'symbol-viewer-tools';
  toolbar.setAttribute('role', 'group'); setAttr(toolbar, 'aria-label', m('Symbol view controls'));
  for (const [name, title, d, action] of [
    ['fit', m('Reset view'), 'M6 2H2v4m8-4h4v4M2 10v4h4m8-4v4h-4', fit],
    ['zoom-in', m('Zoom in'), 'M3 8h10M8 3v10', () => zoom(1.25)],
    ['zoom-out', m('Zoom out'), 'M3 8h10', () => zoom(0.8)],
  ]) {
    const button = document.createElement('button');
    button.type = 'button'; button.dataset.action = name; setAttr(button, "title", title);
    setAttr(button, 'aria-label', title);
    const icon = svgElement('svg', {viewBox: '0 0 16 16', 'aria-hidden': 'true'});
    append(icon, svgElement('path', {d, fill:'none', stroke:'currentColor', 'stroke-width':1.5}));
    append(button, icon); button.addEventListener('click', action, {signal}); append(toolbar, button);
  }
  append(container, svg, toolbar);
  container.addEventListener('wheel', e => {
    e.preventDefault(); e.stopPropagation();
    const rect = container.getBoundingClientRect();
    const delta = e.deltaY * (e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? size.h : 1);
    zoom(Math.exp(-Math.max(-600,Math.min(600,delta))*0.002), e.clientX-rect.left, e.clientY-rect.top);
  }, {signal, passive:false});
  container.addEventListener('pointerdown', e => {
    e.stopPropagation();
    if (e.target.closest('button') || e.button !== 0 || !view) return;
    e.preventDefault(); container.focus({preventScroll:true});
    press = {x:e.clientX, y:e.clientY, view:{...view}};
    container.setPointerCapture(e.pointerId); container.classList.add('panning');
  }, {signal});
  container.addEventListener('pointermove', e => {
    if (!press) return;
    view = {...press.view, x:press.view.x-(e.clientX-press.x)/press.view.k,
      y:press.view.y-(e.clientY-press.y)/press.view.k}; paint();
  }, {signal});
  const end = () => { press = null; container.classList.remove('panning'); };
  for (const type of ['pointerup','pointercancel','lostpointercapture','blur']) container.addEventListener(type,end,{signal});
  container.addEventListener('dblclick', e => { e.stopPropagation(); if (!e.target.closest('button')) fit(); }, {signal});
  container.addEventListener('keydown', e => {
    if (e.ctrlKey || e.metaKey || e.altKey || e.target.closest('button')) return;
    if (['0','f','F'].includes(e.key)) { e.preventDefault(); e.stopPropagation(); fit(); }
    else if (['+','=','-','_'].includes(e.key)) {
      e.preventDefault(); e.stopPropagation(); zoom(['-','_'].includes(e.key) ? 0.8 : 1.25);
    }
  }, {signal});
  return () => { observer.disconnect(); controller.abort(); end(); };
}
