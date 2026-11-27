import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const [port, evidence, mode, pid, tracePath] = process.argv.slice(2);
const priorReleaseBySeed = new Map([
  ['upgrade-seed','0.1.0'], ['upgrade-seed-r5','0.2.1'], ['upgrade-seed-r5-terrain','0.2.2'],
  ['upgrade-seed-r5-living','0.2.3'], ['upgrade-seed-r5-gravel','0.2.4'],
  ['upgrade-seed-r5-sand','0.2.5'], ['upgrade-seed-r5-feedback','0.2.6'],
  ['upgrade-seed-r5-topology','0.2.7'], ['upgrade-seed-r5-sources','0.2.8'], ['upgrade-seed-r5-stone-plant','0.2.9'],
  ['upgrade-seed-r5-ribbon','0.2.10'], ['upgrade-seed-r5-floating','0.2.11'],
  ['upgrade-seed-r5-stone-biofilm','0.2.12'],
  ['upgrade-seed-live-topology','0.2.13'],
  ['upgrade-seed-static-sections','0.2.14'],
  ['upgrade-seed-leaf-sections','0.2.15'],
  ['upgrade-seed-plant-sections','0.2.16'],
  ['upgrade-seed-living-sections','0.2.17'],
  ['upgrade-seed-structures-litter','0.2.18'],
  ['upgrade-seed-botany','0.2.19'],
]);
const seedRelease = priorReleaseBySeed.get(mode);
assert(['legacy','r5','r5-reopen','upgrade'].includes(mode) || seedRelease);
assert(Number(port) > 0 && Number(port) <= 65535 && Number(pid) > 0);
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function until(check, message) {
  let lastError;
  for (let attempt = 0; attempt < 160; attempt += 1) {
    try {
      const result = await check();
      if (result) return result;
    } catch (error) { lastError = error; }
    await delay(60);
  }
  throw new Error(message, { cause:lastError });
}

async function connect(page) {
  const url = new URL(page.webSocketDebuggerUrl);
  assert.equal(url.hostname,'127.0.0.1');
  assert.equal(url.port,port);
  const socket = new WebSocket(url);
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Installed WebView connection timed out')),5000);
    socket.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
    socket.addEventListener('error', (error) => { clearTimeout(timer); reject(error); }, { once: true });
  });
  let sequence = 0;
  function request(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++sequence;
      const timer = setTimeout(() => {
        socket.removeEventListener('message', onMessage);
        reject(new Error(`${method} timed out`));
      }, 5_000);
      function onMessage({ data }) {
        const response = JSON.parse(data);
        if (response.id !== id) return;
        socket.removeEventListener('message', onMessage);
        clearTimeout(timer);
        const error = response.error || response.result?.exceptionDetails;
        if (error) reject(new Error(JSON.stringify(error)));
        else resolve(response.result);
      }
      socket.addEventListener('message', onMessage);
      socket.send(JSON.stringify({ id, method, params }));
    });
  }
  const evaluate = async (expression) => (await request('Runtime.evaluate', {
    expression, awaitPromise: true, returnByValue: true,
  })).result.value;
  return { request, evaluate, close: () => socket.close() };
}

