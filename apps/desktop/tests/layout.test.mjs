import assert from 'node:assert/strict';
import test from 'node:test';
import {
  addEcologyPiece,
  applyEcologyBindings,
  bounds,
  DEFAULT_LAYOUT,
  dragPiece,
  ecologyAnchor,
  effectiveMode,
  LAYOUT_AREA,
  movePanel,
  pieceSize,
  pieceWorld,
  PIECES,
  raisePiece,
  readLayout,
  removePiece,
  resizePiece,
  setComposition,
  shiftToPanelOrigin,
  snapPlacement,
  togglePieceKind,
} from '../src/layout/layout.ts';

const piece = (layout, id) => layout.items.find((item) => item.id === id);
const spread = (layout) => layout.items.map(({ id, kind, x, y }) => ({ id, kind, x, y }));
const fits = (outer, inner) => (
  inner.x >= outer.x && inner.y >= outer.y
  && inner.x + inner.width <= outer.x + outer.width
  && inner.y + inner.height <= outer.y + outer.height
);

test('combined slots fit both auxiliary pieces inside the original enclosure', () => {
  const clockOnly = togglePieceKind(DEFAULT_LAYOUT, 'clock', 'clock');
  const layout = togglePieceKind(clockOnly, 'environment', 'environment');
  const shell = bounds(layout.items);
  assert.deepEqual(shell, bounds(DEFAULT_LAYOUT.items));
  const clock = piece(layout, 'clock');
  const environment = piece(layout, 'environment');
  assert.equal(clock.docked, true);
  assert.equal(environment.docked, true);
  assert.ok(fits(shell, { ...clock, ...pieceSize(clock) }));
  assert.ok(fits(shell, { ...environment, ...pieceSize(environment) }));
  assert.ok(environment.y >= clock.y + pieceSize(clock).height);
  assert.equal(togglePieceKind(DEFAULT_LAYOUT, 'ecology', 'unused'), DEFAULT_LAYOUT);
  assert.deepEqual(readLayout(JSON.parse(JSON.stringify(layout))), layout);
});

test('dragging and raising ecology carry its attached pieces while allowing external overlap', () => {
  const combined = togglePieceKind(DEFAULT_LAYOUT, 'clock', 'clock');
  const mixed = togglePieceKind(setComposition(combined, 'mixed'), 'environment', 'environment');
  const moved = dragPiece(mixed, 'forest', -80, -40).layout;
  const startScene = piece(mixed, 'forest');
  const movedScene = piece(moved, 'forest');
  assert.deepEqual({ x: movedScene.x, y: movedScene.y }, { x: 688, y: 416 });
  for (const id of ['forest', 'clock']) {
    assert.equal(piece(moved, id).x - movedScene.x, piece(mixed, id).x - startScene.x);
    assert.equal(piece(moved, id).y - movedScene.y, piece(mixed, id).y - startScene.y);
  }
  assert.equal(piece(moved, 'environment'), piece(mixed, 'environment'));
  assert.deepEqual(raisePiece(moved, 'forest').items.map(({ id }) => id), ['environment', 'forest', 'clock']);
  assert.deepEqual(mixed.items.map(({ id }) => id), ['forest', 'clock', 'environment']);
});

test('a nearby neighbor beats the lattice and snaps each moving axis to the previewed placement', () => {
  const layout = {
    ...DEFAULT_LAYOUT,
    mode: 'free',
    items: [
      { id: 'clock', kind: 'clock', x: 100, y: 100 },
      { id: 'environment', kind: 'environment', x: 323, y: 171 },
    ],
  };
  const result = dragPiece(layout, 'clock', 46, 74);
  assert.deepEqual(result.preview, result.layout);
  const clock = piece(result.layout, 'clock');
  assert.equal(clock.x + pieceSize(clock).width, 323);
  assert.equal(clock.y, 171);
  assert.equal(piece(result.layout, 'environment'), piece(layout, 'environment'));

  const farApart = { ...layout, items: [layout.items[0], { ...layout.items[1], y: 460 }] };
  const movedAlone = piece(dragPiece(farApart, 'clock', 46, 74).layout, 'clock');
  assert.deepEqual({ x: movedAlone.x, y: movedAlone.y }, { x: 144, y: 176 });
});

