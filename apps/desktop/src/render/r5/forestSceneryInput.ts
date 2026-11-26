import { type KeyboardEvent, type PointerEvent, type RefObject, useEffect, useRef, useState } from 'react';
import type { ForestScenery } from '../../world';
import { clamp, type Point, type Rect, pointerOnSample } from './geometry';
import type { R5ForestEdit } from './ForestLiveScene';
import type { ForestDraft } from './forestPlacement';
import type { ForestLayer, ForestSceneryDraft } from './forestComposition';
import { forestPosition } from './forestComposition';

export interface R5ForestSceneryEdit {
  selectedId?: number | null;
  placement?: string;
  onSelect: (id: number) => void;
  onMove: (id: number, x: number, y: number) => void;
  onPlace: (x: number, y: number) => void;
  onCancel?: () => void;
}

type Draft = { kind: 'terrain'; value: ForestDraft } | { kind: 'scenery'; value: ForestSceneryDraft };
type Gesture = {
  id: number;
  start: Point;
  viewport: Rect;
  draft: Draft;
  attached: boolean;
  flip: number;
  revision: string;
};

function pointOf(draft: Draft): Point {
  return draft.kind === 'terrain' ? draft.value.position : draft.value;
}

function moveDraft(draft: Draft, point: Point): Draft {
  return draft.kind === 'terrain' ? { kind: 'terrain', value: { ...draft.value, position: point } }
    : { kind: 'scenery', value: { ...draft.value, ...point } };
}

function bound(point: Point, kind: Draft['kind'], attached: boolean) {
  const min = kind === 'terrain' ? 0 : attached ? -.5 : -.25;
  const max = kind === 'terrain' ? 1 : attached ? .5 : 1.25;
  return { x: clamp(point.x, min, max), y: clamp(point.y, min, max) };
}

const steps: Record<string, Point> = {
  ArrowLeft: { x: -1, y: 0 }, ArrowRight: { x: 1, y: 0 }, ArrowUp: { x: 0, y: -1 }, ArrowDown: { x: 0, y: 1 },
};

