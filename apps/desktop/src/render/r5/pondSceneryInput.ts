import { type KeyboardEvent, type PointerEvent, type RefObject, useEffect, useRef, useState } from 'react';
import type { PondScenery } from '../../world';
import { clamp, pointerOnSample, type Point, type Rect } from './geometry';
import { composedHeight, composedWidth, sceneryStyles, spriteLocal, type SpritePose } from './pondComposedArt';
import type { ComposedLayer, SceneryDraft } from './pondComposition';

export interface R5SceneryEdit {
  selectedId?: number | null;
  placement?: string;
  onSelect: (id: number) => void;
  onMove: (id: number, x: number, y: number) => void;
  onPlace: (x: number, y: number) => void;
}

interface Drag {
  pointer: number;
  revision: string;
  viewport: Rect;
  offset: Point;
  start: Point;
  item: PondScenery;
  parent?: SpritePose;
}

function normalized(at: Point, parent?: SpritePose): Point {
  return parent ? spriteLocal(parent, at) : { x: at.x / composedWidth, y: at.y / composedHeight };
}

function inBounds(point: Point, attached: boolean, style: string): Point {
  const min = attached ? 0 : -.125;
  const max = attached ? 1 : 1.125;
  return { x: clamp(point.x, min, max),
    y: sceneryStyles.get(style)?.rings && !attached ? clamp(point.y, 8 / composedHeight, 65 / composedHeight) : clamp(point.y, min, max) };
}

function sceneryDraft(point: Point, attached: boolean, style: string, id: number | null): SceneryDraft {
  const bed = sceneryStyles.get(style)?.bedAnchor;
  if (bed) {
    const allowed = !attached && point.x >= bed.x[0] && point.x <= bed.x[1] && point.y >= bed.y[0] && point.y <= bed.y[1];
    return { ...point, id, style, bedSite: allowed ? 'ground' : 'outside' };
  }
  return { ...inBounds(point, attached, style), id, style };
}

function pointerPosition(event: PointerEvent<HTMLCanvasElement>, view: Rect) {
  return pointerOnSample({ x: event.clientX, y: event.clientY }, event.currentTarget.getBoundingClientRect(), view);
}

