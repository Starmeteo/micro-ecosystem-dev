import assert from 'node:assert/strict';
import { appendFile, readFile, rename, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const [mode, port, directory, durationArg = '10', soakArg = '120'] = process.argv.slice(2);
const labels = ['panel', 'editor', 'observe', 'layout'];
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const output = join(directory, 'checks.jsonl');
const trace = join(directory, 'native.jsonl');
const sessions = new Map();

async function eventually(read, description, timeout = 15_000) {
  const until = Date.now() + timeout;
  do {
    const value = await read();
    if (value) return value;
    await delay(120);
  } while (Date.now() < until);
  throw new Error(description);
}

async function rows(path) {
  const raw = await readFile(path, 'utf8').catch((error) => {
    if (error.code === 'ENOENT') return '';
    throw error;
  });
  return raw.replace(/^\uFEFF/, '').split('\n').filter(Boolean).flatMap((line) => {
    try { return [JSON.parse(line)]; } catch { return []; }
  });
}

async function record(check, payload) {
  const line = JSON.stringify({ check, ...payload });
  await appendFile(output, `${line}\n`);
  console.log(line);
}

async function connect(url) {
  const socket = new WebSocket(url);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  let lastId = 0;
  const pending = new Map();
  socket.addEventListener('message', ({ data }) => {
    const message = JSON.parse(data);
    pending.get(message.id)?.(message);
  });
  function send(method, params) {
    return new Promise((resolve, reject) => {
      const id = ++lastId;
      const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 12_000);
      pending.set(id, (message) => {
        clearTimeout(timer);
        pending.delete(id);
        if (message.error || message.result?.exceptionDetails) reject(new Error(JSON.stringify(message.error || message.result.exceptionDetails)));
        else resolve(message.result);
      });
      socket.send(JSON.stringify({ id, method, params }));
    });
  }
  return { async evaluate(expression) {
    return (await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })).result.value;
  }, close: () => socket.close(), send };
}

