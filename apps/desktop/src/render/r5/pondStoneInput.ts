import { type KeyboardEvent, type PointerEvent, type RefObject, useEffect, useRef, useState } from 'react';
import type { PondBasinSnapshot, PondStonePreset, PondWoodPreset } from '../../world';
import { type Point, type Rect, clamp, pointerOnSample } from './geometry';
import {
  type PondStoneDraft, type PondTerrainObject, type PondTerrainShape,
  bedLeft, bedPoint, bedWidth, bedY, pondTerrainObjects, stoneHint,
} from './pondStoneGeometry';
import type { PondMossDraft } from './pondMoss';

type Selectable = PondTerrainObject | { kind: 'plant'; id: number };

export type R5PondRockEdit = {
  mode: 'move' | 'add';
  addPreset?: PondStonePreset;
  addKind?: PondTerrainShape['kind'];
  woodPreset?: PondWoodPreset;
  selectedId?: number | null;
  selectedKind?: Selectable['kind'];
  mossDraft?: PondMossDraft;
  onSelect: (id: number | null, kind?: Selectable['kind']) => void;
  onMove: (id: number, x: number, kind: PondTerrainShape['kind']) => void;
  onAdd: (x: number) => void;
  onCancel?: () => void;
};

type Placement = PondStoneDraft & { revision: string };
type Gesture = PondTerrainShape & {
  id: number | null; pointer: number; startX: number; offset: number;
  view: Rect; revision: string;
};

function atPointer(event: PointerEvent<HTMLCanvasElement>, view: Rect) {
  const rect = event.currentTarget.getBoundingClientRect();
  const { clientX: x, clientY: y } = event;
  if (!rect.width || !rect.height || x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) return null;
  return pointerOnSample({ x, y }, rect, view);
}

function overBed(point: Point) {
  return point.x >= bedLeft && point.x <= bedLeft + bedWidth && point.y >= bedY - 37 && point.y <= bedY + 15;
}