test('small lattice radius leaves open placement and does not change an untouched axis', () => {
  const layout = {
    ...DEFAULT_LAYOUT,
    items: [{ ...DEFAULT_LAYOUT.items[0], x: 100, y: 111 }],
  };
  assert.deepEqual(bounds(movePanel(layout, 6, 0).items), { x: 106, y: 111, ...pieceSize(layout.items[0]) });
  assert.deepEqual(bounds(movePanel(layout, 11, 0).items), { x: 112, y: 111, ...pieceSize(layout.items[0]) });
});

test('whole-panel snapping keeps the physical grid relative to the DPI-scaled work area', () => {
  const workArea = { x: -1730, y: 49, width: 1600, height: 850 };
  const panel = { x: -1587, y: 211, width: 480, height: 270 };
  assert.deepEqual(snapPlacement(panel, workArea, { x: 5, y: 5 }, [], 1.25), { x: -1590, y: 209 });
  assert.deepEqual(snapPlacement({ ...panel, x: -614 }, workArea, { x: 5, y: 0 }, [], 1.25), { x: -610, y: 211 });
  const combined = togglePieceKind(DEFAULT_LAYOUT, 'clock', 'clock');
  const moved = movePanel(combined, 9, 0);
  assert.equal(bounds(moved.items).x, 784);
  assert.equal(piece(moved, 'clock').x - piece(moved, 'forest').x, piece(combined, 'clock').x - piece(combined, 'forest').x);
});

test('switching from combined to free gives both cards a separate starting position', () => {
  const clock = togglePieceKind(DEFAULT_LAYOUT, 'clock', 'clock');
  const combined = togglePieceKind(clock, 'environment', 'environment');
  const free = setComposition(combined, 'free');
  const scene = piece(free, 'forest');
  assert.deepEqual(scene, piece(combined, 'forest'));
  const entities = free.items.filter((item) => item.kind !== 'ecology');
  for (const entity of entities) {
    const size = pieceSize(entity);
    assert.ok(entity.x + size.width <= scene.x || entity.x >= scene.x + PIECES.ecology.width
      || entity.y + size.height <= scene.y || entity.y >= scene.y + PIECES.ecology.height);
    assert.equal(entity.docked, undefined);
  }
  assert.deepEqual(readLayout(JSON.parse(JSON.stringify(free))), free);
});

test('mixed dragging shows a free ghost before attachment and allows a later detachment', () => {
  const free = togglePieceKind(setComposition(DEFAULT_LAYOUT, 'free'), 'clock', 'clock');
  const mixed = setComposition(free, 'mixed');
  const scene = piece(mixed, 'forest');
  const clock = piece(mixed, 'clock');
  const centerX = scene.x + PIECES.ecology.width / 2;
  const centerY = scene.y + PIECES.ecology.height / 2;
  const dx = centerX - clock.x - PIECES.clock.width / 2;
  const dy = centerY - clock.y - PIECES.clock.height / 2;
  const entered = dragPiece(mixed, 'clock', dx, dy);
  assert.equal(entered.portal, 'attach');
  assert.equal(piece(entered.preview, 'clock').docked, undefined);
  assert.equal(piece(entered.layout, 'clock').docked, true);
  assert.deepEqual(bounds(entered.layout.items), bounds([scene]));
  assert.equal(dragPiece(mixed, 'clock', dx, dy).layout.items[1].id, 'clock');

  const attached = entered.layout;
  const nearEdge = dragPiece(attached, 'clock', -(PIECES.ecology.width / 2 + 8), 0);
  assert.equal(nearEdge.portal, null, 'small edge excursions leave the component docked');
  const left = dragPiece(attached, 'clock', -320, 0);
  assert.equal(left.portal, 'detach');
  assert.equal(piece(left.layout, 'clock').docked, undefined);
  assert.ok(bounds(left.layout.items).width > PIECES.ecology.width);

  const far = dragPiece(left.layout, 'clock', -10_000, -10_000);
  assert.equal(far.portal, null);
  assert.deepEqual({ x: piece(far.layout, 'clock').x, y: piece(far.layout, 'clock').y }, { x: 0, y: 0 });
  assert.deepEqual(piece(far.layout, 'forest'), scene);
});