async function discover() {
  const targets = await eventually(async () => {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`);
      const pages = (await response.json()).filter((page) => page.type === 'page');
      return pages.length >= labels.length ? pages : null;
    } catch { return null; }
  }, 'Windows WebView2 targets unavailable', 30_000);
  for (const target of targets) {
    const session = await connect(target.webSocketDebuggerUrl);
    const label = await eventually(() => session.evaluate('window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label'), 'An expected WebView did not initialize its native label', 5000);
    if (labels.includes(label) && !sessions.has(label)) sessions.set(label, session);
    else session.close();
  }
  for (const label of labels) assert(sessions.has(label), `Missing integrated window ${label}`);
}

const evaluate = (view, expression) => sessions.get(view).evaluate(expression);
const ipc = (view, command, payload = {}) => evaluate(view, `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(payload)})`);
const show = (viewId) => ipc('editor', 'show_window', { viewId });
const hideAll = () => ipc('editor', 'hide_windows');
const worldList = () => ipc('editor', 'list_worlds');
const tick = (library, id) => Number(library.worlds.find((world) => world.worldId === id)?.simulationTick);

async function workspace(mode, worldId = '') {
  const layout = await ipc('layout', 'get_workspace_layout');
  await ipc('layout', 'save_workspace_idle', { idle: { mode, text: '' }, layout: layout ?? {
    version: 2, mode: 'combined', idle: { mode, text: '' }, items: [{ id: 'forest', kind: 'ecology', x: 772, y: 460 }],
  }, panelWorldId: worldId });
}

async function exitApplication(view = 'panel') {
  await evaluate(view, "setTimeout(() => window.__TAURI_INTERNALS__.invoke('request_exit'), 80); true");
}

async function standby() {
  await show('editor');
  const forest = (await worldList()).activeWorldId;
  const pond = await ipc('editor', 'create_world', { presetId: 'pond', name: '待机探针 · 暂停浅塘' });
  assert(pond.editPaused);
  const running = await ipc('editor', 'switch_world', { worldId: forest });
  assert.equal(running.worldId, forest);
  assert(!running.paused);
  await hideAll();
  await eventually(async () => (await rows(trace)).some((row) => row.event === 'power-registration' && row.details.registered), 'Power registration failed');
  const ready = { wallMillis: Date.now(), forest, pond: pond.worldId, pausedTick: pond.simulationTick };
  await writeFile(join(directory, 'standby-ready.json'), JSON.stringify(ready));
  console.log('M4_STANDBY_READY');

  const completed = await eventually(async () => {
    const transitions = (await rows(trace)).filter((row) => row.event === 'power-transition' && row.wallMillis >= ready.wallMillis);
    const suspend = transitions.find((row) => row.suspended);
    const resume = suspend && transitions.find((row) => !row.suspended && row.wallMillis > suspend.wallMillis);
    return resume ? { suspend, resume } : null;
  }, '未收到完整的 Windows 挂起和恢复事件', 300_000);
  const { suspend, resume } = completed;
  assert.equal(suspend.worldId, forest);
  assert.equal(resume.runGeneration, suspend.runGeneration);
  assert.equal(resume.tick, suspend.tick);
  for (const event of [suspend, resume]) {
    const paused = event.details.background.find((world) => world.worldId === pond.worldId);
    assert(paused?.paused);
    assert.equal(paused.tick, pond.simulationTick);
  }
  const resumed = await eventually(async () => (await rows(trace)).find((row) => row.event === 'active-clock-sample'
    && row.wallMillis > resume.wallMillis && Number(row.tick) >= Number(resume.tick) + 5), 'World did not continue after wake');
  await record('standby', { ready, suspend, resume, firstRunningSample: resumed });
  await show('panel');
  await exitApplication();
}

function statistics(values) {
  const sorted = values.filter(Number.isFinite).sort((a, b) => a - b);
  const at = (part) => sorted[Math.max(0, Math.ceil(part * sorted.length) - 1)];
  return sorted.length ? { samples: sorted.length, median: at(.5), p95: at(.95), maximum: sorted.at(-1) } : null;
}

async function timing(view) {
  const frames = await evaluate(view, `new Promise((resolve) => {
    const times = []; let first, previous;
    function sample(now) {
      if (first === undefined) first = now;
      if (previous !== undefined) times.push(now - previous);
      previous = now;
      if (now - first < 1800) requestAnimationFrame(sample); else resolve(times);
    }
    requestAnimationFrame(sample);
  })`);
  const ipcResults = await evaluate(view, `(async () => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const viewId = ${JSON.stringify(view)};
    const { snapshot, subscriptionToken } = await invoke('subscribe_world', { viewId });
    try {
      const baseline = { viewId, subscriptionToken };
      const payloadBytes = new TextEncoder().encode(JSON.stringify(snapshot)).length;
      const roundTripMs = [];
      for (let i = 0; i < 32; i++) {
        const start = performance.now();
        await invoke('poll_world', baseline);
        roundTripMs.push(performance.now() - start);
      }
      return { payloadBytes, roundTripMs, devicePixelRatio, width: innerWidth, height: innerHeight };
    } finally { await invoke('unsubscribe_world', { viewId, subscriptionToken }); }
  })()`);
  await record('frame-and-ipc', { view, rafIntervalsMs: statistics(frames), ...ipcResults, roundTripMs: statistics(ipcResults.roundTripMs) });
}

async function phase(name, seconds, before = async () => {}, after = async () => {}) {
  await before();
  await delay(650);
  const started = Date.now();
  await writeFile(join(directory, 'phase-next.json'), JSON.stringify({ name, started }));
  await rename(join(directory, 'phase-next.json'), join(directory, 'phase.json'));
  await delay(seconds * 1000);
  await record('phase', { name, started, ended: Date.now() });
  await after();
}

async function performanceRun() {
  const duration = Number(durationArg);
  const soak = Number(soakArg);
  await workspace('off');
  const forest = (await worldList()).activeWorldId;
  await phase('single-hidden', duration, hideAll);
  await phase('single-live-panel', duration, () => show('panel'));
  await phase('single-observe', duration, async () => { await hideAll(); await show('observe'); }, () => timing('observe'));
  await phase('single-two-views', duration, () => show('panel'));
  await phase('single-covered', duration, async () => {
    await hideAll();
    await workspace('clock');
    await show('panel');
    await evaluate('panel', `(() => {
      document.activeElement?.blur();
      document.querySelector('[data-idle-group="true"]')?.dispatchEvent(new PointerEvent('pointerout',{bubbles:true}));
    })()`);
    await eventually(() => evaluate('panel', "Boolean(document.querySelector('.landscape--covered'))"), 'Panel idle cover did not appear', 12_000);
  }, async () => {
    await evaluate('panel', "document.querySelector('.landscape').focus()");
    await eventually(() => evaluate('panel', "!document.querySelector('.landscape--covered')"), 'Panel idle cover did not clear on keyboard focus');
  });

  await show('editor');
  const pond = await ipc('editor', 'create_world', { presetId: 'pond', name: '双主题性能探针' });
  await ipc('editor', 'set_edit_paused', { paused: false });
  await workspace('off', forest);
  const start = await worldList();
  await phase('dual-hidden', duration, hideAll);
  let cycles = 0;
  await phase('dual-two-views', duration, async () => { await show('panel'); await show('observe'); });
  await hideAll();
  for (let cycle = 0; cycle < 24; cycle++) {
    await ipc('editor', 'show_windows');
    await delay(180);
    await hideAll();
    await delay(210);
    cycles++;
  }
  const hiddenAt = Date.now();
  await eventually(async () => (await rows(trace)).some((row) => row.event === 'active-clock-sample'
    && row.wallMillis >= hiddenAt && row.subscriberCount === 0 && row.visibleViews.length === 0), 'Hidden views leaked subscriptions');
  await record('view-cycle', { cycles, subscribersAfterHide: 0 });
  await phase('dual-visible-soak', soak, async () => { await show('panel'); await show('observe'); });
  const finish = await worldList();
  const deltas = { forest: tick(finish, forest) - tick(start, forest), pond: tick(finish, pond.worldId) - tick(start, pond.worldId) };
  assert(deltas.forest > 0 && deltas.pond > 0);
  assert(Math.abs(deltas.forest - deltas.pond) <= 2, `Independent running worlds drifted: ${JSON.stringify(deltas)}`);
  await record('dual-world-ticks', { forestId: forest, pondId: pond.worldId, deltas });
  await exitApplication();
}

async function captureWorld(theme, selector, snapshot) {
  await show('panel');
  await show('observe');
  for (const view of ['panel', 'observe']) {
    const canvas = await eventually(() => evaluate(view, `(() => {
      const element = document.querySelector(${JSON.stringify(selector)});
      return element?.width && element?.height && !document.querySelector('.forest-view__error, .pond-scene__status')
        ? { width: element.width, height: element.height, devicePixelRatio } : null;
    })()`), `${theme} (${view}) did not render its actual native canvas`);
    const picture = await sessions.get(view).send('Page.captureScreenshot', { format: 'png', fromSurface: true });
    const file = `${theme}-${view}.png`;
    await writeFile(join(directory, file), Buffer.from(picture.data, 'base64'));
    await record('capture', { view, file, canvas, worldId: snapshot.worldId, tick: snapshot.simulationTick, daylightPhase: snapshot.daylightPhase });
  }
}

async function visual() {
  await workspace('off');
  await show('editor');
  const forest = await ipc('editor', 'set_paused', { paused: true });
  await captureWorld('forest', '.forest-view canvas', forest);
  const pond = await ipc('editor', 'create_world', { presetId: 'pond', name: '浅水池塘' });
  assert(pond.editPaused);
  await captureWorld('pond', '.pond-scene canvas', pond);
  await exitApplication();
}

async function report() {
  const checks = await rows(output);
  const samples = await rows(join(directory, 'resource-samples.jsonl'));
  const phases = checks.filter((row) => row.check === 'phase');
  const summary = phases.map(({ name, started, ended }) => {
    const group = samples.filter((row) => row.wallMillis >= started + 1000 && row.wallMillis <= ended && row.phase === name);
    return { name, seconds: (ended - started) / 1000, cpuMachinePercent: statistics(group.map((row) => row.cpuMachinePercent)),
      gpuBusiestEnginePercent: statistics(group.map((row) => row.gpuBusiestEnginePercent)),
      privateMiB: statistics(group.map((row) => row.privateMiB)), workingSetMiB: statistics(group.map((row) => row.workingSetMiB)),
      privateMiBFirstAndLast: [group[0]?.privateMiB, group.at(-1)?.privateMiB] };
  });
  await writeFile(join(directory, 'summary.json'), JSON.stringify({ summary, checks }, null, 2));
}

if (mode === 'report') await report();
else {
  try {
    await discover();
    if (mode === 'standby') await standby();
    else if (mode === 'performance') await performanceRun();
    else if (mode === 'visual') await visual();
    else throw new Error('Expected performance, standby, visual or report');
  } finally {
    for (const session of sessions.values()) session.close();
  }
}