export function usePondRockInput({
  pond, edit, revision, canvas, view, pick,
}: {
  pond: PondBasinSnapshot;
  edit?: R5PondRockEdit;
  revision: string;
  canvas: RefObject<HTMLCanvasElement | null>;
  view: Rect;
  pick: (at: Point) => Selectable | undefined;
}) {
  const gesture = useRef<Gesture | null>(null);
  const [draft, setDraft] = useState<Placement | null>(null);
  const mode = edit?.mode;
  const editingMoss = Boolean(edit?.mossDraft);
  const addition: PondTerrainShape | undefined = mode !== 'add' ? undefined : edit?.addKind === 'wood'
    ? { kind: 'wood', preset: edit.woodPreset ?? 'branch' } : { kind: 'stone', preset: edit?.addPreset ?? 'main' };
  const selectedKind = edit?.selectedKind ?? 'stone';
  const objects = pondTerrainObjects(pond);
  const selected = objects.find(({ id, kind }) => id === edit?.selectedId && kind === selectedKind);
  const current = gesture.current;
  const latched = !editingMoss && current?.revision === revision
    && (mode === 'add' ? current.id === null && current.kind === addition?.kind && current.preset === addition.preset
      : mode === 'move' && current.id !== null)
    ? current.view : undefined;
  const preview = !editingMoss && draft?.revision === revision
    && (mode === 'add' ? draft.id === null && draft.kind === addition?.kind && draft.preset === addition.preset
      : mode === 'move' && draft.id === selected?.id && draft.kind === selected.kind) ? draft : null;

  function clear() {
    const pointerId = gesture.current?.pointer;
    gesture.current = null;
    setDraft(null);
    if (pointerId !== undefined && canvas.current?.hasPointerCapture(pointerId)) canvas.current.releasePointerCapture(pointerId);
  }

  useEffect(() => { clear(); }, [mode, addition?.kind, addition?.preset, editingMoss, revision]);

  function onPointerDown(event: PointerEvent<HTMLCanvasElement>) {
    if (!edit || event.button !== 0 || !event.isPrimary || gesture.current) return;
    const at = atPointer(event, view);
    if (!at) return;
    const picked = mode === 'move' ? pick(at) : undefined;
    if (mode === 'move') edit.onSelect(picked?.id ?? null, picked?.kind);
    // Patches use an explicit carrier/offset editor. Canvas picking only selects them.
    if (editingMoss || picked?.kind === 'plant') return;
    const shape = picked ?? (addition && overBed(at) ? addition : undefined);
    if (!shape) return;

    const id = picked?.id ?? null;
    const offset = picked ? at.x - bedPoint(picked.x).x : 0;
    event.preventDefault();
    event.currentTarget.focus({ preventScroll: true });
    event.currentTarget.setPointerCapture(event.pointerId);
    gesture.current = { ...shape, id, pointer: event.pointerId, startX: at.x, offset, view, revision };
    if (id === null) setDraft({ ...shape, id, x: (at.x - bedLeft) / bedWidth, revision });
  }

  function position(event: PointerEvent<HTMLCanvasElement>, captured: Gesture): number | null {
    const at = atPointer(event, captured.view);
    if (!at || captured.revision !== revision) return null;
    if (captured.id === null) {
      if (mode !== 'add' || captured.kind !== addition?.kind || captured.preset !== addition.preset || !overBed(at)) return null;
    } else if (mode !== 'move' || Math.abs(at.x - captured.startX) < 1) return null;
    const x = (at.x - captured.offset - bedLeft) / bedWidth;
    return x >= 0 && x <= 1 ? x : null;
  }

  function onPointerMove(event: PointerEvent<HTMLCanvasElement>) {
    if (!edit || editingMoss || !event.isPrimary) return;
    const captured = gesture.current;
    if (captured) {
      if (captured.pointer !== event.pointerId) return;
      const x = position(event, captured);
      setDraft(x === null ? null : { ...captured, x });
    } else if (addition) {
      const at = atPointer(event, view);
      setDraft(at && overBed(at) ? { ...addition, id: null, x: (at.x - bedLeft) / bedWidth, revision } : null);
    }
  }

  function commit(placement: Placement) {
    if (!edit || editingMoss || placement.revision !== revision) return;
    if (placement.id === null && mode === 'add' && placement.kind === addition?.kind && placement.preset === addition.preset) edit.onAdd(placement.x);
    if (placement.id !== null && mode === 'move'
      && objects.some(({ id, kind, preset }) => id === placement.id && kind === placement.kind && preset === placement.preset)) {
      edit.onMove(placement.id, placement.x, placement.kind);
    }
  }

  function onPointerUp(event: PointerEvent<HTMLCanvasElement>) {
    const captured = gesture.current;
    if (captured?.pointer !== event.pointerId) return;
    const x = position(event, captured);
    clear();
    if (x !== null) commit({ ...captured, x });
  }

  function abort(event: PointerEvent<HTMLCanvasElement>) {
    if (gesture.current?.pointer === event.pointerId) clear();
  }

  function onKeyDown(event: KeyboardEvent<HTMLCanvasElement>) {
    if (!edit) return;
    if (event.key === 'Escape') {
      clear();
      edit.onCancel?.();
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    if (editingMoss || gesture.current) return;
    if (event.key === 'Enter' && preview) {
      commit(preview);
      clear();
      event.preventDefault();
      return;
    }
    const shape = addition ?? selected;
    if (!shape || (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight')) return;
    const step = (event.shiftKey ? .02 : .004) * (event.key === 'ArrowLeft' ? -1 : 1);
    const x = clamp((preview?.x ?? (mode === 'move' ? selected?.x : undefined) ?? .5) + step, 0, 1);
    setDraft({ ...shape, id: addition ? null : selected?.id ?? null, x, revision });
    event.preventDefault();
  }

  return {
    mode, preset: addition?.preset, addKind: addition?.kind, selected, preview, latched,
    hint: preview ? stoneHint(preview, pond) : undefined,
    dragging: Boolean(latched && preview?.id != null),
    handlers: {
      onPointerDown, onPointerMove, onPointerUp, onPointerCancel: abort, onLostPointerCapture: abort, onKeyDown,
      onPointerLeave: () => { if (addition && !gesture.current) setDraft(null); },
    },
  };
}