test('snapping a free ghost across the docking margin does not attach before the pointer does', () => {
  const layout = {
    ...DEFAULT_LAYOUT,
    mode: 'mixed',
    items: [
      { ...DEFAULT_LAYOUT.items[0], x: 198, y: 200 },
      { id: 'clock', kind: 'clock', x: 30, y: 225 },
    ],
  };
  const first = dragPiece(layout, 'clock', 97, 0);
  assert.equal(piece(first.preview, 'clock').x, 128);
  assert.equal(first.portal, null);
  const crossed = dragPiece(layout, 'clock', 98, 0);
  assert.equal(crossed.portal, 'attach');
  assert.equal(piece(crossed.preview, 'clock').x, piece(first.preview, 'clock').x);
});

test('moving a panel origin clips the group as a unit', () => {
  const combined = togglePieceKind(DEFAULT_LAYOUT, 'clock', 'clock');
  const moved = shiftToPanelOrigin(combined, 15, -25);
  const shell = bounds(moved.items);
  assert.deepEqual(shell, { x: 15, y: 0, width: 384, height: 216 });
  assert.equal(piece(moved, 'clock').x - piece(moved, 'forest').x, piece(combined, 'clock').x - piece(combined, 'forest').x);
  const clamped = bounds(shiftToPanelOrigin(combined, 10_000, 10_000).items);
  assert.equal(clamped.x + clamped.width, LAYOUT_AREA.width);
  assert.equal(clamped.y + clamped.height, LAYOUT_AREA.height);
});

test('V1 migration preserves piece identities, order, positions and idle preferences', () => {
  const oldLayout = {
    version: 1,
    items: [
      { id: 'clock-in', kind: 'clock', x: 280, y: 210 },
      { id: 'clock-out', kind: 'clock', x: 680, y: 310 },
      { id: 'forest', kind: 'ecology', x: 100, y: 200 },
      { id: 'secondary-scene', kind: 'ecology', x: 620, y: 225 },
    ],
    idle: { mode: 'text', text: '雨后林下' },
  };
  const migrated = readLayout(oldLayout);
  assert.equal(migrated.version, 2);
  assert.equal(migrated.mode, 'mixed');
  assert.deepEqual(spread(migrated), oldLayout.items);
  assert.deepEqual(migrated.idle, oldLayout.idle);
  assert.equal(piece(migrated, 'clock-in').docked, true);
  assert.equal(piece(migrated, 'clock-out').docked, undefined);
  assert.equal(ecologyAnchor(raisePiece(migrated, 'secondary-scene').items).id, 'forest');
  assert.deepEqual(togglePieceKind(migrated, 'clock', 'unused').items.map(({ kind }) => kind), ['ecology', 'ecology']);
  assert.deepEqual(readLayout(JSON.parse(JSON.stringify(migrated))), migrated);
});

test('removing ecology leaves other items usable and re-enabling respects stored composition', () => {
  const combined = togglePieceKind(DEFAULT_LAYOUT, 'clock', 'clock');
  const clockOnly = togglePieceKind(combined, 'ecology', 'unused');
  assert.equal(clockOnly.mode, 'combined');
  assert.equal(effectiveMode(clockOnly), 'free');
  assert.equal(clockOnly.items[0].docked, undefined);
  assert.deepEqual(readLayout(JSON.parse(JSON.stringify(clockOnly))), clockOnly);
  const restored = togglePieceKind(clockOnly, 'ecology', 'replacement');
  assert.equal(effectiveMode(restored), 'combined');
  assert.equal(piece(restored, 'clock').docked, true);
  const free = setComposition(restored, 'free');
  assert.equal(piece(free, 'clock').docked, undefined);
  assert.deepEqual(readLayout(JSON.parse(JSON.stringify(free))), free);
});

