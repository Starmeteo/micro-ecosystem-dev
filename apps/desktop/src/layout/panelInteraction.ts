import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import type { PointerEvent } from 'react';
import { errorText, native } from '../world';
import { addEcologyPiece, bounds, dragPiece, movePanel, pieceSize, raisePiece, removePiece, resizePiece, setComposition, snapPlacement, togglePieceKind,
  type WorkspaceLayout, type WorkspacePiece } from './layout';
import { layoutAtPanel, pendingTrayActions, placeNativePanel, readPanelBounds, readPanelPointer,
  resizeAroundPanel, setPanelBounds, trayActionEvent, type PanelBounds, type useWorkspaceLayout } from './workspace';

type Workspace = ReturnType<typeof useWorkspaceLayout>;
type Portal = ReturnType<typeof dragPiece>['portal'];
type Operation = 'panel' | 'piece' | 'resize';
interface Preview { layout: WorkspaceLayout; pieceId: string; portal: Portal }
interface Gesture {
  pointerId: number;
  clientX: number;
  clientY: number;
  screenX: number;
  screenY: number;
  pieceId: string;
  operation: Operation;
  before: WorkspaceLayout;
  started: boolean;
  cancelled: boolean;
  dx: number;
  dy: number;
}

const geometryKey = (layout: WorkspaceLayout) => JSON.stringify([bounds(layout.items), layout.panelPlacement]);
const pause = () => new Promise<void>((resolve) => window.setTimeout(resolve, 20));

function sameScreen(a: PanelBounds, b: PanelBounds) {
  return a.monitorName === b.monitorName && a.monitorX === b.monitorX && a.monitorY === b.monitorY
    && a.scaleFactor === b.scaleFactor && a.workArea.x === b.workArea.x && a.workArea.y === b.workArea.y
    && a.workArea.width === b.workArea.width && a.workArea.height === b.workArea.height;
}

function fitsWorkArea(panel: PanelBounds) {
  const area = panel.workArea;
  return panel.x >= area.x && panel.y >= area.y && panel.x + panel.width <= area.x + area.width
    && panel.y + panel.height <= area.y + area.height;
}

function originalBounds(value: PanelBounds) {
  return { x: value.x, y: value.y, width: value.width / value.scaleFactor, height: value.height / value.scaleFactor };
}

async function moveNativePanel(origin: PanelBounds, current: PanelBounds, dx: number, dy: number) {
  const target = { x: origin.x + dx, y: origin.y + dy };
  const size = originalBounds(origin);
  function position(panel: PanelBounds) {
    const { workArea, scaleFactor } = panel;
    // Leave monitor handoff to the native window host before applying the new screen's lattice.
    if (target.x < workArea.x || target.y < workArea.y
      || target.x >= workArea.x + workArea.width || target.y >= workArea.y + workArea.height) return target;
    return snapPlacement({ ...target, width: panel.width, height: panel.height }, workArea, { x: dx, y: dy }, [], scaleFactor);
  }

  const first = position(current);
  const placed = await setPanelBounds({ ...size, ...first });
  if (sameScreen(placed, current)) return placed;
  const snapped = position(placed);
  return snapped.x === first.x && snapped.y === first.y ? placed : setPanelBounds({ ...size, ...snapped });
}

