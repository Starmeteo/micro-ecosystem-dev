import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cp, mkdir, readFile, readdir, rename, unlink, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const [phase, port, evidence] = process.argv.slice(2);
const data = join(evidence, phase === 'auto-reopen' ? 'auto' : phase, 'data');
const layoutPath = (directory) => join(directory, 'workspace', 'layout.json');
const scenarios = ['pending', 'newest-corrupt', 'write-failure', 'all-corrupt', 'future-format', 'missing-definition', 'locked'];
const healthyCases = ['pending', 'newest-corrupt', 'write-failure'];
const mark = (caseName, fields) => console.log(JSON.stringify({ case: caseName, ...fields }));
const interval = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
const readJson = async (path) => JSON.parse(await readFile(path, 'utf8'));
const name = (serial) => `checkpoint-${String(serial).padStart(20, '0')}.json`;
const directoryFor = (base, id) => join(base, `world-${id}`);

async function checkpointFiles(base, id) {
  return (await readdir(directoryFor(base, id))).filter((item) => /^checkpoint-\d{20}\.json$/.test(item)).sort();
}

async function recordHashes(base, id, files) {
  return Object.fromEntries(await Promise.all(files.map(async (file) =>
    [file, digest(await readFile(join(directoryFor(base, id), file)))])));
}

function checksum(serial, payload) {
  const serialBytes = Buffer.alloc(8);
  serialBytes.writeBigUInt64LE(BigInt(serial));
  let hash = 0xcbf29ce484222325n;
  for (const byte of Buffer.concat([serialBytes, Buffer.from(payload)])) {
    hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * 0x100000001b3n);
  }
  return hash.toString(16).padStart(16, '0');
}

async function prepare() {
  const seed = await readJson(join(evidence, 'seed.json'));
  const seedData = join(evidence, 'seed', 'data');
  const files = await checkpointFiles(seedData, seed.worldId);
  assert.deepEqual(files, [name(1), name(2)]);
  for (const scenario of scenarios) {
    const scenarioRoot = join(evidence, scenario);
    const target = join(scenarioRoot, 'data');
    await mkdir(scenarioRoot);
    await cp(seedData, target, { recursive: true, errorOnExist: true, force: false });
    const directory = directoryFor(target, seed.worldId);
    const latest = join(directory, files.at(-1));
    const current = await readJson(latest);
    if (scenario === 'pending') {
      await writeFile(join(directory, 'pending-truncated.tmp'), '{"magic":"micro-', { flag: 'wx' });
      const payload = current.payload.replace('"editRevision":1', '"editRevision":999');
      assert.notEqual(payload, current.payload);
      await writeFile(join(directory, 'pending-complete.tmp'), JSON.stringify({
        ...current, checkpointSerial: 3, payload, checksum: checksum(3, payload),
      }), { flag: 'wx' });
    } else if (scenario === 'newest-corrupt') {
      const payload = current.payload.replace('"editRevision":1', '"editRevision":2');
      assert.notEqual(payload, current.payload);
      await writeFile(latest, JSON.stringify({ ...current, payload }));
    } else if (scenario === 'all-corrupt') {
      for (const file of files) await writeFile(join(directory, file), '{');
    } else if (scenario === 'future-format') {
      await writeFile(latest, JSON.stringify({ ...current, formatVersion: 2 }));
    } else if (scenario === 'missing-definition') {
      const payload = current.payload.replace('pond-shallow/1', 'pond-shallow/unavailable');
      assert.notEqual(payload, current.payload);
      await writeFile(latest, JSON.stringify({ ...current, payload, checksum: checksum(current.checkpointSerial, payload) }));
    }
    const pendingFiles = (await readdir(directory)).filter((file) => file.startsWith('pending-')).sort();
    const monitored = [...files, ...pendingFiles];
    await writeFile(join(scenarioRoot, 'before.json'), JSON.stringify({
      worldId: seed.worldId, monitored, worldHashes: await recordHashes(target, seed.worldId, monitored),
      layoutHash: digest(await readFile(layoutPath(target))),
      activeIndexHash: digest(await readFile(join(target, 'active-world.json'))),
    }, null, 2), { flag: 'wx' });
    mark('fixture', { scenario, originalSerials: files.length, pendingFiles: pendingFiles.length });
  }
}