export function useR5SceneryInput({
  edit, revision, canvas, scene, viewport, waterShift, wet, pick,
}: {
  edit?: R5SceneryEdit; revision: string; canvas: RefObject<HTMLCanvasElement | null>;
  scene: readonly PondScenery[]; viewport: Rect; waterShift: number; wet: boolean;
  pick: (point: Point) => ComposedLayer | undefined;
}) {
  const gesture = useRef<Drag | null>(null);
  const bedPress = useRef<{ pointer: number; revision: string; style: string; viewport: Rect } | null>(null);
  const [shown, setShown] = useState<{ revision: string; draft: SceneryDraft } | null>(null);
  const selected = scene.find(({ id }) => id === edit?.selectedId);
  const placement = edit?.placement;
  const drawing = Boolean(edit);
  const canPlace = Boolean(placement && (wet || !sceneryStyles.get(placement)?.rings));
  const bedPlacing = Boolean(placement && sceneryStyles.get(placement)?.bedAnchor);
  const currentDraft = drawing && shown?.revision === revision ? shown.draft : null;
  const draft = currentDraft && (currentDraft.id === null
    ? canPlace && currentDraft.style === placement
    : !placement && currentDraft.id === selected?.id && currentDraft.style === selected.style) ? currentDraft : null;

  function activeDrag() {
    const captured = gesture.current;
    return drawing && !placement && captured?.revision === revision && captured.item.id === selected?.id ? captured : null;
  }

  function setDraft(value: SceneryDraft | null) {
    setShown(value ? { revision, draft: value } : null);
  }

  function clear() {
    const pointer = gesture.current?.pointer ?? bedPress.current?.pointer;
    gesture.current = null;
    bedPress.current = null;
    setDraft(null);
    if (pointer !== undefined && canvas.current?.hasPointerCapture(pointer)) canvas.current.releasePointerCapture(pointer);
  }

  useEffect(() => {
    clear();
  }, [revision, placement, drawing, canPlace]);

  useEffect(() => {
    if (!gesture.current || gesture.current.item.id !== selected?.id) clear();
  }, [selected?.id]);

  function placing(at: Point) {
    if (!placement || !canPlace) return;
    const rings = sceneryStyles.get(placement)?.rings;
    // A floating leaf is placed on the water plane at the chosen horizontal position.
    const point = normalized(rings ? { x: at.x, y: at.y - waterShift } : at);
    return sceneryDraft(point, false, placement, null);
  }

  function start(event: PointerEvent<HTMLCanvasElement>) {
    if (!edit || event.button !== 0 || !event.isPrimary || gesture.current || bedPress.current) return false;
    const at = pointerPosition(event, viewport);
    if (placement) {
      const where = placing(at);
      if (where) {
        event.preventDefault();
        if (bedPlacing) {
          bedPress.current = { pointer: event.pointerId, revision, style: placement, viewport };
          setDraft(where);
          event.currentTarget.focus({ preventScroll: true });
          event.currentTarget.setPointerCapture(event.pointerId);
        } else {
          edit.onPlace(where.x, where.y);
        }
      }
      return true;
    }
    const layer = pick(at);
    if (layer?.pick.kind !== 'scenery') return false;
    const item = scene.find(({ id }) => id === layer.pick.id);
    if (!item) return false;
    const local = normalized(at, layer.parentPose);
    gesture.current = {
      pointer: event.pointerId, revision, viewport, start: at, item, parent: layer.parentPose,
      offset: { x: item.x - local.x, y: item.y - local.y },
    };
    setDraft(null);
    event.preventDefault();
    event.currentTarget.focus({ preventScroll: true });
    event.currentTarget.setPointerCapture(event.pointerId);
    edit.onSelect(item.id);
    return true;
  }

  function moved(event: PointerEvent<HTMLCanvasElement>, captured: Drag): SceneryDraft | undefined {
    const at = pointerPosition(event, captured.viewport);
    if (Math.hypot(captured.start.x - at.x, captured.start.y - at.y) < 1) return;
    const local = normalized(at, captured.parent);
    return sceneryDraft({ x: local.x + captured.offset.x, y: local.y + captured.offset.y },
      Boolean(captured.parent), captured.item.style, captured.item.id);
  }

  function move(event: PointerEvent<HTMLCanvasElement>) {
    if (!edit || !event.isPrimary) return false;
    if (placement) {
      const press = bedPress.current;
      if (press && press.pointer !== event.pointerId) return true;
      setDraft(placing(pointerPosition(event, press?.viewport ?? viewport)) ?? null);
      return true;
    }
    if (gesture.current?.pointer !== event.pointerId) return false;
    const drag = activeDrag();
    if (drag) setDraft(moved(event, drag) ?? null);
    return true;
  }

  function stop(event: PointerEvent<HTMLCanvasElement>, cancel = false) {
    const press = bedPress.current;
    if (press?.pointer === event.pointerId) {
      const next = !cancel && press.revision === revision && press.style === placement
        ? placing(pointerPosition(event, press.viewport)) : undefined;
      clear();
      if (next?.bedSite === 'ground') edit?.onPlace(next.x, next.y);
      return true;
    }
    if (gesture.current?.pointer !== event.pointerId) return false;
    const drag = activeDrag();
    const next = !cancel && drag ? moved(event, drag) : undefined;
    clear();
    if (next?.id != null && next.bedSite !== 'outside') edit?.onMove(next.id, next.x, next.y);
    return true;
  }

  function keyboard(event: KeyboardEvent<HTMLCanvasElement>) {
    if (!edit) return false;
    if (event.key === 'Escape') {
      const wasDragging = Boolean(gesture.current || bedPress.current);
      clear();
      if (wasDragging) {
        event.preventDefault();
        event.stopPropagation();
      }
      return wasDragging;
    }
    if (gesture.current || bedPress.current) return false;
    if (event.key === 'Enter' && draft) {
      event.preventDefault();
      if (draft.bedSite === 'outside') return true;
      if (draft.id === null) edit.onPlace(draft.x, draft.y);
      else edit.onMove(draft.id, draft.x, draft.y);
      clear();
      return true;
    }
    const shape = placement ? { x: .5, y: bedPlacing ? .85 : .76, id: null, style: placement, parent: null } : selected;
    const keys: Record<string, Point> = { ArrowLeft: { x: -1, y: 0 }, ArrowRight: { x: 1, y: 0 },
      ArrowUp: { x: 0, y: -1 }, ArrowDown: { x: 0, y: 1 } };
    const direction = keys[event.key];
    if (!shape || !direction || (placement && !canPlace)) return false;
    const step = event.shiftKey ? .02 : .004;
    const start = draft ?? shape;
    setDraft(sceneryDraft({ x: start.x + direction.x * step, y: start.y + direction.y * step },
      Boolean(shape.parent), shape.style, shape.id));
    event.preventDefault();
    return true;
  }

  const drag = activeDrag();
  return { draft, bedPlacing, latched: drag?.viewport ?? bedPress.current?.viewport, start, move, stop, keyboard,
    dragging: Boolean(drag && draft), leave: () => { if (placement && !bedPress.current) setDraft(null); } };
}