export function usePanelInteraction({ layout, latest, ready, save }: Workspace) {
  const [preview, setPreview] = useState<Preview | null>(null);
  const [dragging, setDragging] = useState<Operation | null>(null);
  const [notice, setNotice] = useState('');
  const [pulse, setPulse] = useState(0);
  const gesture = useRef<Gesture | null>(null);
  const busy = useRef(false);
  const applied = useRef('');
  const lastScreen = useRef<PanelBounds | null>(null);
  const kickTrayQueue = useRef<() => void>(() => {});

  function feedback(message: string) { setNotice(message); }

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(''), 3800);
    return () => window.clearTimeout(timer);
  }, [notice]);

  useEffect(() => {
    if (!ready || !native) return;
    let live = true;
    let requested = false;
    let unlisten: (() => void) | undefined;

    async function drain() {
      if (!live || busy.current || gesture.current || !requested) return;
      requested = false;
      busy.current = true;
      let timer: number | undefined;
      try {
        const actions = await pendingTrayActions();
        if (!live || !actions.length) return;
        const before = latest.current;
        let next = before;
        for (const action of actions) {
          const update = action.action === 'mode' ? setComposition(next, action.mode)
            : action.action === 'add-ecology' ? addEcologyPiece(next, crypto.randomUUID())
              : togglePieceKind(next, action.kind, crypto.randomUUID());
          if (update === next && action.action === 'toggle') feedback('当前组合无法切换这个组件。面板至少要保留一个组件。');
          if (update === next && action.action === 'add-ecology') feedback('面板已达到组件数量上限。');
          next = update;
        }
        if (next !== before) {
          const saved = await save(next, actions.at(-1)?.id);
          const expected = geometryKey(saved);
          if (applied.current !== expected) {
            const placed = await placeNativePanel(saved, () => live);
            if (placed) lastScreen.current = placed;
            applied.current = expected;
          }
          setPulse((value) => value + 1);
        } else {
          // The same native transaction also acknowledges a valid no-op.
          await save(before, actions.at(-1)?.id);
        }
      } catch (error) {
        if (live) feedback(errorText(error));
        timer = window.setTimeout(() => { if (live && requested) void drain(); }, 180);
      } finally {
        busy.current = false;
        if (live && requested && timer === undefined) void drain();
      }
    }

    function queue() { requested = true; void drain(); }
    kickTrayQueue.current = () => { void drain(); };
    void listen(trayActionEvent, queue, { target: 'panel' }).then((dispose) => {
      if (!live) { dispose(); return; }
      unlisten = dispose;
      queue();
    }).catch((error: unknown) => { if (live) feedback(errorText(error)); });

    return () => {
      live = false;
      kickTrayQueue.current = () => {};
      unlisten?.();
      if (gesture.current) gesture.current.cancelled = true;
    };
  }, [ready, latest]);

  const dimensions = geometryKey(layout);
  useEffect(() => {
    if (!native || !ready || dimensions === applied.current || gesture.current || busy.current) return;
    let live = true;
    busy.current = true;
    void placeNativePanel(latest.current, () => live).then((placed) => {
      if (placed && live) {
        applied.current = dimensions;
        lastScreen.current = placed;
      }
    }).catch((error: unknown) => { if (live) feedback(errorText(error)); }).finally(() => {
      busy.current = false;
      kickTrayQueue.current();
    });
    return () => { live = false; };
  }, [dimensions, latest, ready]);

  useEffect(() => {
    if (!native || !ready) return;
    let live = true;
    let reading = false;

    async function checkScreen() {
      if (reading || busy.current || gesture.current || document.hidden) return;
      reading = true;
      try {
        const current = await readPanelBounds();
        if (!live || busy.current || gesture.current) return;
        const previous = lastScreen.current;
        const next = latest.current;
        const key = geometryKey(next);
        if ((!previous || sameScreen(previous, current)) && fitsWorkArea(current) && applied.current === key) {
          lastScreen.current = current;
          return;
        }
        busy.current = true;
        try {
          const placed = await placeNativePanel(next, () => live);
          if (placed && live) {
            lastScreen.current = placed;
            applied.current = key;
          }
        } finally {
          busy.current = false;
          kickTrayQueue.current();
        }
      } catch (error) {
        if (live) feedback(errorText(error));
      } finally {
        reading = false;
      }
    }

    const timer = window.setInterval(() => void checkScreen(), 1800);
    const visible = () => { if (!document.hidden) void checkScreen(); };
    document.addEventListener('visibilitychange', visible);
    return () => {
      live = false;
      window.clearInterval(timer);
      document.removeEventListener('visibilitychange', visible);
    };
  }, [latest, ready]);

  async function savePosition(next: WorkspaceLayout, physical?: PanelBounds) {
    const positioned = physical ? layoutAtPanel(next, physical) : next;
    await save(positioned);
    if (physical) {
      applied.current = geometryKey(positioned);
      lastScreen.current = physical;
    }
  }

  function clear(current: Gesture) {
    if (gesture.current !== current) return;
    gesture.current = null;
    setPreview(null);
    setDragging(null);
    kickTrayQueue.current();
  }

  function markStarted(current: Gesture, dx: number, dy: number) {
    if (!current.started && Math.hypot(dx, dy) >= 4) {
      current.started = true;
      setDragging(current.operation);
    }
    return current.started;
  }

  function changePiece(current: Gesture, dx: number, dy: number): ReturnType<typeof dragPiece> {
    if (current.operation === 'piece') return dragPiece(current.before, current.pieceId, dx, dy);
    const piece = current.before.items.find(({ id }) => id === current.pieceId)!;
    const { width, height } = pieceSize(piece);
    const delta = (dx * width + dy * height) / (width * width + height * height);
    const next = resizePiece(current.before, piece.id, (piece.scalePercent ?? 100) * (1 + delta));
    return { layout: next, preview: next, portal: null };
  }

  async function nativeDrag(current: Gesture) {
    let starting: PanelBounds | undefined;
    try {
      const [first, origin] = await Promise.all([readPanelPointer(), readPanelBounds()]);
      starting = origin;
      const box = bounds(current.before.items);
      const fit = Math.min(1, origin.width / origin.scaleFactor / box.width, origin.height / origin.scaleFactor / box.height);
      const downX = origin.x + Math.round(current.clientX * origin.scaleFactor);
      const downY = origin.y + Math.round(current.clientY * origin.scaleFactor);
      let previous = { x: downX, y: downY };
      let pointer = first;
      let actual = origin;
      let result: ReturnType<typeof dragPiece> | undefined;
      while (!current.cancelled) {
        if (pointer.x !== previous.x || pointer.y !== previous.y) {
          const x = pointer.x - downX;
          const y = pointer.y - downY;
          if (markStarted(current, x / origin.scaleFactor, y / origin.scaleFactor)) {
            if (current.operation !== 'panel') {
              result = changePiece(current, x / origin.scaleFactor / fit, y / origin.scaleFactor / fit);
              actual = await setPanelBounds(resizeAroundPanel(result.preview, current.before, origin));
              setPreview({ layout: raisePiece(result.preview, current.pieceId), pieceId: current.pieceId, portal: result.portal });
            } else {
              actual = await moveNativePanel(origin, actual, x, y);
            }
          }
        }
        previous = pointer;
        if (!pointer.primaryDown) break;
        await pause();
        pointer = await readPanelPointer();
      }
      if (current.cancelled) {
        if (current.started) await setPanelBounds(originalBounds(origin));
      } else if (result) {
        const next = raisePiece(result.layout, current.pieceId);
        actual = await setPanelBounds(resizeAroundPanel(next, current.before, origin));
        setPreview({ layout: next, pieceId: current.pieceId, portal: result.portal });
        await savePosition(next, actual);
        if (result.portal) setPulse((value) => value + 1);
      } else if (current.started) {
        await savePosition(current.before, actual);
      } else {
        const raised = raisePiece(latest.current, current.pieceId);
        if (raised !== latest.current) await save(raised);
      }
    } catch (error) {
      if (starting && current.started) await setPanelBounds(originalBounds(starting)).catch(() => {});
      applied.current = geometryKey(latest.current);
      feedback(errorText(error));
    } finally {
      clear(current);
    }
  }

  function start(event: PointerEvent<HTMLElement>, piece: WorkspacePiece, operation: Operation) {
    if (!ready || busy.current || gesture.current || event.button !== 0 || !event.isPrimary) return false;
    const current = {
      pointerId: event.pointerId, clientX: event.clientX, clientY: event.clientY,
      screenX: event.screenX, screenY: event.screenY, pieceId: piece.id,
      operation: operation === 'piece' && latest.current.items.length < 2 ? 'panel' as const : operation, before: latest.current,
      started: false, cancelled: false, dx: 0, dy: 0,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    gesture.current = current;
    if (native) void nativeDrag(current);
    return true;
  }

  function browserDelta(current: Gesture, event: PointerEvent<HTMLElement>) {
    current.dx = event.screenX - current.screenX;
    current.dy = event.screenY - current.screenY;
    return markStarted(current, current.dx, current.dy);
  }

  function browserMove(event: PointerEvent<HTMLElement>) {
    const current = gesture.current;
    if (native || !current || event.pointerId !== current.pointerId || !browserDelta(current, event)) return;
    if (current.operation !== 'panel') {
      const result = changePiece(current, current.dx, current.dy);
      setPreview({ layout: raisePiece(result.preview, current.pieceId), pieceId: current.pieceId, portal: result.portal });
    } else {
      setPreview({ layout: movePanel(current.before, current.dx, current.dy), pieceId: '', portal: null });
    }
  }

  function browserEnd(event: PointerEvent<HTMLElement>) {
    const current = gesture.current;
    if (native || !current || event.pointerId !== current.pointerId) return;
    browserDelta(current, event);
    const next = !current.started ? raisePiece(current.before, current.pieceId)
      : current.operation !== 'panel' ? raisePiece(changePiece(current, current.dx, current.dy).layout, current.pieceId)
        : movePanel(current.before, current.dx, current.dy);
    void save(next).catch((error: unknown) => feedback(errorText(error))).finally(() => clear(current));
  }

  function cancel() {
    const current = gesture.current;
    if (!current) return;
    current.cancelled = true;
    if (!native) clear(current);
  }

  async function applyPieceChange(update: (layout: WorkspaceLayout) => WorkspaceLayout) {
    if (!ready || busy.current || gesture.current) return;
    const before = latest.current;
    const next = update(before);
    if (next === before) return;
    busy.current = true;
    let origin: PanelBounds | undefined;
    try {
      if (native) origin = await readPanelBounds();
      const placed = origin && await setPanelBounds(resizeAroundPanel(next, before, origin));
      await savePosition(next, placed);
      setPulse((value) => value + 1);
    } catch (error) {
      if (origin) await setPanelBounds(originalBounds(origin)).catch(() => {});
      feedback(errorText(error));
    } finally {
      busy.current = false;
      kickTrayQueue.current();
    }
  }

  function resizeBy(id: string, amount: number) {
    void applyPieceChange((before) => {
      const piece = before.items.find((item) => item.id === id);
      return piece ? resizePiece(before, id, (piece.scalePercent ?? 100) + amount) : before;
    });
  }

  function remove(id: string) { void applyPieceChange((before) => removePiece(before, id)); }

  return { preview, dragging, pulse, notice, start, browserMove, browserEnd, cancel, resizeBy, remove };
}
