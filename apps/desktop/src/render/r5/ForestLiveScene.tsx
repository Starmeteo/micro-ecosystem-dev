import { type KeyboardEvent, type PointerEvent, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { BarkShelterPreset, ForestTerrainKind, LeafPatchPreset, ScenePoint, StoneworkPreset, WorldSnapshot } from '../../world';
import {
  R5_HEIGHT, R5_WIDTH, type Point, type R5SampleFocus, type R5SampleZoom, type Rect,
  clamp, insideScene, pointerOnSample, sampleViewport, sceneNormalised, scenePixels,
} from './geometry';
import { type SampleImages, loadSampleArt } from './sceneArt';
import { drawForest, forestAxis, forestObjectAt, forestObjects } from './drawForest';
import { type ForestLandscape, loadForestLandscape } from './forestLandscape';
import { type ForestDraft, type ForestShape, forestArt, forestPlacementHint, leafPatchFrame } from './forestPlacement';
import { drawRuler, presentScene } from './drawSample';
import { useCanvasFit } from './useCanvasFit';
import type { ForestSpringtailView } from './forestSpringtails';
import './sample.css';

export type R5ForestEdit = {
  mode: 'move' | 'add';
  addKind?: ForestTerrainKind;
  stoneworkPreset?: StoneworkPreset;
  barkPreset?: BarkShelterPreset;
  leafPreset?: LeafPatchPreset;
  selectedId?: number | null;
  onSelect: (id: number | null) => void;
  onMove: (id: number, position: ScenePoint) => void;
  onAdd: (position: ScenePoint) => void;
  onCancel?: () => void;
};

export type R5ForestView = { zoom?: R5SampleZoom; focus?: R5SampleFocus; showScale?: boolean; springtails?: ForestSpringtailView; animalId?: string };

type ForestLiveSceneProps = {
  snapshot: WorldSnapshot;
  active?: boolean;
  forestEdit?: R5ForestEdit;
  forestView?: R5ForestView;
  onReady?: (backend: string) => void;
};

type InputDraft = ForestDraft & { editRevision: string };
type LoadedForestArt = { images: SampleImages; landscape?: ForestLandscape };
type PointerGesture = ForestShape & {
  pointerId: number;
  mode: R5ForestEdit['mode'];
  id: number | null;
  mirrored: boolean;
  start: Point;
  offset: Point;
  viewport: Rect;
  editRevision: string;
};

const center = { x: R5_WIDTH / 2, y: R5_HEIGHT / 2 };

function atPointer(event: PointerEvent<HTMLCanvasElement>, viewport: Rect): Point | null {
  const { clientX: x, clientY: y } = event;
  const rect = event.currentTarget.getBoundingClientRect();
  if (!rect.width || !rect.height || x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) return null;
  return pointerOnSample({ x, y }, rect, viewport);
}

export function ForestLiveScene({
  snapshot, active = true, forestEdit, forestView, onReady,
}: ForestLiveSceneProps) {
  const host = useRef<HTMLDivElement>(null);
  const output = useRef<HTMLCanvasElement>(null);
  const logical = useRef<HTMLCanvasElement | null>(null);
  const pointer = useRef<PointerGesture | null>(null);
  const backendReady = useRef(onReady);
  const [loadedArt, setLoadedArt] = useState<LoadedForestArt | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);
  const [draft, setDraft] = useState<InputDraft | null>(null);
  const fit = useCanvasFit(host);
  const editable = active ? forestEdit : undefined;
  const mode = editable?.mode;
  const addKind = mode === 'add' ? editable?.addKind ?? 'rock' : undefined;
  const addPreset = addKind === 'stonework' ? editable?.stoneworkPreset ?? 'slab'
    : addKind === 'barkShelter' ? editable?.barkPreset ?? 'arch'
      : addKind === 'leafPatch' ? editable?.leafPreset ?? 'leaves' : undefined;
  const { editRevision } = snapshot;
  const objects = forestObjects(snapshot);
  const selected = objects.find((entry) => entry.id === editable?.selectedId);
  const objectFocus = selected ?? objects[0];
  const hasLeafPatches = Array.isArray(snapshot.forestRocks?.leafPatches);
  const art = loadedArt && Boolean(loadedArt.landscape) === hasLeafPatches ? loadedArt : null;
  const images = art?.images;
  const landscape = art?.landscape;
  const preciseHit = Array.isArray(snapshot.forestRocks?.logs) || Array.isArray(snapshot.forestRocks?.stonework)
    || Array.isArray(snapshot.forestRocks?.barkShelters) || hasLeafPatches;
  const focus = forestView?.focus;
  const subject = focus === 'contact' ? objectFocus?.position : focus === 'animal' ? snapshot.animals[0] : undefined;
  const zoom = mode === 'add' ? 1 : forestView?.zoom ?? 1;
  const samplingWidth = hasLeafPatches ? clamp(Math.ceil(fit.pixelWidth * zoom / 16) * 16, R5_WIDTH, R5_WIDTH * 2) : R5_WIDTH;
  const gesture = pointer.current;
  const latchedView = gesture?.editRevision === editRevision && gesture.mode === mode
    && (!addKind || (gesture.kind === addKind && gesture.preset === addPreset)) ? gesture.viewport : undefined;
  const viewport = latchedView ?? sampleViewport(zoom, subject ? scenePixels(subject) : center);
  const shownDraft = editable && draft?.editRevision === editRevision
    && (mode === 'add' ? draft.id === null && draft.kind === addKind && draft.preset === addPreset
      : draft.id === selected?.id && draft.kind === selected.kind && draft.preset === selected.preset) ? draft : null;
  const preview = shownDraft ? forestPlacementHint(shownDraft, objects, snapshot.animals) : null;
  const dragging = Boolean(shownDraft && shownDraft.id !== null && gesture);

  useEffect(() => { backendReady.current = onReady; }, [onReady]);
  useEffect(() => {
    let live = true;
    setLoadFailed(false);
    void loadSampleArt('forest').then(async (images) => {
      const landscape = hasLeafPatches ? await loadForestLandscape(images) : undefined;
      if (!live) return;
      setLoadedArt({ images, landscape });
      backendReady.current?.('canvas2d-r5');
    }).catch(() => {
      if (!live) return;
      setLoadFailed(true);
      backendReady.current?.('unavailable');
    });
    return () => { live = false; };
  }, [hasLeafPatches]);

  function clearPointer() {
    const activePointer = pointer.current?.pointerId;
    pointer.current = null;
    setDraft(null);
    const canvas = output.current;
    if (activePointer !== undefined && canvas?.hasPointerCapture(activePointer)) canvas.releasePointerCapture(activePointer);
  }

  useEffect(() => { clearPointer(); }, [mode, addKind, addPreset, editRevision]);

  useLayoutEffect(() => {
    const canvas = output.current;
    if (!active || !canvas || !images || !fit.cssWidth || !fit.cssHeight) return;
    logical.current ??= document.createElement('canvas');
    if (logical.current.width !== samplingWidth) {
      logical.current.width = samplingWidth;
      logical.current.height = samplingWidth / R5_WIDTH * R5_HEIGHT;
    }
    drawForest(logical.current, images, snapshot, objects, selected?.id ?? null, shownDraft, preview, landscape);
    const ctx = presentScene(canvas, logical.current, viewport);
    if (ctx && forestView?.showScale) {
      drawRuler(ctx, viewport, canvas, forestArt.referenceWidthMm, fit.devicePixelRatio, forestAxis(snapshot));
    }
  });

  function press(event: PointerEvent<HTMLCanvasElement>) {
    if (!editable || !images || pointer.current || event.button !== 0 || !event.isPrimary) return;
    const at = atPointer(event, viewport);
    if (!at) return;
    const selectedObject = mode === 'move' ? forestObjectAt(objects, at, images, preciseHit, !hasLeafPatches, Boolean(landscape)) : undefined;
    if (mode === 'move') {
      editable.onSelect(selectedObject?.id ?? null);
      if (!selectedObject) return;
    }
    const kind = selectedObject?.kind ?? addKind;
    if (!kind) return;
    const preset = selectedObject?.preset ?? addPreset;
    event.preventDefault();
    event.currentTarget.focus({ preventScroll: true });
    event.currentTarget.setPointerCapture(event.pointerId);
    const anchor = selectedObject ? scenePixels(selectedObject.position) : at;
    const { mode: action } = editable;
    pointer.current = {
      mode: action, pointerId: event.pointerId, kind, preset, id: selectedObject?.id ?? null, mirrored: selectedObject?.mirrored ?? false,
      start: at, offset: { x: at.x - anchor.x, y: at.y - anchor.y }, viewport, editRevision,
    };
    if (action === 'add') setDraft({ kind, preset, id: null, position: sceneNormalised(at), mirrored: false, editRevision });
  }

  function dragPosition(event: PointerEvent<HTMLCanvasElement>, current: PointerGesture) {
    const at = atPointer(event, current.viewport);
    if (!at || current.mode !== mode || (addKind && (current.kind !== addKind || current.preset !== addPreset))
      || current.editRevision !== editRevision
      || (current.id !== null && Math.hypot(at.x - current.start.x, at.y - current.start.y) < 1)) return null;
    const anchor = { x: at.x - current.offset.x, y: at.y - current.offset.y };
    return insideScene(anchor) ? sceneNormalised(anchor) : null;
  }

  function move(event: PointerEvent<HTMLCanvasElement>) {
    if (!editable || !images || !event.isPrimary) return;
    const current = pointer.current;
    if (current) {
      if (current.pointerId !== event.pointerId) return;
      const at = dragPosition(event, current);
      setDraft(at ? {
        kind: current.kind, preset: current.preset, id: current.id, position: at, mirrored: current.mirrored, editRevision,
      } : null);
    } else if (addKind) {
      const at = atPointer(event, viewport);
      setDraft(at ? { kind: addKind, preset: addPreset, id: null, position: sceneNormalised(at), mirrored: false, editRevision } : null);
    }
  }

  function commit(current: ForestDraft) {
    if (!editable) return;
    if (current.id === null && current.kind === addKind && current.preset === addPreset) editable.onAdd(current.position);
    if (current.id !== null && mode === 'move') editable.onMove(current.id, current.position);
  }

  function release(event: PointerEvent<HTMLCanvasElement>) {
    const current = pointer.current;
    if (!current || current.pointerId !== event.pointerId) return;
    const position = dragPosition(event, current);
    clearPointer();
    if (position && (current.id === null
      || objects.some(({ id, kind, preset }) => id === current.id && kind === current.kind && preset === current.preset))) {
      commit({ ...current, position });
    }
  }

  function abort(event: PointerEvent<HTMLCanvasElement>) {
    if (pointer.current?.pointerId === event.pointerId) clearPointer();
  }

  function keyboard(event: KeyboardEvent<HTMLCanvasElement>) {
    if (!editable || !images) return;
    if (event.key === 'Escape') {
      clearPointer();
      editable.onCancel?.();
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    if (pointer.current || (mode === 'move' && !selected)) return;
    if (event.key === 'Enter' && shownDraft) {
      commit(shownDraft);
      setDraft(null);
      event.preventDefault();
      return;
    }
    const step = event.shiftKey ? .02 : .004;
    const selectedPosition = mode === 'move' ? selected?.position : undefined;
    const position = { ...(shownDraft?.position ?? selectedPosition ?? { x: .5, y: .74 }) };
    if (event.key === 'ArrowLeft') position.x -= step;
    else if (event.key === 'ArrowRight') position.x += step;
    else if (event.key === 'ArrowUp') position.y -= step;
    else if (event.key === 'ArrowDown') position.y += step;
    else return;
    position.x = clamp(position.x, 0, 1);
    position.y = clamp(position.y, 0, 1);
    const kind = addKind ?? selected?.kind;
    if (!kind) return;
    event.preventDefault();
    setDraft({
      kind, id: addKind ? null : selected?.id ?? null,
      preset: addKind ? addPreset : selected?.preset,
      position, mirrored: mode === 'move' && !!selected?.mirrored, editRevision,
    });
  }

  return <div className="r5-sample" ref={host}>
    <canvas
      className="r5-sample__canvas"
      ref={output}
      role={editable ? undefined : 'img'}
      aria-label={editable ? 'R5 林下。拖动物件，或用方向键预览，回车确认。' : 'R5 林下'}
      tabIndex={editable ? 0 : undefined}
      width={fit.pixelWidth}
      height={fit.pixelHeight}
      style={{ width: fit.cssWidth, height: fit.cssHeight }}
      data-r5-live
      data-ready={Boolean(images) || undefined}
      data-forest-landscape={landscape ? 'r5' : undefined}
      data-art-sampling-width={samplingWidth}
      data-editable={Boolean(editable) || undefined}
      data-dragging={dragging || undefined}
      data-placement-mode={mode}
      data-placement-kind={addKind}
      data-placement-preset={addPreset}
      data-preview-site={preview?.site}
      data-preview-cells={preview?.cells.length ? preview.cells.map(({ index }) => index).join(',') : undefined}
      data-preview-foot-cells={shownDraft?.kind === 'barkShelter' && preview?.cells.length
        ? preview.cells.filter(({ role }) => role === 'solid').map(({ index }) => index).join(',') : undefined}
      data-preview-passage-cells={shownDraft?.kind === 'barkShelter' && preview?.cells.length
        ? preview.cells.filter(({ role }) => role === 'passage').map(({ index }) => index).join(',') : undefined}
      data-viewport={`${viewport.x},${viewport.y},${viewport.width},${viewport.height}`}
      data-rock-ids={snapshot.forestRocks?.rocks.map(({ id }) => id).join(',') ?? ''}
      data-log-ids={snapshot.forestRocks?.logs?.map(({ id }) => id).join(',')}
      data-stonework-ids={snapshot.forestRocks?.stonework?.map(({ id }) => id).join(',')}
      data-bark-shelter-ids={snapshot.forestRocks?.barkShelters?.map(({ id }) => id).join(',')}
      data-bark-occupant-ids={snapshot.forestRocks?.barkShelters?.flatMap(({ occupantIds }) => occupantIds).join(',')}
      data-leaf-patch-ids={snapshot.forestRocks?.leafPatches?.map(({ id }) => id).join(',')}
      data-leaf-patch-frames={snapshot.forestRocks?.leafPatches?.map((patch) => `${patch.id}:${leafPatchFrame(patch)}`).join(',')}
      data-selected-object-id={selected?.id}
      data-selected-rock-id={selected?.kind === 'rock' ? selected.id : undefined}
      data-selected-log-id={selected?.kind === 'log' ? selected.id : undefined}
      data-selected-stonework-id={selected?.kind === 'stonework' ? selected.id : undefined}
      data-selected-bark-shelter-id={selected?.kind === 'barkShelter' ? selected.id : undefined}
      data-selected-leaf-patch-id={selected?.kind === 'leafPatch' ? selected.id : undefined}
      onPointerDown={press}
      onPointerMove={move}
      onPointerUp={release}
      onPointerCancel={abort}
      onLostPointerCapture={abort}
      onPointerLeave={() => { if (mode === 'add' && !pointer.current) setDraft(null); }}
      onKeyDown={keyboard}
    />
    {!images && <span className="r5-sample__status" role="status">{loadFailed ? 'R5 林下画面载入失败' : '正在载入林下画面'}</span>}
  </div>;
}
