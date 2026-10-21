import { expect, test } from '../viewer.js';

test('cancelling while Library awaits prevents the stale native preload IPC', async ({page, viewer}) => {
  await viewer.open('04-hierarchy');
  const result = await page.evaluate(async () => {
    let release;
    const pending = new Promise(r=>{release=r}), calls=[];
    const before = window.__TAURI__;
    window.__TAURI__ = {core:{invoke: async cmd=>{calls.push(cmd); return cmd==='update_library'?pending:undefined;}}};
    try {
      const {default:shell} = await import('./shells/desktop.js?preload-race');
      const library = shell.setPool([]);
      const stale = shell.preload({path:'C:/stale.sp'});
      await shell.cancelPreload();
      release([]); await library;
      return {status:await stale, sent:calls.includes('preload_document')};
    } finally {window.__TAURI__=before;}
  });
  expect(result).toEqual({status:'cancelled',sent:false});
});

test('preload order follows descendants breadth first, deduplicates cycles and appends PATH', async ({page, viewer}) => {
  await viewer.open('04-hierarchy');
  const candidates = await page.evaluate(async () => {
    const {preloadSources} = await import('./preload-queue.js');
    const doc = {path:'C:/top.sp', levels:[
      {lines:[0], children:[{level:1},{level:2},{level:1},{level:null}]},
      {lines:[1], children:[{level:3},{level:0}]},
      {lines:[2], children:[]}, {lines:[3], children:[]},
    ]};
    const paths = ['C:/top.sp', 'C:/a.sp', 'C:/b.sp', 'C:/deep.sp'];
    const shell = {platform:'desktop', sourceForFile: (_s,_d,i) => ({path:paths[i]}),
      preloadCandidates: () => ['C:\\A.sp', 'C:/other.sp', 'C:/pkg.rnl'].map(path=>({path}))};
    return preloadSources(doc,0,{path:paths[0]},shell).map(s=>s.path);
  });
  expect(candidates).toEqual(['C:/a.sp','C:/b.sp','C:/deep.sp','C:/other.sp']);
});

test('preloader yields to interaction, serializes builds and discards superseded queues', async ({page, viewer}) => {
  await viewer.open('04-hierarchy');
  const result = await page.evaluate(async () => {
    const {PreloadQueue} = await import('./preload-queue.js');
    let time=0, allowed=true, release, cancelled=0;
    const starts=[];
    const scheduler = new PreloadQueue({now:()=>time, allowed:()=>allowed,
      candidates:()=>['a','b'], cancel:()=>{cancelled++},
      prepare:s=>{starts.push(s); return new Promise(resolve=>{release=resolve});}});
    await scheduler.tick();
    time=1500; allowed=false; await scheduler.tick();
    allowed=true; scheduler.activity(); await scheduler.tick();
    const idle=starts.length;
    time=3000; const pending=scheduler.tick(); await scheduler.tick();
    time=4000; release('ready'); await pending;
    time=6999; await scheduler.tick();
    const rest=starts.length;
    time=7000; const second=scheduler.tick();
    scheduler.reset(); release('full'); await second;
    // A stale 'full' response must not empty the newly planned queue.
    time=9000; const third=scheduler.tick(); release('ready'); await third;
    return {idle, rest, starts, cancelled};
  });
  expect(result).toEqual({idle:0,rest:1,starts:['a','b','a'],cancelled:1});
});

test('preload errors do not stop later candidates; full cache stops speculation', async ({page, viewer}) => {
  await viewer.open('04-hierarchy');
  const result = await page.evaluate(async () => {
    const {PreloadQueue} = await import('./preload-queue.js');
    let time=2000, n=0;
    const starts=[];
    const q = new PreloadQueue({now:()=>time, allowed:()=>true, cancel:()=>{},
      candidates:()=>['bad','low-memory','last','extra'], prepare:async s=>{
        starts.push(s);
        if(s==='bad') throw Error('missing');
        if(s==='low-memory' && n++===0) return 'paused';
        return s==='last'?'full':'ready';
      }});
    for(let i=0;i<8;i++) {time+=2000; await q.tick();}
    return starts;
  });
  expect(result).toEqual(['bad','low-memory','low-memory','last']);
});