async function waitFor(action, message) {
  for (let attempt = 0; attempt < 180; attempt += 1) {
    const result = await action();
    if (result) return result;
    await interval(60);
  }
  throw new Error(message);
}

async function connect(target) {
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  let sequence = 0;
  const pending = new Map();
  socket.addEventListener('message', ({ data: bytes }) => {
    const message = JSON.parse(bytes);
    pending.get(message.id)?.(message);
  });
  function send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++sequence;
      const timeout = setTimeout(() => {
        pending.delete(id);
        reject(new Error(`${method} timed out`));
      }, 10_000);
      pending.set(id, ({ error, result }) => {
        pending.delete(id);
        clearTimeout(timeout);
        if (error || result?.exceptionDetails) reject(new Error(JSON.stringify(error ?? result.exceptionDetails)));
        else resolve(result);
      });
      socket.send(JSON.stringify({ id, method, params }));
    });
  }
  return {
    async evaluate(expression) {
      const response = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      return response.result.value;
    },
    async screenshot(path) {
      const { data: bytes } = await send('Page.captureScreenshot', { format: 'png', fromSurface: true });
      await writeFile(path, Buffer.from(bytes, 'base64'));
    },
    close: () => socket.close(),
  };
}

async function discover() {
  const targets = await waitFor(async () => {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`);
      const pages = (await response.json()).filter((target) => target.type === 'page');
      return pages.length >= 4 ? pages : null;
    } catch { return null; }
  }, 'The isolated desktop did not expose its WebViews');
  const sessions = new Map();
  for (const target of targets) {
    const session = await connect(target);
    const label = await waitFor(() => session.evaluate('window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label'),
      'Native WebView did not initialize its window label');
    if (['panel', 'editor'].includes(label)) sessions.set(label, session);
    else session.close();
  }
  assert(sessions.has('panel') && sessions.has('editor'));
  return sessions;
}

function invoke(session, command, args = {}) {
  return session.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)})`);
}

function result(session, command) {
  return session.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}).then(
    value => ({ ok: true, value }), error => ({ ok: false, error }))`);
}

const layout = { version: 2, mode: 'combined', items: [{ id: 'forest', kind: 'ecology', x: 34, y: 52 }],
  idle: { mode: 'text', text: '隔离恢复验证' } };

async function seedApp(sessions) {
  const panel = sessions.get('panel');
  const editor = sessions.get('editor');
  const initial = await invoke(editor, 'create_world', { presetId: 'pond', name: 'M4 存档恢复样本' });
  assert.match(initial.worldId, /^pond-[a-f\d]+$/);
  assert(initial.editPaused && initial.paused);
  const changed = await invoke(editor, 'edit_pond', {
    commandId: 'm4-seed-water', expectedGeneration: initial.runGeneration, expectedRevision: initial.editRevision,
    edit: { type: 'addWater', basinId: initial.pond.basins[0].id, liters: 0.8 },
  });
  const layoutResult = await invoke(panel, 'save_workspace_layout', { layout });
  assert.deepEqual(layoutResult.idle, layout.idle);
  await invoke(editor, 'save_world');
  assert.deepEqual(await checkpointFiles(data, initial.worldId), [name(1), name(2)]);
  await writeFile(join(evidence, 'seed.json'), JSON.stringify({
    worldId: initial.worldId,
    first: { revision: initial.editRevision, tick: initial.simulationTick, waterLiters: initial.pond.basins[0].waterLiters },
    last: { revision: changed.editRevision, tick: changed.simulationTick, waterLiters: changed.pond.basins[0].waterLiters },
    layout,
  }, null, 2), { flag: 'wx' });
  mark('seed-ready-before-forced-stop', { worldId: changed.worldId, checkpoints: 2, revision: changed.editRevision,
    tick: changed.simulationTick, waterLiters: changed.pond.basins[0].waterLiters });
}

async function recover(sessions) {
  const panel = sessions.get('panel');
  const editor = sessions.get('editor');
  const seed = await readJson(join(evidence, 'seed.json'));
  const healthy = healthyCases.includes(phase);
  const expectedState = ({ pending: 'ready', 'write-failure': 'ready', 'newest-corrupt': 'recovered', 'all-corrupt': 'unreadable',
    'future-format': 'unsupported', 'missing-definition': 'unsupported', locked: 'unreadable' })[phase];
  const overview = await invoke(editor, 'list_worlds');
  const row = overview.worlds.find(({ worldId }) => worldId === seed.worldId);
  assert.equal(overview.activeWorldId, seed.worldId);
  assert.equal(row.state, expectedState);
  const { snapshot } = await invoke(editor, 'subscribe_world', { viewId: 'editor' });
  const bootMessage = snapshot.lastSaveMessage;
  const visibleMessage = healthy ? phase === 'newest-corrupt' ? '已从上一有效检查点恢复' : '已恢复已保存的世界' : '现有世界无法读取';
  assert(bootMessage.includes(visibleMessage), JSON.stringify(bootMessage));
  await waitFor(() => editor.evaluate(`document.querySelector('.editor-footer')?.textContent.includes(${JSON.stringify(visibleMessage)})`),
    'Startup error/recovery message did not appear on the native editor');
  await editor.screenshot(join(evidence, phase, 'editor.png'));
  assert.deepEqual((await invoke(panel, 'get_workspace_layout')).idle, seed.layout.idle);
  if (healthy) {
    const expected = phase === 'newest-corrupt' ? seed.first : seed.last;
    assert.deepEqual({ revision: snapshot.editRevision, tick: snapshot.simulationTick,
      waterLiters: snapshot.pond.basins[0].waterLiters }, expected);
    assert(snapshot.editPaused && snapshot.paused);
    const filesBeforeSettings = await checkpointFiles(data, seed.worldId);
    const worldBeforeSettings = await recordHashes(data, seed.worldId, filesBeforeSettings);
    const layoutBeforeSave = digest(await readFile(layoutPath(data)));
    await invoke(panel, 'save_workspace_layout', { layout: { ...layout, items: [{ ...layout.items[0], x: 58 }] } });
    assert.notEqual(digest(await readFile(layoutPath(data))), layoutBeforeSave);
    assert.deepEqual(await recordHashes(data, seed.worldId, filesBeforeSettings), worldBeforeSettings);
    const layoutAfterSettings = digest(await readFile(layoutPath(data)));
    if (phase === 'write-failure') await faultAndCheckRetry(editor, seed.worldId, worldBeforeSettings);
    const saved = await result(editor, 'save_world');
    assert(saved.ok, JSON.stringify(saved));
    assert.equal(digest(await readFile(layoutPath(data))), layoutAfterSettings);
  } else {
    const saved = await result(editor, 'save_world');
    const restored = await result(editor, 'restore_world');
    assert.equal(saved.error?.code, 'save_blocked');
    assert.equal(restored.error?.code, ({ 'all-corrupt': 'checkpoint_unusable', 'future-format': 'checkpoint_unsupported',
      'missing-definition': 'checkpoint_unsupported', locked: 'world_operation_failed' })[phase]);
    assert.equal(snapshot.pond, undefined);
    if (phase === 'future-format') assert(bootMessage.includes('format version 2'));
    if (phase === 'missing-definition') assert(bootMessage.includes('pond-shallow/unavailable'));
    mark('original-preserved-during-refusal', { scenario: phase, saveError: saved.error.code, restoreError: restored.error.code });
  }
  mark('native-recovery', { scenario: phase, catalog: row.state, worldId: snapshot.worldId, revision: snapshot.editRevision,
    simulationTick: snapshot.simulationTick, waterLiters: snapshot.pond?.basins[0].waterLiters, visibleMessage });
  await editor.evaluate(`setTimeout(() => window.__TAURI_INTERNALS__.invoke('request_exit'), 50); true`);
}

async function faultAndCheckRetry(editor, id, expectedHashes) {
  assert.match(id, /^pond-[a-f\d]+$/);
  const target = directoryFor(data, id);
  const temporary = `${target}-test-held`;
  await rename(target, temporary);
  let obstructionOwned = false;
  try {
    await writeFile(target, 'isolated checkpoint write fault', { flag: 'wx' });
    obstructionOwned = true;
    const failed = await result(editor, 'save_world');
    assert.equal(failed.error?.code, 'save_failed');
    await waitFor(() => editor.evaluate(`document.querySelector('.editor-footer')?.textContent.includes('保存失败')`),
      'Checkpoint write error did not appear in the editor');
    await editor.screenshot(join(evidence, phase, 'save-error.png'));
    mark('native-scratch-write-failure', { error: failed.error.code, visible: true });
  } finally {
    if (obstructionOwned) await unlink(target);
    await rename(temporary, target);
  }
  assert.deepEqual(await recordHashes(data, id, await checkpointFiles(data, id)), expectedHashes);
}

async function seal() {
  for (const scenario of scenarios) {
    const base = join(evidence, scenario, 'data');
    const before = await readJson(join(evidence, scenario, 'before.json'));
    const files = await checkpointFiles(base, before.worldId);
    assert.deepEqual(await recordHashes(base, before.worldId, before.monitored), before.worldHashes);
    const trace = (await readFile(join(evidence, scenario, 'trace.jsonl'), 'utf8')).trim().split(/\r?\n/).map(JSON.parse);
    const exit = trace.find(({ event }) => event === 'exit-request-committed');
    assert(exit);
    const guarded = !healthyCases.includes(scenario);
    assert.equal(exit.details.skippedUnavailableActiveWorld, guarded);
    assert.deepEqual(guarded ? files : files.slice(0, 2), [name(1), name(2)]);
    const layoutUnchanged = digest(await readFile(layoutPath(base))) === before.layoutHash;
    assert.equal(layoutUnchanged, guarded);
    assert.equal(digest(await readFile(join(base, 'active-world.json'))), before.activeIndexHash);
    if (!guarded) assert.deepEqual(files, [name(1), name(2), name(3), name(4)]);
    mark('after-exit', { scenario, originalFilesByteIdentical: true, newCommitted: files.length - 2, layoutUnchanged });
  }
}

async function readTrace(file) {
  return (await readFile(file, 'utf8')).trimEnd().split(/\r?\n/).flatMap((line) => {
    try { return [JSON.parse(line)]; } catch { return []; }
  });
}

const viewContinuity = (snapshot) => ({ worldId: snapshot.worldId, paused: snapshot.paused,
  editPaused: snapshot.editPaused, simulationTick: snapshot.simulationTick, editRevision: snapshot.editRevision,
  daylightPhase: snapshot.daylightPhase, pond: snapshot.pond });

async function automaticSeed(editor) {
  const initial = await invoke(editor, 'create_world', { presetId: 'pond', name: 'M4 自动存档样本' });
  await invoke(editor, 'start_forest_trial');
  await invoke(editor, 'advance_forest_trial', { ticks: 30 });
  let current = await invoke(editor, 'adopt_trial');
  assert(current.editPaused && current.paused);
  assert.equal(current.simulationTick, '30');
  const id = current.worldId;
  const manualFiles = await checkpointFiles(data, id);
  const manualHashes = await recordHashes(data, id, manualFiles);
  const directory = directoryFor(data, id);
  const tracePath = join(evidence, 'auto', 'trace.jsonl');
  const filterCommits = (trace) => trace.filter(({ event, details }) =>
    event === 'automatic-checkpoint-saved' && details.affectedWorldId === id);
  for (let step = 0; step < 4; step += 1) {
    current = await invoke(editor, 'edit_pond', {
      commandId: `m4-auto-water-${step}`, expectedGeneration: current.runGeneration, expectedRevision: current.editRevision,
      edit: { type: 'addWater', basinId: current.pond.basins[0].id, liters: 0.1 },
    });
    await waitFor(async () => {
      const commits = filterCommits(await readTrace(tracePath));
      const latest = commits.at(-1);
      if (!latest) return false;
      const file = join(directory, `autosave-${String(latest.details.checkpointSerial).padStart(20, '0')}.json`);
      try { return String(JSON.parse((await readJson(file)).payload).editRevision) === current.editRevision; }
      catch { return false; }
    }, `Missing host-timer commit for automatic revision ${current.editRevision}`);
  }
  assert.deepEqual(await recordHashes(data, id, manualFiles), manualHashes);
  const autoFiles = (await readdir(directory)).filter((path) => /^autosave-\d{20}\.json$/.test(path)).sort();
  assert.equal(autoFiles.length, 2);
  const autoHashes = await recordHashes(data, id, autoFiles);
  const commits = filterCommits(await readTrace(tracePath));
  assert.equal(commits.length, 4);
  await writeFile(join(evidence, 'auto-expected.json'), JSON.stringify({
    continuity: viewContinuity(current), generation: current.runGeneration, manualFiles, manualHashes, autoFiles, autoHashes,
  }, null, 2), { flag: 'wx' });
  mark('native-timer-before-forced-stop', { automaticCommits: commits.map(({ details }) => details.checkpointSerial),
    retained: autoFiles, manualFiles, revision: current.editRevision, tick: current.simulationTick, daylight: current.daylightPhase });
}

async function automaticReopen(editor) {
  const expected = await readJson(join(evidence, 'auto-expected.json'));
  const { snapshot } = await invoke(editor, 'subscribe_world', { viewId: 'editor' });
  assert.deepEqual(viewContinuity(snapshot), expected.continuity);
  assert.notEqual(snapshot.runGeneration, expected.generation);
  assert.equal(snapshot.lastSaveMessage, '已从自动检查点恢复');
  await waitFor(() => editor.evaluate(`document.querySelector('.editor-footer')?.textContent.includes('已从自动检查点恢复')`),
    'Automatic checkpoint recovery message was missing from the editor');
  await editor.screenshot(join(evidence, phase, 'editor.png'));
  await editor.evaluate(`setTimeout(() => window.__TAURI_INTERNALS__.invoke('request_exit'), 50); true`);
  mark('native-reopen-from-autosave', { revision: snapshot.editRevision, tick: snapshot.simulationTick,
    daylight: snapshot.daylightPhase, editablePaused: snapshot.editPaused, exactObservedPondAndPhase: true });
}

async function automaticSeal() {
  const expected = await readJson(join(evidence, 'auto-expected.json'));
  const base = join(evidence, 'auto', 'data');
  const id = expected.continuity.worldId;
  assert.deepEqual(await recordHashes(base, id, [...expected.manualFiles, ...expected.autoFiles]),
    { ...expected.manualHashes, ...expected.autoHashes });
  const saved = await checkpointFiles(base, id);
  assert.equal(saved.length, expected.manualFiles.length + 1);
  const directory = directoryFor(base, id);
  const fileState = async (filename) => {
    const { payload } = await readJson(join(directory, filename));
    const parsed = JSON.parse(payload, (_field, value, context) =>
      typeof value === 'number' ? { exact: context.source } : value);
    // The external pending solar goal may be refreshed when the app reopens.
    // The current light phase and the rest of the persisted state must match.
    delete parsed.state.daylight.target;
    return parsed;
  };
  assert.deepEqual(await fileState(saved.at(-1)), await fileState(expected.autoFiles.at(-1)));
  const trace = await readTrace(join(evidence, 'auto-reopen', 'trace.jsonl'));
  assert(trace.some(({ event }) => event === 'exit-request-committed'));
  mark('autosave-after-exit', { originalFilesByteIdentical: true, reopenedGenerationChanged: true,
    retainedAutomatically: expected.autoFiles.length, persistedStateAndRngExact: true });
}

if (phase === 'prepare') await prepare();
else if (phase === 'seal') await seal();
else if (phase === 'seal-auto') await automaticSeal();
else {
  assert(['seed', 'auto', 'auto-reopen', ...scenarios].includes(phase));
  const sessions = await discover();
  try {
    await invoke(sessions.get('panel'), 'show_window', { viewId: 'editor' });
    if (phase === 'seed') await seedApp(sessions);
    else if (phase === 'auto') await automaticSeed(sessions.get('editor'));
    else if (phase === 'auto-reopen') await automaticReopen(sessions.get('editor'));
    else await recover(sessions);
  } finally {
    for (const session of sessions.values()) session.close();
  }
}