const pages = await until(async () => {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/json/list`, { signal:AbortSignal.timeout(1200) });
    const found = (await response.json()).filter((page) => page.type === 'page');
    return found.length >= 4 && found;
  } catch {
    return false;
  }
}, 'Installed application did not initialize all four WebViews');

const open = [];
const views = new Map();
const forestCanvas = 'canvas[data-forest-composed=v2][data-ready=true]';
const pondCanvas = 'canvas[data-pond-composed=v2][data-ready=true]';
const legacyForestCanvas = '.forest-view canvas';
const legacyPondCanvas = '.pond-scene canvas:not([data-pond-composed])';
const editor = (expression) => views.get('editor').evaluate(expression);
const ipc = (label, command, args = {}) => views.get(label).evaluate(
  `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`,
);

function record(caseName, result) { console.log(JSON.stringify({ case:caseName, ...result })); }

async function snapshot(label = 'observe') {
  const subscription = await ipc(label,'subscribe_world',{viewId:label});
  try { return subscription.snapshot; }
  finally { await ipc(label,'unsubscribe_world',{viewId:label,subscriptionToken:subscription.subscriptionToken}); }
}

async function waitWorld(id, version) {
  return until(async () => {
    const state=await snapshot();
    return state.worldId===id && state.modelVersion===version && state;
  }, `Installed app did not open expected model v${version}`);
}

async function verifyOwnership() {
  const current=await until(()=>snapshot('panel'),'Installed panel did not supply its own authoritative state');
  const seeded = await until(async()=>{
    const lines=(await readFile(tracePath,'utf8')).trim().split(/\r?\n/).filter(Boolean).map((line)=>JSON.parse(line));
    const boot=lines.find(({event,processId})=>event==='runtime-started'&&processId===Number(pid));
    const binding=lines.find(({event,processId,runGeneration})=>event==='view-subscribed'&&processId===Number(pid)&&runGeneration===current.runGeneration);
    return boot && binding && boot;
  },'Installed WebView did not match the trace and owned executable process');
  record('owned-installed-process',{pid:Number(pid),bootGeneration:seeded.runGeneration});
}

async function showViews() {
  await ipc('panel','show_window',{viewId:'observe'});
  await ipc('panel','show_window',{viewId:'editor'});
  await until(()=>editor("document.querySelector('main.world-editor')?.dataset.worldId"),'Installed editor did not show its world');
}

async function revealPanel() {
  const current = await ipc('layout','get_workspace_layout');
  if (current?.idle?.mode==='off') return;
  const idle={mode:'off',text:''};
  const layout=current ?? {version:2,mode:'combined',items:[{id:'forest',kind:'ecology',x:772,y:460}],idle};
  const saved=await ipc('layout','save_workspace_idle',{idle,layout,panelWorldId:''});
  assert.deepEqual(saved.idle,idle);
}

async function canvas(label, selector) {
  return until(()=>views.get(label).evaluate(`(() => {
    const frame=document.querySelector(${JSON.stringify(selector)});
    const cover=document.querySelector('.landscape__cover');
    if(!frame?.width || !frame?.height || cover && Number(getComputedStyle(cover).opacity)>.01) return null;
    return {width:frame.width,height:frame.height,dpr:devicePixelRatio,scene:{...frame.dataset}};
  })()`),`Installed ${label} did not render ${selector}`);
}

async function png(label, file) {
  const result=await views.get(label).request('Page.captureScreenshot',{format:'png',fromSurface:true});
  await writeFile(join(evidence,file),Buffer.from(result.data,'base64'));
}

async function backing(label, selector, file) {
  const data=await views.get(label).evaluate(`document.querySelector(${JSON.stringify(selector)}).toDataURL('image/png')`);
  const bytes=Buffer.from(data.split(',')[1],'base64');
  await writeFile(join(evidence,file),bytes);
  return [bytes.readUInt32BE(16),bytes.readUInt32BE(20)];
}

async function clickButton(label, scope='button') {
  await delay(120);
  const view=views.get('editor');
  const point=await until(()=>view.evaluate(`(() => {
    const nodes=[...document.querySelectorAll(${JSON.stringify(scope)})].filter((node)=>
      !node.disabled&&node.getClientRects().length&&(node.getAttribute('aria-label')||node.textContent).trim()===${JSON.stringify(label)});
    if(nodes.length!==1)return null;
    const target=nodes[0];target.scrollIntoView({block:'nearest',inline:'nearest'});
    const rect=target.getBoundingClientRect(); const point={x:rect.x+rect.width/2,y:rect.y+rect.height/2};
    return target.contains(document.elementFromPoint(point.x,point.y))&&point;
  })()`), `Installed editor control did not become available: ${label}`);
  await view.request('Input.dispatchMouseEvent',{type:'mouseMoved',...point,button:'none',buttons:0});
  await view.request('Input.dispatchMouseEvent',{type:'mousePressed',...point,button:'left',buttons:1,clickCount:1});
  await view.request('Input.dispatchMouseEvent',{type:'mouseReleased',...point,button:'left',buttons:0,clickCount:1});
}

async function createWorld(preset, name, version) {
  await clickButton('世界库');
  await clickButton('从预设新建','dialog[open] button');
  await until(()=>editor(`(() => {
    const form=document.querySelector('dialog[open]');
    const select=form?.querySelector('select');
    if(!select?.querySelector('option[value=${preset}]'))return false;
    select.value=${JSON.stringify(preset)};select.dispatchEvent(new Event('change',{bubbles:true}));
    const field=form.querySelector('input[required]');field.focus();return document.activeElement===field;
  })()`),'Installed world library did not display its expected preset');
  await views.get('editor').request('Input.insertText',{text:name});
  await clickButton('新建并打开','dialog[open] button');
  return until(async()=>{
    const world=await snapshot();
    return world.worldName===name&&world.modelVersion===version&&world;
  },`Installed world library did not create ${preset}`);
}

function worldState(state) {
  const keys=['worldId','worldName','modelVersion','definition','simulationTick','editRevision',
    'paused','editPaused','manualPaused','daylightPhase','rock','animals','forest','forestRocks','pond','season','location'];
  const saved = Object.fromEntries(keys.map((key)=>[key,state[key]??null]));
  if (saved.forestRocks) {
    // Undo and redo are runtime-session state, including for paused worlds.
    const {undoAvailable,redoAvailable,...forestRocks} = saved.forestRocks;
    saved.forestRocks = forestRocks;
  }
  return saved;
}

function canonicalDigest(state) {
  const body = JSON.stringify(state, (_, value) => {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return value;
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, value[key]]));
  });
  return createHash('sha256').update(body).digest('hex');
}

function validateDefaultSubstrate(projected, prior) {
  assert(!('substrateCells' in prior.forestRocks) && !('substratePresets' in prior.forestRocks));
  const { substrateCells, substratePresets } = projected.forestRocks;
  assert.deepEqual(substratePresets, [
    {substrate:'native',waterCapacityMl:30,evaporationMultiplier:1},
    {substrate:'fineSoil',waterCapacityMl:36,evaporationMultiplier:0.85},
    {substrate:'coarseHumus',waterCapacityMl:24,evaporationMultiplier:1.15},
  ]);
  assert.equal(substrateCells.length,48);
  const waterBySide = [0, 0];
  const capacityBySide = [0, 0];
  for (const [index, cell] of substrateCells.entries()) {
    const column = index % 12;
    const row = Math.floor(index / 12);
    const side = column < 6 ? 0 : 1;
    assert.equal(cell.cellIndex,index);
    assert(Math.abs(cell.position.x - (0.04 + (column + 0.5) * (0.96 - 0.04) / 12)) < 1e-12);
    assert(Math.abs(cell.position.y - (0.58 + (row + 0.5) * (0.94 - 0.58) / 4)) < 1e-12);
    assert.deepEqual({substrate:cell.substrate,waterCapacityMl:cell.waterCapacityMl,
      evaporationMultiplier:cell.evaporationMultiplier},substratePresets[0]);
    assert(Number.isFinite(cell.waterMl) && cell.waterMl >= 0 && cell.waterMl <= cell.waterCapacityMl);
    waterBySide[side] += cell.waterMl;
    capacityBySide[side] += cell.waterCapacityMl;
  }
  assert.deepEqual(waterBySide,prior.forest.waterMl);
  assert.deepEqual(capacityBySide,prior.forest.waterCapacityMl);
  delete projected.forestRocks.substrateCells;
  delete projected.forestRocks.substratePresets;
  return ['forestRocks.substrateCells', 'forestRocks.substratePresets'];
}

function validateDefaultPondShape(projected, prior) {
  assert.equal(prior.pond.basins.length,1);
  assert.equal(projected.pond.basins.length,1);
  const before = prior.pond.basins[0];
  const basin = projected.pond.basins[0];
  assert.equal(basin.id,before.id);
  assert(!('shape' in before));
  assert.deepEqual(basin.shape, {
    columns:12, rows:12, cells:Array.from({length:24}, (_, index) => 60 + index),
    protectedRow:6, cellAreaCm2:before.surfaceAreaCm2 / 24,
  });
  delete basin.shape;
  return ['pond.basins[0].shape'];
}

function stripDerivedUpgradeFields(actual, before, priorRelease) {
  if (priorRelease === '0.2.7' && before.modelVersion === 13) {
    return actual.pond.basins.map(basin => {
      const saved = before.pond.basins.find(({id}) => id === basin.id);
      assert(saved && !('biofilmDistribution' in saved));
      const { bedMgC, inactiveMgC, surfaces } = basin.biofilmDistribution;
      const active = new Set((basin.stones ?? []).filter(({preset}) => preset === 'main').map(({id}) => `stone:${id}`));
      for (const { id } of basin.woods ?? []) active.add(`wood:${id}`);
      assert(Number.isFinite(bedMgC) && bedMgC >= 0 && Number.isFinite(inactiveMgC) && inactiveMgC >= 0);
      for (const {kind, id, mgC} of surfaces) {
        assert(active.delete(`${kind}:${id}`));
        assert(Number.isFinite(mgC) && mgC > 0);
      }
      if (basin.wet) assert.equal(inactiveMgC, 0);
      else assert.deepEqual({bedMgC, inactiveMgC, surfaces}, {bedMgC:0, inactiveMgC:saved.biofilmMgC, surfaces:[]});
      assert(Math.abs(bedMgC + inactiveMgC + surfaces.reduce((sum, {mgC}) => sum + mgC, 0) - saved.biofilmMgC) < 1e-8);
      delete basin.biofilmDistribution;
      return `pond.basins[${basin.id}].biofilmDistribution`;
    });
  }
  if (priorRelease === '0.2.2' && before.modelVersion === 14) {
    assert(!('fernPatches' in before.forestRocks) && !('fernPreset' in before.forestRocks));
    assert.deepEqual(actual.forestRocks.fernPatches, []);
    assert.deepEqual(actual.forestRocks.fernPreset, {initialMgCnp:[15,0.54,0.075],heightMm:140,widthMm:130});
    delete actual.forestRocks.fernPatches;
    delete actual.forestRocks.fernPreset;
    return ['forestRocks.fernPatches', 'forestRocks.fernPreset'];
  }
  if (priorRelease === '0.2.1') {
    if (before.modelVersion === 14) return validateDefaultSubstrate(actual, before);
    assert.equal(before.modelVersion,13);
    return validateDefaultPondShape(actual, before);
  }
  if (priorRelease === '0.1.0' && before.modelVersion === 3) {
    const projections = [];
    for (const basin of actual.pond.basins) {
      const saved = before.pond.basins.find(({ id }) => id === basin.id);
      if (!saved || 'shrimpLife' in saved || !('shrimpLife' in basin)) continue;
      assert.deepEqual(basin.shrimpLife.map(({ id }) => id), saved.shrimp.map(({ id }) => id));
      assert.ok(basin.shrimpLife.every(({ ageSeconds, stage }) =>
        Number.isFinite(ageSeconds) && ageSeconds >= 0 && ['adult', 'juvenile'].includes(stage)));
      delete basin.shrimpLife;
      projections.push(`pond.basins[${basin.id}].shrimpLife`);
    }
    return projections;
  }
  return [];
}

async function compareSaved(world, priorRelease) {
  const library=await ipc('editor','list_worlds');
  if(library.activeWorldId!==world.state.worldId) await ipc('editor','switch_world',{worldId:world.state.worldId});
  const current=await waitWorld(world.state.worldId,world.state.modelVersion);
  if (current.forestRocks) {
    assert.equal(current.forestRocks.undoAvailable,false);
    assert.equal(current.forestRocks.redoAvailable,false);
  }
  if (current.pond) {
    assert.equal(current.pondUndoAvailable,false);
    assert.equal(current.pondRedoAvailable,false);
  }
  const actual = structuredClone(worldState(current));
  const verifiedDerivedFields = stripDerivedUpgradeFields(actual, world.state, priorRelease);
  const substrate = actual.forestRocks?.substrateCells;
  const beforeSubstrate = world.state.forestRocks?.substrateCells;
  const verifyDecomposers = priorRelease && substrate?.some(cell=>Object.hasOwn(cell,'decomposerMgC')) &&
    beforeSubstrate?.every(cell=>!Object.hasOwn(cell,'decomposerMgC'));
  if (priorRelease && (actual.modelVersion === 13 || verifyDecomposers)) {
    const folder=join(process.env.MICRO_ECOSYSTEM_DATA_DIR,`world-${world.state.worldId}`);
    const names=(await readdir(folder)).filter(name=>/^checkpoint-\d+\.json$/.test(name)).sort();
    const wrapper=JSON.parse(await readFile(join(folder,names.at(-1)),'utf8'));
    const saved=JSON.parse(wrapper.payload);
    assert.equal(saved.worldId,world.state.worldId);
    assert.equal(String(saved.state.tick),world.state.simulationTick);
    assert.equal(String(saved.editRevision),world.state.editRevision);
    for (const basin of actual.pond?.basins ?? []) {
      const before=world.state.pond.basins.find(({id})=>id===basin.id);
      const stored=saved.state.pond.basins.find(({id})=>id===basin.id);
      if ('daphniaStockMgCnp' in basin && !('daphniaStockMgCnp' in before)) {
        assert.equal(stored.daphnia.count,basin.daphniaCount);
        assert.deepEqual(basin.daphniaStockMgCnp,stored.daphnia.body.map((mg,element)=>mg+stored.daphnia.reserve[element]));
        delete basin.daphniaStockMgCnp;
        verifiedDerivedFields.push(`pond.basins[${basin.id}].daphniaStockMgCnp`);
      }
      if (basin.retainObservationRow && !('retainObservationRow' in before)) {
        assert.equal(stored.shrimp.length,0);
        assert(stored.nextShrimpId>1);
        delete basin.retainObservationRow;
        verifiedDerivedFields.push(`pond.basins[${basin.id}].retainObservationRow`);
      }
    }
    if(verifyDecomposers) {
      assert.equal(substrate.length,saved.state.forest.soil.length);
      for (const cell of substrate) {
        assert.equal(cell.decomposerMgC,saved.state.forest.soil[cell.cellIndex].microbes.mgC);
        delete cell.decomposerMgC;
      }
      verifiedDerivedFields.push('forestRocks.substrateCells[].decomposerMgC');
    }
  }
  assert.deepEqual(actual,world.state);
  assert.notEqual(current.runGeneration,world.runGeneration);
  const stateSha256 = canonicalDigest(actual);
  assert.equal(stateSha256,world.stateSha256 ?? canonicalDigest(world.state));
  return { current, stateSha256, verifiedDerivedFields };
}

function editContext(world, commandId) {
  return {worldId:world.worldId,commandId,expectedGeneration:world.runGeneration,expectedRevision:world.editRevision};
}

async function seedEditedForest(world) {
  assert(world.paused);
  const cells = [11,23];
  const before = world.forestRocks.substrateCells;
  assert(cells.every((cell)=>before[cell].substrate==='native'));
  const changed = await ipc('editor','edit_forest_rocks',{
    ...editContext(world,'upgrade-f07-fine'), action:{action:'change',edit:{kind:'setSubstrate',cells,substrate:'fineSoil'}},
  });
  assert.equal(BigInt(changed.editRevision),BigInt(world.editRevision)+1n);
  assert.equal(changed.forestRocks.undoAvailable,true);
  assert.deepEqual(changed.forestRocks.substrateCells,before.map((cell)=>cells.includes(cell.cellIndex) ?
    {...cell,substrate:'fineSoil',waterCapacityMl:36,evaporationMultiplier:0.85} : cell));
  assert.deepEqual(changed.forest.waterMl,world.forest.waterMl);
  record('prior-r5-f07-stock-preserved',{cells,substrate:'fineSoil',editRevision:changed.editRevision,waterMl:changed.forest.waterMl});
  return changed;
}

async function seedEditedPond(world) {
  assert(world.paused);
  const cells = [41,53];
  const before = world.pond.basins[0];
  assert(cells.every((cell)=>!before.shape.cells.includes(cell)));
  const changed = await ipc('editor','edit_pond',{
    ...editContext(world,'upgrade-pond-footprint'), edit:{type:'paintShape',basinId:before.id,mode:'add',cells},
  });
  assert.equal(BigInt(changed.editRevision),BigInt(world.editRevision)+1n);
  assert.equal(changed.pondUndoAvailable,true);
  const basin = changed.pond.basins[0];
  assert.deepEqual(basin.shape,{...before.shape,cells:[...before.shape.cells,...cells].sort((a,b)=>a-b)});
  assert.equal(basin.surfaceAreaCm2,before.surfaceAreaCm2+cells.length*before.shape.cellAreaCm2);
  assert.equal(basin.waterLiters,before.waterLiters);
  assert.equal(basin.oxygenMgPerLiter,before.oxygenMgPerLiter);
  assert(basin.depthCm<before.depthCm);
  record('prior-r5-pond-volume-preserved',{cells,editRevision:changed.editRevision,areaCm2:basin.surfaceAreaCm2,
    waterLiters:basin.waterLiters,depthCm:basin.depthCm});
  return changed;
}

async function seedLivingFern(world) {
  assert(world.paused && world.forestRocks.fernPatches.length===0);
  const position={x:0.45,y:0.85};
  const planted=await ipc('editor','edit_forest_rocks',{
    ...editContext(world,'upgrade-f10-plant'), action:{action:'change',edit:{kind:'addFernPatch',position}},
  });
  assert.equal(BigInt(planted.editRevision),BigInt(world.editRevision)+1n);
  assert.equal(planted.forestRocks.fernPatches.length,1);
  const fern=planted.forestRocks.fernPatches[0];
  assert.deepEqual(fern.position,position);
  assert(Math.abs(fern.stockMgCnp[0]-15)<1e-9);
  const trimmed=await ipc('editor','edit_forest_rocks',{
    ...editContext(planted,'upgrade-f10-trim'), action:{action:'change',edit:{kind:'trimFernPatch',id:fern.id,mgC:12}},
  });
  assert.equal(BigInt(trimmed.editRevision),BigInt(planted.editRevision)+1n);
  assert.equal(trimmed.forestRocks.undoAvailable,true);
  const remaining=trimmed.forestRocks.fernPatches[0];
  assert.equal(remaining.id,fern.id);
  assert.deepEqual(remaining.position,position);
  assert(Math.abs(remaining.stockMgCnp[0]-3)<1e-9);
  assert(Math.abs(trimmed.forest.litterMgC-planted.forest.litterMgC-12)<1e-9);
  record('prior-r5-f10-live-fern',{id:fern.id,position,cellIndex:remaining.cellIndex,
    remainingMgC:remaining.stockMgCnp[0],editRevision:trimmed.editRevision});
  return trimmed;
}

async function seedLivingDualPond(world) {
  assert(world.paused);
  const [first,second]=world.pond.basins;
  assert.deepEqual(world.pond.basins.map(({id})=>id),['pond-main','pond-2']);
  assert.deepEqual(world.pond.basins.map(({waterLiters})=>waterLiters),[18,18]);
  assert(!first.leafDeposits && !second.leafDeposits);
  const editSecond=(snapshot,commandId,edit)=>ipc('editor','edit_pond',{
    ...editContext(snapshot,commandId),edit:{...edit,basinId:second.id},
  });
  const watered=await editSecond(world,'upgrade-dual-second-water',{type:'addWater',liters:0.8});
  assert.deepEqual(watered.pond.basins.map(({waterLiters})=>waterLiters),[18,18.8]);
  const placed=await editSecond(watered,'upgrade-p05-add',{type:'addLeafDeposit',x:0.47,preset:'foreground',mgC:50});
  const plantedBasin=placed.pond.basins[1];
  assert.equal(plantedBasin.leafDeposits.length,1);
  const leaf=plantedBasin.leafDeposits[0];
  assert.equal(leaf.initialMgC,50);
  assert.equal(leaf.remainingMgC,50);
  assert.equal(leaf.stage,'whole');
  assert.equal(plantedBasin.detritusMgC,watered.pond.basins[1].detritusMgC+50);
  const moved=await editSecond(placed,'upgrade-p05-move',{type:'moveLeafDeposit',id:leaf.id,x:0.76});
  assert.equal(BigInt(moved.editRevision),BigInt(world.editRevision)+3n);
  assert.equal(moved.pondUndoAvailable,true);
  assert.deepEqual(moved.pond.basins[0],first);
  assert.deepEqual(moved.pond.basins[1],{...plantedBasin,leafDeposits:[{...leaf,x:0.76}]});
  record('prior-r5-p05-and-dual',{basinIds:[first.id,second.id],waterLiters:[18,18.8],
    leafId:leaf.id,leafX:0.76,remainingMgC:leaf.remainingMgC,editRevision:moved.editRevision});
  return moved;
}

const gravelStyle = 'bottom-gravel-patch';
const sandStyles = ['bottom-fine-sand-patch', 'bottom-coarse-sand-patch'];

async function seedGravelPatch(world) {
  const [first,second]=world.pond.basins;
  assert(!second.scenery.some(({style})=>style===gravelStyle));
  const placed=await ipc('editor','edit_pond',{
    ...editContext(world,'upgrade-p04-add'),
    edit:{type:'addScenery',basinId:second.id,style:gravelStyle,x:0.42,y:0.86},
  });
  const patches=placed.pond.basins[1].scenery.filter(({style})=>style===gravelStyle);
  assert.equal(patches.length,1);
  const [patch]=patches;
  assert.deepEqual(patch,{id:patch.id,style:gravelStyle,x:0.42,y:0.86});
  assert.deepEqual(placed.pond.basins,[first,{...second,scenery:[...second.scenery,patch]}]);

  const position={x:0.53,y:0.90};
  const moved=await ipc('editor','edit_pond',{
    ...editContext(placed,'upgrade-p04-move'),
    edit:{type:'moveScenery',basinId:second.id,id:patch.id,...position},
  });
  assert.equal(BigInt(moved.editRevision),BigInt(world.editRevision)+2n);
  assert.equal(moved.pondUndoAvailable,true);
  assert.deepEqual(moved.pond.basins,[first,{...second,scenery:[...second.scenery,{...patch,...position}]}]);
  record('prior-r5-p04-gravel',{basinId:second.id,gravelId:patch.id,position,editRevision:moved.editRevision});
  return moved;
}

async function seedSandPatches(world) {
  const [first, second] = world.pond.basins;
  const positions = [{x:0.39,y:0.86}, {x:0.63,y:0.91}];
  const placed = [];
  for (const [index, style] of sandStyles.entries()) {
    const position = positions[index];
    world = await ipc('editor','edit_pond',{
      ...editContext(world, `upgrade-sand-${index}`),
      edit:{type:'addScenery', basinId:second.id, style, ...position},
    });
    const patch = world.pond.basins[1].scenery.find(item => item.style===style);
    assert.deepEqual(patch, {id:patch.id, style, ...position});
    placed.push(patch);
  }
  assert.deepEqual(world.pond.basins, [first, {...second, scenery:[...second.scenery,...placed]}]);
  record('prior-r5-p04-sand',{basinId:second.id,placed});
  return world;
}

async function seedFloatingPlant(world) {
  const [first,second]=world.pond.basins;
  const {columns,rows,protectedRow,cells}=first.shape;
  const column=Math.floor(columns/3);
  assert(first.wet && cells.includes(protectedRow*columns+column));
  assert.equal(first.floatingPlants,undefined);
  const position={x:(column+0.5)/columns,y:(protectedRow+0.5)/rows};
  const planted=await ipc('editor','edit_pond',{
    ...editContext(world,'upgrade-p11-floating'),
    edit:{type:'addFloatingPlant',basinId:first.id,position,preset:'large'},
  });
  assert.equal(BigInt(planted.editRevision),BigInt(world.editRevision)+1n);
  const [updated,unchanged]=planted.pond.basins;
  assert.deepEqual(unchanged,second);
  assert.equal(updated.floatingPlants.length,1);
  const [plant]=updated.floatingPlants;
  assert.deepEqual(plant.position,position);
  assert.equal(plant.preset,'large');
  assert.equal(plant.floating,true);
  assert.equal(plant.diameterMm,60);
  assert.deepEqual(plant.stockMgCnp,[32,1.92,0.224]);
  const canopy=updated.floatingCanopy;
  assert(canopy.coverage>0 && canopy.coverage<1);
  assert.equal(canopy.lightTransmission,1-canopy.coverage);
  record('prior-r5-floating-plant',{basinId:first.id,plant,canopy});
  return planted;
}

async function seedTopologyWorld(world, staticLayout, living = false) {
  const [first,second] = world.pond.basins;
  async function edit(name, change) {
    world = await ipc('editor','edit_pond',{...editContext(world,name), edit:change});
  }
  await edit('upgrade-topology-water-a',{type:'addWater',basinId:first.id,liters:5});
  await edit('upgrade-topology-water-b',{type:'addWater',basinId:second.id,liters:7});
  await edit('upgrade-topology-litter',{type:'addDetritus',basinId:second.id,mgC:80});
  if (staticLayout) {
    await edit('upgrade-topology-place-stone',{type:'addStone',basinId:first.id,x:0.65,preset:'main'});
    await edit('upgrade-topology-place-wood',{type:'addWood',basinId:first.id,x:0.28,preset:'root'});
    await edit('upgrade-topology-place-sand',{type:'addScenery',basinId:first.id,style:'bottom-coarse-sand-patch',x:0.56,y:0.86});
  }
  if (living) {
    const position=(basin,x)=>({x,y:(basin.shape.protectedRow+.5)/basin.shape.rows});
    await edit('upgrade-topology-live-shrimp',{type:'stockShrimp',basinId:first.id,position:{x:0.86,y:0.4},count:1});
    await edit('upgrade-topology-first-daphnia',{type:'stockDaphnia',basinId:first.id,count:5});
    await edit('upgrade-topology-second-daphnia',{type:'stockDaphnia',basinId:second.id,count:2});
    await edit('upgrade-topology-first-float',{type:'addFloatingPlant',basinId:first.id,position:position(first,0.35),preset:'medium'});
    await edit('upgrade-topology-second-float',{type:'addFloatingPlant',basinId:second.id,position:position(second,0.72),preset:'large'});
  }
  const retainedLayout=world.pond.basins[0];
  await edit('upgrade-topology-merge',{type:'commitTopology',basinId:first.id,mode:'add',cells:[29]});
  assert.equal(world.pond.basins.length,1);
  await edit('upgrade-topology-split',{type:'commitTopology',basinId:first.id,mode:'remove',cells:[29]});
  const basins = world.pond.basins.map(({id,waterLiters,detritusMgC,shape}) => [id,waterLiters,detritusMgC,shape.protectedRow]);
  assert.deepEqual(basins,[['pond-main',6,40,1],['pond-3',6,40,3]]);
  const layout=({stones,woods,scenery})=>({stones,woods,scenery});
  assert.deepEqual(layout(world.pond.basins[0]),layout(retainedLayout));
  assert.deepEqual(layout(world.pond.basins[1]),{stones:[],woods:[],scenery:[]});
  record('prior-r5-split-shape',{worldId:world.worldId,basins,retained:layout(retainedLayout)});
  if (living) {
    assert.deepEqual(world.pond.basins.map((basin)=>[basin.shrimp.length,basin.daphniaCount,basin.floatingPlants.length]),[[1,4,1],[0,3,1]]);
    record('prior-r5-live-split',{worldId:world.worldId,basins:world.pond.basins.map(({id,shrimp,daphniaCount,floatingPlants})=>
      ({id,shrimp:shrimp.map(({id})=>id),daphniaCount,floatingPlants}))});
  }
  return world;
}

async function seedStructuresAndFragments(seedBotany) {
  let forest=await createWorld('blank-forest-r5-composed','安装升级验收 · 枝根桥和碎叶',14);
  for (const [index,edit] of [
    {kind:'addBarkShelter',position:{x:0.27,y:0.78},preset:'rootArch'},
    {kind:'addBarkShelter',position:{x:0.72,y:0.78},preset:'branchBridge'},
    {kind:'spreadLeafFragments',positions:[{x:0.2,y:0.63},{x:0.75,y:0.63}],mgC:28},
    ...(seedBotany ? [{kind:'addScenery',style:'acorn-shell-loose',x:0.44,y:0.87}] : []),
  ].entries()) forest=await ipc('editor','edit_forest_rocks',{
    ...editContext(forest,`upgrade-f05-f06-${index}`),action:{action:'change',edit},
  });
  assert.deepEqual(forest.forestRocks.barkShelters.map(({preset})=>preset),['rootArch','branchBridge']);
  assert.deepEqual(forest.forestRocks.leafPatches.map(({preset,remainingMgCnp})=>[preset,remainingMgCnp[0]]),
    [['fragments',14],['fragments',14]]);

  let pond=await createWorld('blank-pond-r5-composed','安装升级验收 · 浅塘通道',13);
  const basinId=pond.pond.basins[0].id;
  for (const [index,edit] of [
    {type:'addWood',x:0.19,preset:'hangingRoots'},
    {type:'addWood',x:0.5,preset:'rootNetwork'},
    {type:'addStone',x:0.8,preset:'flatStoneCave'},
    {type:'addWater',liters:18},
    ...(seedBotany ? [{type:'addSedge',anchor:{waterCell:63,shoreCell:51},preset:'medium'}] : []),
  ].entries()) pond=await ipc('editor','edit_pond',{
    ...editContext(pond,`upgrade-p03-p06-${index}`),edit:{basinId,...edit},
  });
  const basin=pond.pond.basins[0];
  assert.deepEqual([...basin.woods,...basin.stones].map(({preset})=>preset),['hangingRoots','rootNetwork','flatStoneCave']);
  assert.equal(basin.waterLiters,18);
  if (seedBotany) {
    const shells=forest.forestRocks.scenery.find(({style})=>style==='acorn-shell-loose');
    const sedge=basin.shoreSedges[0];
    assert.equal(sedge.rootWetFraction,1);
    assert.equal(sedge.stockMgCnp[0],32);
    assert.equal(shells.x,0.44);
    record('prior-r5-seed-shells-and-shore',{shells,sedge});
  }
  record('prior-r5-structures-and-fragments',{forestId:forest.worldId,pondId:pond.worldId,
    bridges:forest.forestRocks.barkShelters.map(({preset})=>preset),fragments:forest.forestRocks.leafPatches.length,
    woods:basin.woods.map(({preset})=>preset),cave:basin.stones[0].preset});
  return [forest,pond];
}

async function captureSecondBasin(snapshot, prefix, expectedStyles) {
  const [first,second]=snapshot.pond.basins;
  const {id,x,stage}=second.leafDeposits[0];
  const materials = expectedStyles.map(expectedStyle => {
    const items=second.scenery.filter(({style})=>style===expectedStyle);
    assert.equal(items.length,1);
    assert(!first.scenery.some(({style})=>style===expectedStyle));
    return items[0];
  });
  const materialSelector=materials.map(({id,x,y})=>`[data-pond-scenery-anchors*="${id}:${x*576},${y*324}"]`).join('');
  const inspector=`.editor-pond-field select:has(option[value="${second.id}"])`;
  if (!await editor(`Boolean(document.querySelector(${JSON.stringify(inspector)})?.getClientRects().length)`)) {
    await clickButton('环境','.editor-inspector-rail button');
  }
  await until(()=>editor(`(() => {
    const select=document.querySelector(${JSON.stringify(inspector)});
    if (!select?.getClientRects().length) return false;
    select.value=${JSON.stringify(second.id)};
    select.dispatchEvent(new Event('change',{bubbles:true}));
    return select.value===${JSON.stringify(second.id)};
  })()`),'Installed dual pond did not expose the second basin selector');
  const target=`.pond-scene[data-pond-id="${second.id}"] ${pondCanvas}[data-pond-leaf-stages="${id}:${stage}"]${materialSelector}`;
  const rendered=await canvas('editor',target);
  assert.equal(rendered.scene.pondFraming,'full');
  assert.equal(rendered.scene.pondLeafAnchors,`${id}:${x}`);
  for (const material of materials) {
    const match=rendered.scene.pondSceneryAnchors.split(';').find((anchor)=>anchor.startsWith(`${material.id}:`));
    assert(match,'The restored material needs a rendered scene anchor');
    const stagePoint=match.split(':')[1].split(',').map(Number);
    assert.deepEqual(stagePoint,[material.x*576,material.y*324]);
    const detail={basinId:second.id,style:material.style,stagePoint};
    if (material.style===gravelStyle) record('upgraded-r5-rendered-gravel',{...detail,gravelId:material.id});
    else record('upgraded-r5-rendered-sand',{...detail,id:material.id});
  }
  await png('editor',`${prefix}-second-basin-editor.png`);
  record('upgraded-r5-rendered-second-basin',{basinIds:[first.id,second.id],leafId:id,x,stage});
}
try {
  for (const page of pages) {
    const view = await connect(page);
    open.push(view);
    const name = await until(() => view.evaluate('window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label'), 'Missing window label');
    views.set(name, view);
    const origin = new URL(await view.evaluate('location.href'));
    assert.equal(origin.host,'tauri.localhost','Test the packaged production origin');
  }
  assert.deepEqual([...views.keys()].sort(), ['editor', 'layout', 'observe', 'panel']);
  await verifyOwnership();
  await showViews();
  await revealPanel();
  const library = await ipc('editor','list_worlds');
  const builtInIds = library.presets.filter((preset)=>preset.source!=='custom').map(({presetId})=>presetId);
  const currentPresetsOnly = builtInIds.includes('forest-r5-composed') && !builtInIds.includes('forest');
  if (currentPresetsOnly) assert.deepEqual(builtInIds, [
    'forest-r5-composed','blank-forest-r5-composed','pond-r5-composed',
    'pond-r5-dual','blank-pond-r5-composed','blank-pond-r5-dual',
  ]);

  if (mode==='legacy'||mode==='upgrade-seed') {
    assert.equal(library.worlds.length,1);
    assert(library.presets.some(({presetId})=>presetId==='pond'));
    const forest=await waitWorld('forest-understory',4);
    assert.equal(forest.definition,'forest-understory-seasonal/1');
    assert.equal(forest.worldName,'林下切片');
    const rendered=await canvas('panel',legacyForestCanvas);
    await png('editor','installed-editor.png');
    record('installed-legacy-first-start',{views:[...views.keys()],rendered,modelVersion:4,presetCount:library.presets.length});
  }

  if (mode==='r5'||mode==='r5-reopen'||mode==='upgrade'||(seedRelease && seedRelease!=='0.1.0')) {
    for (const [presetId,modelVersion] of [['forest-r5-composed',14],['blank-forest-r5-composed',14],
      ['pond-r5-composed',13],['blank-pond-r5-composed',13]]) {
      assert(library.presets.some((preset)=>preset.presetId===presetId&&preset.modelVersion===modelVersion));
    }
  }

  if (mode==='r5') {
    assert.equal(library.worlds.length,1);
    const forest = await waitWorld('forest-understory',14);
    assert.equal(forest.definition,'forest-r5-composed/1');
    assert.equal(forest.forestRocks.scenery.length,70);
    const rendered=await canvas('panel',forestCanvas);
    assert.match(rendered.scene.forestTerrainAnchors,/\b2:88,232\b/);
    const component=await backing('panel',forestCanvas,'installed-forest-panel.png');
    if(rendered.dpr===1.5) assert.deepEqual(component,[576,324]);
    await canvas('editor',forestCanvas);
    await png('editor','installed-editor.png');
    record('installed-r5-first-start-v14',{views:[...views.keys()],worldId:forest.worldId,definition:forest.definition,
      sceneItems:forest.forestRocks.scenery.length,component,dpr:rendered.dpr,modelVersion:14,presetCount:library.presets.length});

    const frozen = await ipc('editor','set_paused',{paused:true});
    assert(frozen.paused);
    const pond = await createWorld('pond-r5-composed','安装验收 · 浅塘 V2',13);
    assert.equal(pond.definition,'pond-shallow-r5-composed/1');
    assert.equal(pond.pond.basins[0].scenery.length,65);
    const widget=await canvas('panel',pondCanvas);
    const pondEditor=await canvas('editor',pondCanvas);
    assert.equal(widget.scene.pondFraming,'underwater');
    assert.equal(pondEditor.scene.pondFraming,'full');
    assert.equal(widget.scene.pondWoodAnchors,pondEditor.scene.pondWoodAnchors);
    await backing('panel',pondCanvas,'installed-pond-panel.png');
    await png('editor','installed-pond-editor.png');
    await writeFile(join(evidence,'reopen-expected.json'),JSON.stringify({activeWorldId:pond.worldId,initialPid:Number(pid),
      worlds:[frozen,pond].map((world)=>({state:worldState(world),runGeneration:world.runGeneration}))},null,2));
    record('installed-r5-create-v13',{worldId:pond.worldId,modelVersion:13,definition:pond.definition,sceneItems:pond.pond.basins[0].scenery.length,
      pondFraming:[widget.scene.pondFraming,pondEditor.scene.pondFraming],dimensions:[widget.width,widget.height]});
  }

  if(mode==='r5-reopen') {
    const expected=JSON.parse(await readFile(join(evidence,'reopen-expected.json'),'utf8'));
    assert.equal(library.activeWorldId,expected.activeWorldId);
    assert.notEqual(Number(pid),expected.initialPid);
    for(const item of expected.worlds) await compareSaved(item);
    await canvas('panel',pondCanvas);
    await backing('panel',pondCanvas,'reopened-pond-panel.png');
    record('installed-r5-both-worlds-exact-reopen',{previousPid:expected.initialPid,pid:Number(pid),
      worlds:expected.worlds.map(({state})=>({worldId:state.worldId,modelVersion:state.modelVersion,tick:state.simulationTick}))});
  }

  if (seedRelease) {
    const r5 = seedRelease!=='0.1.0';
    const seedTerrain = seedRelease==='0.2.2';
    const seedGravel = seedRelease==='0.2.4';
    const seedBotany = seedRelease==='0.2.19';
    const seedStructures = seedRelease==='0.2.18'||seedBotany;
    const seedLivingSections = seedRelease==='0.2.17'||seedStructures;
    const seedPlantSections = seedRelease==='0.2.16'||seedLivingSections;
    const seedLeafSections = seedRelease==='0.2.15'||seedPlantSections;
    const seedStaticSections = seedRelease==='0.2.14'||seedLeafSections;
    const seedLiveTopology = seedRelease==='0.2.13'||seedStaticSections;
    const seedStoneBiofilm = seedRelease==='0.2.12'||seedLiveTopology;
    const seedFloating = seedRelease==='0.2.11'||seedStoneBiofilm;
    const seedRibbon = seedRelease==='0.2.10'||seedFloating;
    const seedStonePlant = seedRelease==='0.2.9'||seedRibbon;
    const seedStatic = seedRelease==='0.2.8'||seedStonePlant;
    const seedTopology = seedRelease==='0.2.7'||seedStatic;
    const seedSand = seedRelease==='0.2.5'||seedRelease==='0.2.6'||seedTopology;
    const seedLiving = seedRelease==='0.2.3'||seedGravel||seedSand;
    if (r5) {
      assert.equal(library.worlds.length,1);
      await waitWorld('forest-understory',14);
    }
    let oldForest=await ipc('editor','set_paused',{paused:true});
    if (seedTerrain) oldForest=await seedEditedForest(oldForest);
    if (seedLiving) {
      assert.equal(library.presets.length,seedTopology ? 18 : 17);
      assert(library.presets.some(({presetId,modelVersion})=>presetId==='pond-r5-dual'&&modelVersion===13));
      oldForest=await seedLivingFern(oldForest);
    }
    let oldPond=seedLiving ? await createWorld('pond-r5-dual','安装升级验收 · R5 双水域',13) :
      r5 ? await createWorld('pond-r5-composed','安装升级验收 · R5 浅塘',13) : await createWorld('pond','安装升级验收 · 原浅塘',3);
    if (seedTerrain) oldPond=await seedEditedPond(oldPond);
    if (seedLiving) oldPond=await seedLivingDualPond(oldPond);
    if (seedGravel) oldPond=await seedGravelPatch(oldPond);
    if (seedSand) oldPond=await seedSandPatches(oldPond);
    if (seedStonePlant) {
      const [first,second]=oldPond.pond.basins;
      assert.equal(first.stoneRestEnabled,true);
      oldPond=await ipc('editor','edit_pond',{...editContext(oldPond,'upgrade-whorled-plant'),
        edit:{type:'addWaterPlant',basinId:first.id,x:0.36}});
      const [plant]=oldPond.pond.basins[0].waterPlantPatches;
      assert.equal(plant.x,0.36);
      assert.equal(plant.ribbonPreset,undefined);
      assert.deepEqual(oldPond.pond.basins[1],second);
      record('prior-r5-stone-and-whorled-plant',{basinId:first.id,stoneRestEnabled:first.stoneRestEnabled,plant});
    }
    if (seedRibbon) {
      const [first,second]=oldPond.pond.basins;
      oldPond=await ipc('editor','edit_pond',{...editContext(oldPond,'upgrade-ribbon-plant'),
        edit:{type:'placeRibbonPlant',basinId:first.id,x:0.81,preset:'large'}});
      const [whorled,ribbon]=oldPond.pond.basins[0].waterPlantPatches;
      assert.equal(whorled.id,first.waterPlantPatches[0].id);
      assert.equal(ribbon.x,0.81);
      assert.equal(ribbon.ribbonPreset,'large');
      assert.equal(ribbon.stockMgCnp[0],75);
      assert.deepEqual(oldPond.pond.basins[1],second);
      record('prior-r5-ribbon-plant',{basinId:first.id,ribbon});
    }
    if (seedFloating) oldPond=await seedFloatingPlant(oldPond);
    if (seedStoneBiofilm) {
      const [first,second]=oldPond.pond.basins;
      oldPond=await ipc('editor','edit_pond',{...editContext(oldPond,'upgrade-local-biofilm'),
        edit:{type:'enableStoneBiofilm',basinId:first.id}});
      const after=oldPond.pond.basins[0];
      assert.equal(after.stoneBiofilmEnabled,true);
      assert(after.stoneBiofilm.some(({stockMgC})=>stockMgC>0));
      assert(Math.abs(after.biofilmMgC-first.biofilmMgC)<1e-10);
      assert.deepEqual(oldPond.pond.basins[1],second);
      record('prior-r5-positive-stone-biofilm',{basinId:first.id,stocks:after.stoneBiofilm});
    }
    assert(oldForest.paused && oldPond.paused);
    assert.equal(oldForest.modelVersion,r5 ? 14 : 4);
    assert.equal(oldForest.definition,r5 ? 'forest-r5-composed/1' : 'forest-understory-seasonal/1');
    assert.equal(oldPond.definition,r5 ? 'pond-shallow-r5-composed/1' : 'pond-shallow/1');
    if (seedRelease==='0.2.1') {
      assert(!('substrateCells' in oldForest.forestRocks) && !('substratePresets' in oldForest.forestRocks));
      assert(oldPond.pond.basins.every((basin) => !('shape' in basin)));
    }
    if (seedTerrain) {
      assert(!('fernPatches' in oldForest.forestRocks) && !('fernPreset' in oldForest.forestRocks));
      assert(oldPond.pond.basins.every((basin) => !('leafDeposits' in basin)));
    }
    const shapedPond = seedTopology ? await seedTopologyWorld(await createWorld('blank-pond-r5-dual','安装升级验收 · 分池存档',13),seedStatic,seedLiveTopology) : null;
    let sectionPond;
    if (seedStaticSections) {
      sectionPond=await createWorld('blank-pond-r5-dual','安装升级验收 · 主剖面沉叶与副剖面布置',13);
      const [first,second]=sectionPond.pond.basins;
      for (const [name,edit] of [
        ['water-a',{type:'addWater',basinId:first.id,liters:5}],
        ['water-b',{type:'addWater',basinId:second.id,liters:7}],
        ['leaf-a',{type:'addLeafDeposit',basinId:first.id,x:0.33,preset:'midground',mgC:30}],
        ...(seedLeafSections ? [['leaf-b',{type:'addLeafDeposit',basinId:second.id,x:0.67,preset:'foreground',mgC:45}]] : []),
        ['stone-b',{type:'addStone',basinId:second.id,x:0.65,preset:'main'}],
        ['root-b',{type:'addWood',basinId:second.id,x:0.28,preset:'root'}],
        ...(seedPlantSections ? [['plant-b',{type:'placeRibbonPlant',basinId:second.id,x:0.85,preset:'small'}]] : []),
        ...(seedLivingSections ? [
          ['moss-b',{type:'addMoss',basinId:second.id,parent:{kind:'stone',id:1},preset:'mat',offset:0}],
          ['shrimp-b',{type:'stockShrimp',basinId:second.id,position:{x:0.9,y:0.35},count:1}],
        ] : []),
        ['merge',{type:'commitTopology',basinId:first.id,mode:'add',cells:[29]}],
      ]) sectionPond=await ipc('editor','edit_pond',{...editContext(sectionPond,`upgrade-section-${name}`),edit});
      assert.equal(sectionPond.pond.basins.length,1);
      const [merged]=sectionPond.pond.basins;
      assert.deepEqual(merged.leafDeposits.map(({id,remainingMgC})=>[id,remainingMgC]),[[1,30]]);
      assert.deepEqual(merged.staticSections.map(({row,stones,woods,leafDeposits})=>[row,stones.length,woods.length,
        leafDeposits?.map(({id,x,remainingMgC})=>[id,x,remainingMgC])]),
        [[second.shape.protectedRow,1,1,seedLeafSections ? [[1,0.67,45]] : undefined]]);
      assert.deepEqual(merged.staticSections[0].waterPlantPatches?.map(({id,x,ribbonPreset,stockMgCnp})=>[id,x,ribbonPreset,stockMgCnp[0]]),
        seedPlantSections ? [[1,0.85,'small',30]] : undefined);
      if (seedLivingSections) {
        assert.equal(merged.staticSections[0].shrimp[0].id,`${second.id}-shrimp-1`);
        assert.deepEqual(merged.staticSections[0].mossPatches[0].parent,{kind:'stone',id:1});
      }
      record('prior-r5-static-section',{worldId:sectionPond.worldId,mainLeafMgC:30,side:merged.staticSections[0]});
    }
    const structureWorlds=seedStructures ? await seedStructuresAndFragments(seedBotany) : [];
    const snapshots = [oldForest,oldPond,shapedPond,sectionPond,...structureWorlds].filter(Boolean);
    const priorRelease=seedRelease;
    const worlds=snapshots.map((world)=>{
      const state=worldState(world);
      return {state,runGeneration:world.runGeneration,stateSha256:canonicalDigest(state)};
    });
    await writeFile(join(evidence,'prior-worlds-expected.json'),JSON.stringify({activeWorldId:snapshots.at(-1).worldId,priorPid:Number(pid),priorRelease,worlds},null,2));
    record('prior-install-created-saved-worlds',{priorRelease,forestId:oldForest.worldId,forestVersion:oldForest.modelVersion,
      pondId:oldPond.worldId,pondVersion:oldPond.modelVersion,stateSha256:worlds.map((item)=>item.stateSha256)});
  }

  if(mode==='upgrade') {
    const expected=JSON.parse(await readFile(join(evidence,'prior-worlds-expected.json'),'utf8'));
    assert.notEqual(Number(pid),expected.priorPid);
    assert([...priorReleaseBySeed.values()].includes(expected.priorRelease));
    const r5=expected.priorRelease!=='0.1.0';
    if (currentPresetsOnly && !r5) {
      assert.notEqual(library.activeWorldId,expected.activeWorldId);
      const prior = expected.worlds.find(({state})=>state.worldId===expected.activeWorldId);
      assert(prior);
      const version = prior.state.modelVersion===3 ? 13 : 14;
      await waitWorld(library.activeWorldId,version);
    } else {
      assert.equal(library.activeWorldId,expected.activeWorldId);
    }
    const priorGravel=expected.priorRelease==='0.2.4';
    const priorTopology=expected.priorRelease==='0.2.6';
    const priorStructures=['0.2.18','0.2.19'].includes(expected.priorRelease);
    const priorStaticSections=priorStructures||['0.2.14','0.2.15','0.2.16','0.2.17'].includes(expected.priorRelease);
    const priorSplit=priorStaticSections||['0.2.7','0.2.8','0.2.9','0.2.10','0.2.11','0.2.12','0.2.13'].includes(expected.priorRelease);
    const priorSand=expected.priorRelease==='0.2.5'||priorTopology||priorSplit;
    const priorLiving=expected.priorRelease==='0.2.3'||priorGravel||priorSand;
    if (priorLiving && !currentPresetsOnly) assert.equal(library.presets.length,priorTopology||priorSplit ? 18 : 17);
    if (priorTopology||priorSplit) assert(library.presets.some(({presetId,modelVersion})=>presetId==='blank-pond-r5-dual'&&modelVersion===13));
    assert.deepEqual(expected.worlds.map(({state})=>state.modelVersion),priorStructures ? [14,13,13,13,14,13]
      : priorStaticSections ? [14,13,13,13] : priorSplit ? [14,13,13] : r5 ? [14,13] : [4,3]);
    const worlds=[];
    for(const item of expected.worlds) {
      const {current:restored,stateSha256,verifiedDerivedFields}=await compareSaved(item,expected.priorRelease);
      const forest=restored.modelVersion===14||restored.modelVersion===4;
      const structureForest=priorStructures&&restored.forestRocks?.barkShelters.some(({preset})=>preset==='rootArch');
      const structurePond=priorStructures&&restored.pond?.basins[0].stones.some(({preset})=>preset==='flatStoneCave');
      const currentArt = r5 || currentPresetsOnly;
      const selector=forest ? (currentArt ? forestCanvas : legacyForestCanvas) : (currentArt ? pondCanvas : legacyPondCanvas);
      const split = priorSplit && restored.pond?.basins.some(({id})=>id==='pond-3');
      const storedSide=restored.pond?.basins[0].staticSections?.[0];
      const prefix=`upgraded-${r5 ? 'r5' : 'legacy'}-v${restored.modelVersion}${split ? '-split' : storedSide ? '-sections'
        : structureForest||structurePond ? '-structures' : ''}`;
      const floating=restored.pond?.basins[0].floatingCanopy;
      const canopySelector=floating ? `[data-pond-floating-canopy-coverage="${floating.coverage}"][data-pond-floating-plant-anchors]` : '';
      const editorSelector=`main.world-editor[data-world-id=${JSON.stringify(restored.worldId)}] ${selector}${canopySelector}`;
      const rendered=await canvas('editor',editorSelector);
      await png('editor',`${prefix}-editor.png`);
      let component;
      if (r5) {
        const widgetSelector=forest ? selector : `${selector}[data-pond-wood-anchors=${JSON.stringify(rendered.scene.pondWoodAnchors)}]${canopySelector}`;
        const widget=await canvas('panel',widgetSelector);
        if (forest) {
          assert.equal(widget.scene.forestTerrainAnchors,rendered.scene.forestTerrainAnchors);
          if(structureForest) {
            assert.equal(rendered.scene.forestBridgeIds,restored.forestRocks.barkShelters.map(({id})=>id).join(','));
            assert.equal(widget.scene.forestBridgeIds,rendered.scene.forestBridgeIds);
            assert.equal(restored.forestRocks.leafPatches.length,2);
          } else {
            assert.match(rendered.scene.forestTerrainAnchors,/\b2:88,232\b/);
          }
          if(priorLiving&&!structureForest) {
            const fern=restored.forestRocks.fernPatches[0];
            assert.equal(rendered.scene.forestFernIds,String(fern.id));
            assert.equal(widget.scene.forestFernIds,String(fern.id));
          }
        } else {
          assert.equal(rendered.scene.pondFraming,'full');
          assert.equal(widget.scene.pondFraming,'underwater');
          assert.equal(widget.scene.pondWoodAnchors,rendered.scene.pondWoodAnchors);
          if(structurePond) {
            assert.match(rendered.scene.pondStructureSprites,/flatStoneCave/);
            assert.equal(widget.scene.pondStructureSprites,rendered.scene.pondStructureSprites);
          }
          if (floating) {
            assert.equal(Number(rendered.scene.pondFloatingLightTransmission),floating.lightTransmission);
            assert.equal(Number(widget.scene.pondFloatingLightTransmission),floating.lightTransmission);
            assert.equal(widget.scene.pondFloatingPlantAnchors,rendered.scene.pondFloatingPlantAnchors);
            const basin=restored.pond.basins[0];
            record('upgraded-r5-rendered-floating-plant',{basinId:basin.id,plant:basin.floatingPlants[0],canopy:floating});
          }
        }
        component=await backing('panel',widgetSelector,`${prefix}-panel.png`);
        if (widget.dpr===1.5) assert.deepEqual(component,[576,324]);
      }
      if (priorLiving&&!forest&&!split&&!storedSide&&!structurePond) await captureSecondBasin(restored,prefix,priorSand ? sandStyles : priorGravel ? [gravelStyle] : []);
      if (storedSide) {
        await until(()=>editor(`(() => {
          const control=document.querySelector('[data-pond-static-section-select]');
          if (!control || !Array.from(control.options).some(option=>option.value===${JSON.stringify(String(storedSide.row))})) return false;
          control.value=${JSON.stringify(String(storedSide.row))};
          control.dispatchEvent(new Event('change',{bubbles:true})); return true;
        })()`),'Stored secondary section did not appear in the upgraded editor');
        const secondary=await canvas('editor',`[data-pond-static-section-row="${storedSide.row}"] ${pondCanvas}`);
        assert.equal(secondary.scene.pondLeafAnchors,storedSide.leafDeposits?.map(({id,x})=>id+':'+x).join(';'));
        if (storedSide.waterPlantPatches?.length) {
          assert.equal(secondary.scene.pondRibbonPlants,storedSide.waterPlantPatches.map(({id,ribbonPreset})=>id+':'+ribbonPreset).join(';'));
        }
        await png('editor',`${prefix}-secondary.png`);
        record('upgraded-r5-static-section',{worldId:restored.worldId,mainLeafMgC:restored.pond.basins[0].leafDeposits[0].remainingMgC,side:storedSide});
      }
      if (split) record('upgraded-r5-split-shape',{worldId:restored.worldId,basins:restored.pond.basins.map(({id,waterLiters,detritusMgC,shape})=>[id,waterLiters,detritusMgC,shape.protectedRow])});
      worlds.push({worldId:restored.worldId,modelVersion:restored.modelVersion,definition:restored.definition,
        tick:restored.simulationTick,stateSha256,verifiedDerivedFields,component});
    }
    record('upgrade-reopens-old-worlds-exactly',{priorRelease:expected.priorRelease,priorPid:expected.priorPid,pid:Number(pid),worlds});
    if (expected.priorRelease==='0.2.8') {
      const unchanged=await snapshot();
      const [selected,neighbor]=unchanged.pond.basins;
      assert.equal(unchanged.editPaused,true);
      assert(selected.stones.some(({preset})=>preset==='main'));
      assert(unchanged.pond.basins.every((basin)=>!('stoneRestEnabled' in basin)));
      await clickButton('生物','.editor-inspector-rail button');
      await until(()=>editor(`(() => {
        const details=document.querySelector('[data-pond-stone-feeding-readout]');
        if(!details) return false;
        details.open=true;
        return true;
      })()`),'Installed old world did not open the main-stone readout');
      await clickButton('启用主石停靠','[data-pond-stone-feeding-readout] button');
      const enabled=await until(async()=>{
        const current=await snapshot();
        return current.pond.basins[0].stoneRestEnabled===true && current;
      },'Explicit main-stone edit did not affect the selected old basin');
      assert.equal(enabled.simulationTick,unchanged.simulationTick);
      assert.equal(BigInt(enabled.editRevision),BigInt(unchanged.editRevision)+1n);
      assert.deepEqual(enabled.pond.basins[1],neighbor);
      assert.equal(enabled.pond.basins[0].stoneContactFeeding,undefined);
      record('upgraded-r5-explicit-stone-opt-in',{worldId:enabled.worldId,basinId:selected.id,unchangedNeighbor:neighbor.id,
        tick:enabled.simulationTick,noRetroactiveIntake:true});
    }
  }

  await editor("setTimeout(() => { void window.__TAURI_INTERNALS__.invoke('request_exit'); }, 100); true");
} finally {
  for (const view of open) view.close();
}
