import { type KeyboardEvent, type PointerEvent, type RefObject, useEffect, useRef, useState } from 'react';
import type { PondBasinSnapshot, PondRibbonPlantPreset } from '../../world';
import { clamp, pointerOnSample, type Point, type Rect } from './geometry';
import { bedLeft, bedWidth, bedY } from './pondStoneGeometry';
import { type PondWaterPlantDraft, waterPlantRange, waterPlantSite } from './pondWaterPlant';

export interface R5PondWaterPlantEdit {
  selectedId?: number;
  placing?: boolean;
  ribbonPreset?: PondRibbonPlantPreset;
  onSelect: (id: number) => void;
  onAdd: (x: number) => void;
  onMove: (id: number, x: number) => void;
  onCancel: () => void;
}

type Gesture = { id: number; pointer: number; revision: string; view: Rect; startX: number; originalX: number };

function pointerAt(event: PointerEvent<HTMLCanvasElement>, viewport: Rect) {
  const box = event.currentTarget.getBoundingClientRect();
  const { clientX: x, clientY: y } = event;
  if (!box.width || !box.height || x < box.left || x > box.right || y < box.top || y > box.bottom) return;
  return pointerOnSample({ x, y }, box, viewport);
}

export function usePondWaterPlantInput({ edit, pond, revision, canvas, view, pick }: {
  edit?: R5PondWaterPlantEdit;
  pond: PondBasinSnapshot;
  revision: string;
  canvas: RefObject<HTMLCanvasElement | null>;
  view: Rect;
  pick: (point: Point) => number | undefined;
}) {
  const gesture = useRef<Gesture | null>(null);
  const [shown, setShown] = useState<{ revision: string; point: PondWaterPlantDraft } | null>(null);
  const selected = pond.waterPlantPatches?.find(({ id }) => id === edit?.selectedId);
  const placing = Boolean(edit?.placing && pond.wet);
  const enabled = Boolean(edit);
  const candidate = enabled && shown?.revision === revision ? shown.point : null;
  const draft = candidate && (candidate.id === null ? placing : !placing && selected?.id === candidate.id)
    ? { ...candidate, ribbonPreset: candidate.id === null ? edit?.ribbonPreset : selected?.ribbonPreset ?? undefined } : null;

  function activeGesture() {
    const current = gesture.current;
    return enabled && !placing && current?.revision === revision && current.id === selected?.id ? current : null;
  }

  function preview(point: PondWaterPlantDraft | null) {
    setShown(point ? { point, revision } : null);
  }

  function clear() {
    const pointer = gesture.current?.pointer;
    gesture.current = null;
    preview(null);
    if (pointer !== undefined && canvas.current?.hasPointerCapture(pointer)) canvas.current.releasePointerCapture(pointer);
  }

  useEffect(() => { clear(); }, [revision, enabled, placing]);
  useEffect(() => {
    if (!gesture.current || gesture.current.id !== selected?.id) clear();
  }, [selected?.id]);

  function placement(point?: Point): PondWaterPlantDraft | null {
    if (!point || point.y < bedY - 37 || point.y > bedY + 15) return null;
    return { id: null, x: (point.x - bedLeft) / bedWidth };
  }

  function commit(point: PondWaterPlantDraft) {
    if (!edit || waterPlantSite(point, pond) !== 'ground') return false;
    if (point.id === null) edit.onAdd(point.x);
    else if (point.x !== selected?.x) edit.onMove(point.id, point.x);
    return true;
  }

  function start(event: PointerEvent<HTMLCanvasElement>) {
    if (!edit || event.button !== 0 || !event.isPrimary || gesture.current) return false;
    const point = pointerAt(event, view);
    if (placing) {
      const planned = placement(point);
      if (planned) {
        preview(planned);
        if (commit(planned)) clear();
      }
      return true;
    }
    if (!point) return false;
    const id = pick(point);
    const target = pond.waterPlantPatches?.find((item) => item.id === id);
    if (!target) return false;
    event.preventDefault();
    event.currentTarget.focus({ preventScroll: true });
    event.currentTarget.setPointerCapture(event.pointerId);
    gesture.current = { id: target.id, pointer: event.pointerId, revision, view, startX: point.x, originalX: target.x };
    preview(null);
    edit.onSelect(target.id);
    return true;
  }

  function dragged(event: PointerEvent<HTMLCanvasElement>, current: Gesture): PondWaterPlantDraft | null {
    const point = pointerAt(event, current.view);
    if (!point || Math.abs(point.x - current.startX) < 1) return null;
    return { id: current.id, x: current.originalX + (point.x - current.startX) / bedWidth };
  }

  function move(event: PointerEvent<HTMLCanvasElement>) {
    if (!edit || !event.isPrimary) return false;
    if (placing) { preview(placement(pointerAt(event, view))); return true; }
    if (gesture.current?.pointer !== event.pointerId) return false;
    const current = activeGesture();
    if (current) preview(dragged(event, current));
    return true;
  }

  function stop(event: PointerEvent<HTMLCanvasElement>, cancel = false) {
    if (gesture.current?.pointer !== event.pointerId) return false;
    const current = activeGesture();
    const point = !cancel && current ? dragged(event, current) : null;
    clear();
    if (point) commit(point);
    return true;
  }

  function keyboard(event: KeyboardEvent<HTMLCanvasElement>) {
    if (!edit) return false;
    if (event.key === 'Escape' && (placing || draft || gesture.current)) {
      clear();
      edit.onCancel();
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (gesture.current) return false;
    if (event.key === 'Enter' && draft) {
      if (commit(draft)) clear();
      event.preventDefault();
      return true;
    }
    if ((!placing && !selected) || (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight')) return false;
    const step = (event.shiftKey ? .02 : .004) * (event.key === 'ArrowLeft' ? -1 : 1);
    preview({ id: placing ? null : selected?.id ?? null,
      x: clamp((draft?.x ?? selected?.x ?? .5) + step, ...waterPlantRange) });
    event.preventDefault();
    return true;
  }

  return { draft, site: draft ? waterPlantSite(draft, pond) : undefined, latched: activeGesture()?.view,
    dragging: Boolean(activeGesture() && draft), placing, start, move, stop, keyboard,
    leave: () => { if (placing) preview(null); },
  };
}