export function useComposedForestInput({ terrain, scenery, revision, viewport, canvas, items, pick, selected, onMossSelect, onFernSelect }: {
  terrain?: R5ForestEdit;
  scenery?: R5ForestSceneryEdit;
  revision: string;
  viewport: Rect;
  canvas: RefObject<HTMLCanvasElement | null>;
  items: readonly ForestScenery[];
  pick: (point: Point) => ForestLayer | undefined;
  selected?: ForestLayer;
  onMossSelect?: (id: number) => void;
  onFernSelect?: (id: number) => void;
}) {
  const pointer = useRef<Gesture | null>(null);
  const [shown, setShown] = useState<{ revision: string; draft: Draft } | null>(null);
  const terrainMode = terrain?.mode;
  const kind = terrainMode === 'add' ? terrain?.addKind ?? 'rock' : undefined;
  const preset = kind === 'stonework' ? terrain?.stoneworkPreset ?? 'slab'
    : kind === 'barkShelter' ? terrain?.barkPreset ?? 'arch'
    : kind === 'leafPatch' ? terrain?.leafPreset ?? 'leaves' : undefined;
  const style = scenery?.placement;
  const enabled = Boolean(terrain || scenery);
  const current = shown?.revision === revision && enabled ? shown.draft : null;
  const selectedId = current?.kind === 'terrain' ? terrain?.selectedId : scenery?.selectedId;
  const draft = current && (current.value.id === null || current.value.id === selectedId) ? current : null;

  function setDraft(draft: Draft | null) {
    setShown(draft ? { revision, draft } : null);
  }

  function clear() {
    const id = pointer.current?.id;
    pointer.current = null;
    setDraft(null);
    if (id !== undefined && canvas.current?.hasPointerCapture(id)) canvas.current.releasePointerCapture(id);
  }

  useEffect(() => {
    clear();
  }, [revision, kind, preset, style, terrainMode, enabled]);

  useEffect(() => {
    if (!pointer.current) setShown(null);
  }, [terrain?.selectedId, scenery?.selectedId]);

  function at(event: PointerEvent<HTMLCanvasElement>, view: Rect) {
    const rect = event.currentTarget.getBoundingClientRect();
    return pointerOnSample({ x: event.clientX, y: event.clientY }, rect, view);
  }

  function overCanvas(event: PointerEvent<HTMLCanvasElement>) {
    const { left, right, top, bottom } = event.currentTarget.getBoundingClientRect();
    return event.clientX >= left && event.clientX <= right && event.clientY >= top && event.clientY <= bottom;
  }

  function placement(point: Point): Draft | undefined {
    if (kind) return { kind: 'terrain', value: { kind, preset, position: point, id: null, mirrored: false } };
    if (style) return { kind: 'scenery', value: { ...point, style, id: null } };
  }

  function existing(layer?: ForestLayer): Draft | undefined {
    if (layer?.key?.kind === 'terrain' && layer.terrain && terrain) {
      return { kind: 'terrain', value: layer.terrain };
    }
    if (layer?.key?.kind === 'scenery' && scenery) {
      const item = items.find(({ id }) => id === layer.key?.id);
      if (item) return { kind: 'scenery', value: item };
    }
  }

  function press(event: PointerEvent<HTMLCanvasElement>) {
    if (!enabled || pointer.current || event.button !== 0 || !event.isPrimary) return;
    const point = at(event, viewport);
    const under = kind || style ? undefined : pick(point);
    if (under?.key?.kind === 'moss' && onMossSelect) {
      event.preventDefault();
      onMossSelect(under.key.id);
      return;
    }
    if (under?.key?.kind === 'fern' && onFernSelect) {
      event.preventDefault();
      onFernSelect(under.key.id);
      return;
    }
    const planned = placement(forestPosition(point));
    const change = planned ?? existing(under);
    if (!change) {
      terrain?.onSelect(null);
      return;
    }
    event.preventDefault();
    event.currentTarget.focus({ preventScroll: true });
    event.currentTarget.setPointerCapture(event.pointerId);
    pointer.current = { id: event.pointerId, start: point, viewport, draft: change,
      attached: Boolean(under?.parent), flip: under?.parentMirror ?? 1, revision };
    if (planned) setDraft(planned);
    else if (change.kind === 'terrain') terrain?.onSelect(change.value.id);
    else if (change.value.id != null) scenery?.onSelect(change.value.id);
  }

  function positionDuring(event: PointerEvent<HTMLCanvasElement>, state: Gesture) {
    const point = at(event, state.viewport);
    const { draft, start } = state;
    const next = draft.value.id === null ? forestPosition(point) : (() => {
      const delta = forestPosition({ x: point.x - start.x, y: point.y - start.y });
      const anchor = pointOf(draft);
      return { x: anchor.x + delta.x * state.flip, y: anchor.y + delta.y };
    })();
    return moveDraft(draft, bound(next, draft.kind, state.attached));
  }

  function move(event: PointerEvent<HTMLCanvasElement>) {
    if (!enabled || !event.isPrimary) return;
    const state = pointer.current;
    if (state?.id === event.pointerId && state.revision === revision) {
      if (state.draft.kind === 'terrain' && !overCanvas(event)) {
        setDraft(null);
        return;
      }
      const point = at(event, state.viewport);
      const moved = Math.hypot(point.x - state.start.x, point.y - state.start.y) >= 1;
      setDraft(moved || state.draft.value.id === null ? positionDuring(event, state) : null);
    } else if (!state) {
      setDraft(placement(forestPosition(at(event, viewport))) ?? null);
    }
  }

  function commit(next: Draft) {
    if (next.kind === 'terrain') {
      const { id, position } = next.value;
      if (id === null) terrain?.onAdd(position);
      else terrain?.onMove(id, position);
    } else {
      const { id, x, y } = next.value;
      if (id === null) scenery?.onPlace(x, y);
      else scenery?.onMove(id, x, y);
    }
  }

  function release(event: PointerEvent<HTMLCanvasElement>) {
    const state = pointer.current;
    if (state?.id !== event.pointerId) return;
    const point = at(event, state.viewport);
    const didMove = Math.hypot(point.x - state.start.x, point.y - state.start.y) >= 1;
    clear();
    if (!enabled || state.revision !== revision || (!didMove && state.draft.value.id !== null)) return;
    if (state.draft.kind === 'terrain' && !overCanvas(event)) return;
    commit(positionDuring(event, state));
  }

  function abort(event: PointerEvent<HTMLCanvasElement>) {
    if (pointer.current?.id === event.pointerId) clear();
  }

  function keyboard(event: KeyboardEvent<HTMLCanvasElement>) {
    if (!enabled) return;
    if (event.key === 'Escape') {
      clear();
      terrain?.onCancel?.();
      scenery?.onCancel?.();
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    if (pointer.current) return;
    if (event.key === 'Enter' && draft) {
      commit(draft);
      clear();
      event.preventDefault();
      return;
    }
    const direction = steps[event.key];
    if (!direction) return;
    const chosen = draft ?? placement({ x: .5, y: .74 }) ?? existing(selected);
    if (!chosen) return;
    const start = pointOf(chosen);
    const step = event.shiftKey ? .02 : .004;
    const next = { x: start.x + direction.x * step * (selected?.parentMirror ?? 1), y: start.y + direction.y * step };
    setDraft(moveDraft(chosen, bound(next, chosen.kind, Boolean(selected?.parent) && chosen.value.id !== null)));
    event.preventDefault();
  }

  return {
    terrainDraft: draft?.kind === 'terrain' ? draft.value : null,
    sceneryDraft: draft?.kind === 'scenery' ? draft.value : null,
    placementKind: style ? 'scenery' : kind,
    placementPreset: style ?? preset,
    placementMode: style || kind ? 'add' : enabled ? 'move' : undefined,
    latched: pointer.current?.revision === revision ? pointer.current.viewport : undefined,
    dragging: Boolean(pointer.current && draft?.value.id != null),
    handlers: {
      onPointerDown: press,
      onPointerMove: move,
      onPointerUp: release,
      onPointerCancel: abort,
      onLostPointerCapture: abort,
      onPointerLeave: () => { if (!pointer.current && (style || kind)) setDraft(null); },
      onKeyDown: keyboard,
    },
  };
}
