import { expect, test } from '../viewer.js';

test('the files to preload are those below the file shown, breadth first, each once, from the outline', async ({page, viewer}) => {
  await viewer.open('04-hierarchy');
  const files = await page.evaluate(async () => {
    const {preloadFiles} = await import('./preload-queue.js');
    // top.sp calls a cell of a.inc and one of b.inc; a's cell calls one of deep.inc, and top again (a cycle); a
    // device no file defines; b.inc's own other cell is not called
    const level = (file, calls) => ({file, calls: calls.map(level => ({master: 'x', level}))});
    const doc = {file: 'top.sp', outline: {
      files: [{path: 'top.sp', levels: [0]}, {path: 'a.inc', levels: [1]}, {path: 'b.inc', levels: [2, 4]}, {path: 'deep.inc', levels: [3]}],
      levels: [level(0, [1, 2, null]), level(1, [3, 0]), level(2, []), level(3, []), level(2, [])],
    }};
    return {below: preloadFiles(doc), fromA: preloadFiles({...doc, file: 'a.inc'}), none: preloadFiles({file: 'top.sp'})};
  });
  expect(files).toEqual({below: ['a.inc', 'b.inc', 'deep.inc'], fromA: ['deep.inc', 'top.sp', 'b.inc'], none: []});
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