test('V2 rejects missing composition controls and impossible attachments', () => {
  assert.throws(() => readLayout({ ...DEFAULT_LAYOUT, mode: 'unrecognized' }));
  assert.throws(() => readLayout({ ...DEFAULT_LAYOUT, items: [{ id: 'clock', kind: 'clock', x: 20, y: 20, docked: true }] }));
  const valid = togglePieceKind(DEFAULT_LAYOUT, 'clock', 'clock');
  assert.throws(() => readLayout({ ...valid, items: [valid.items[0], { ...valid.items[1], x: 20 }] }));
  assert.throws(() => readLayout({ ...valid, items: [valid.items[0], { ...valid.items[1], docked: false }] }));
});

test('V2 remembers monitor-relative placement alongside world binding and legacy geometry', () => {
  const panelPlacement = { monitorName: 'Display 2', monitorX: -1600, monitorY: -200, horizontal: .25, vertical: .75 };
  const saved = { ...DEFAULT_LAYOUT, panelWorldId: 'pond-example', panelPlacement };
  assert.deepEqual(readLayout(JSON.parse(JSON.stringify(saved))), saved);
  const combined = togglePieceKind(saved, 'clock', 'clock');
  assert.deepEqual(readLayout(JSON.parse(JSON.stringify(combined))).panelPlacement, panelPlacement);
  for (const invalid of [{ ...panelPlacement, horizontal: 2 }, { ...panelPlacement, monitorX: 1.5 }]) {
    const recovered = readLayout({ ...saved, panelPlacement: invalid });
    assert.equal(recovered.panelPlacement, undefined);
    assert.equal(recovered.panelWorldId, saved.panelWorldId);
    assert.deepEqual(recovered.items, saved.items);
  }
});

test('each added ecology can follow its own world without changing the established dock host', () => {
  const initial = { ...DEFAULT_LAYOUT, panelWorldId: 'pond-legacy', items: [{ ...DEFAULT_LAYOUT.items[0], id: 'z-primary' }] };
  const withClock = togglePieceKind(initial, 'clock', 'clock');
  const added = addEcologyPiece(withClock, 'a-ecology');
  assert.equal(ecologyAnchor(added.items).id, 'z-primary');
  assert.equal(pieceWorld(piece(added, 'z-primary'), added.panelWorldId), 'pond-legacy');
  assert.equal(pieceWorld(piece(added, 'a-ecology'), added.panelWorldId), undefined);
  assert.equal(piece(added, 'a-ecology').docked, undefined);
  const linked = applyEcologyBindings(added, { 'a-ecology': 'forest-example' });
  assert.equal(pieceWorld(piece(readLayout(linked), 'a-ecology'), linked.panelWorldId), 'forest-example');
  const removed = readLayout(removePiece(linked, 'z-primary'));
  assert.equal(ecologyAnchor(removed.items).id, 'a-ecology');
  assert.equal(piece(removed, 'clock').docked, true);
});

test('individual resize keeps other free cards and world bindings intact, even when the host shrinks', () => {
  const layout = addEcologyPiece(togglePieceKind(DEFAULT_LAYOUT, 'environment', 'environment'), 'next');
  const largeDock = resizePiece(layout, 'environment', 175);
  const smallerHost = resizePiece(largeDock, 'forest', 75);
  assert.deepEqual(pieceSize(piece(smallerHost, 'forest')), { width: 288, height: 162 });
  assert.equal(piece(smallerHost, 'next'), piece(layout, 'next'));
  assert.equal(piece(smallerHost, 'environment').scalePercent, 175);
  assert.ok(fits({ ...piece(smallerHost, 'forest'), ...pieceSize(piece(smallerHost, 'forest')) },
    { ...piece(smallerHost, 'environment'), ...pieceSize(piece(smallerHost, 'environment')) }));
  const biggerCard = resizePiece(smallerHost, 'next', 1000);
  assert.deepEqual(pieceSize(piece(biggerCard, 'next')), { width: 672, height: 378 });
  assert.equal(piece(biggerCard, 'next').worldId, null);
  assert.deepEqual(readLayout(biggerCard), biggerCard);
});
