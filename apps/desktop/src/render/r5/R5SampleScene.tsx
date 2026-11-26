import { type KeyboardEvent, type PointerEvent, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import {
  R5_HEIGHT, R5_WIDTH, type Point, type Rect, type R5SampleDensity, type R5SampleFocus,
  type R5SampleObject, type R5SampleTheme, type R5SampleZoom, pointerOnSample,
  previewPosition, sampleProjection, sampleViewport,
} from './geometry';
import {
  type SampleImages, type SamplePose, initialPlacements, loadSampleArt, placedArt, placementHit,
  sampleAnimal, sceneArt, supportPoint, waterWindow,
} from './sceneArt';
import { drawSampleBuffer, presentSample } from './drawSample';
import { useCanvasFit } from './useCanvasFit';
import './sample.css';

export type {
  R5SampleDensity, R5SampleFocus, R5SampleObject, R5SampleTheme, R5SampleZoom,
} from './geometry';

export type R5SamplePlacement = Point & { object: R5SampleObject };

export type R5SampleProps = {
  theme: R5SampleTheme;
  density?: R5SampleDensity;
  zoom?: R5SampleZoom;
  focus?: R5SampleFocus;
  waterLevel?: number;
  pose?: SamplePose;
  editPreview?: boolean;
  selectedObject?: R5SampleObject | null;
  resetKey?: string | number;
  showScale?: boolean;
  onPlacement?: (placement: R5SamplePlacement) => void;
  className?: string;
};

type Drag = {
  pointerId: number;
  object: R5SampleObject;
  offset: Point;
  viewport: Rect;
};

const movement: Record<R5SampleTheme, Rect> = {
  forest: { x: 0, y: 132, width: R5_WIDTH, height: 113 },
  pond: { x: 0, y: 126, width: R5_WIDTH, height: 110 },
};
const centre = { x: R5_WIDTH / 2, y: R5_HEIGHT / 2 };

function Scene({
  theme, density = 480, zoom = 1, focus = 'center', waterLevel = .65, pose = 0,
  editPreview = false, selectedObject, showScale = false, onPlacement, className,
}: R5SampleProps) {
  const art = sceneArt(theme);
  const host = useRef<HTMLDivElement>(null);
  const output = useRef<HTMLCanvasElement>(null);
  const buffer = useRef<HTMLCanvasElement | null>(null);
  const drag = useRef<Drag | null>(null);
  const [images, setImages] = useState<SampleImages | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);
  const [placements, setPlacements] = useState(() => initialPlacements(art));
  const [dragPreview, setDragPreview] = useState<R5SamplePlacement | null>(null);
  const fit = useCanvasFit(host);
  const displayed = useMemo(() => editPreview && dragPreview
    ? { ...placements, [dragPreview.object]: dragPreview } : placements, [dragPreview, editPreview, placements]);
  const animal = useMemo(() => sampleAnimal(art, theme, displayed, pose), [art, theme, displayed, pose]);
  const viewport = useMemo(() => {
    const focalPoint = focus === 'animal' ? animal.position : focus === 'contact' ? supportPoint(art, displayed) : centre;
    return (editPreview && drag.current?.viewport) || sampleViewport(zoom, focalPoint);
  }, [animal, art, displayed, editPreview, focus, zoom]);
  const water = useMemo(() => theme === 'pond' ? waterWindow(art, waterLevel) : null, [art, theme, waterLevel]);
  const activeObject = editPreview ? dragPreview?.object ?? selectedObject ?? null : null;
  const label = theme === 'forest' ? '林下 R5 尺度样板' : '浅塘 R5 尺度样板';

  useEffect(() => {
    let live = true;
    loadSampleArt(theme).then((loaded) => { if (live) setImages(loaded); })
      .catch(() => { if (live) setLoadFailed(true); });
    return () => { live = false; };
  }, [theme]);

  function cancelDrag() {
    const pointerId = drag.current?.pointerId;
    drag.current = null;
    setDragPreview(null);
    const canvas = output.current;
    if (pointerId !== undefined && canvas?.hasPointerCapture(pointerId)) canvas.releasePointerCapture(pointerId);
  }

  useEffect(() => {
    if (!editPreview || (selectedObject !== undefined && drag.current && drag.current.object !== selectedObject)) cancelDrag();
  }, [editPreview, selectedObject]);

  useLayoutEffect(() => {
    const canvas = output.current;
    if (!canvas || !images || !fit.cssWidth || !fit.cssHeight) return;
    const logical = buffer.current ??= document.createElement('canvas');
    if (logical.width !== density) logical.width = density;
    if (logical.height !== density * R5_HEIGHT / R5_WIDTH) logical.height = density * R5_HEIGHT / R5_WIDTH;
    const frame = { art, images, theme, placements: displayed, animal, selected: activeObject, water };
    drawSampleBuffer(logical, frame);
    presentSample(canvas, logical, viewport, frame, showScale, fit.devicePixelRatio);
  }, [activeObject, animal, art, density, displayed, fit, images, showScale, theme, viewport, water]);

  function positionOf(event: PointerEvent<HTMLCanvasElement>, view: Rect): Point | null {
    const canvas = event.currentTarget.getBoundingClientRect();
    if (!canvas.width || !canvas.height) return null;
    return pointerOnSample({ x: event.clientX, y: event.clientY }, canvas, view);
  }

  function startDrag(event: PointerEvent<HTMLCanvasElement>) {
    if (!images || !editPreview || event.button !== 0 || !event.isPrimary || drag.current) return;
    const pointer = positionOf(event, viewport);
    if (!pointer) return;
    const candidates: R5SampleObject[] = selectedObject === undefined ? ['stone', 'log'] : selectedObject ? [selectedObject] : [];
    candidates.sort((a, b) => placements[b].y - placements[a].y);
    const object = candidates.find((candidate) => placementHit(placedArt(art, candidate), placements[candidate], pointer, images));
    if (!object) return;
    const { x, y } = placements[object];
    event.preventDefault();
    event.currentTarget.focus({ preventScroll: true });
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { pointerId: event.pointerId, object, offset: { x: pointer.x - x, y: pointer.y - y }, viewport };
    setDragPreview({ object, x, y });
  }

  function dragTo(event: PointerEvent<HTMLCanvasElement>): R5SamplePlacement | null {
    const active = drag.current;
    if (!editPreview || !active || active.pointerId !== event.pointerId) return null;
    const at = positionOf(event, active.viewport);
    if (!at) return null;
    return { object: active.object, ...previewPosition({ x: at.x - active.offset.x, y: at.y - active.offset.y }, movement[theme]) };
  }

  function moveDrag(event: PointerEvent<HTMLCanvasElement>) {
    const next = dragTo(event);
    if (next) setDragPreview(next);
  }

  function commit(next: R5SamplePlacement) {
    const previous = placements[next.object];
    if (previous.x === next.x && previous.y === next.y) return;
    setPlacements((current) => ({ ...current, [next.object]: { x: next.x, y: next.y } }));
    onPlacement?.(next);
  }

  function finishDrag(event: PointerEvent<HTMLCanvasElement>) {
    const next = dragTo(event);
    if (next) commit(next);
    if (drag.current?.pointerId === event.pointerId) cancelDrag();
  }

  function abandonDrag(event: PointerEvent<HTMLCanvasElement>) {
    if (drag.current?.pointerId === event.pointerId) cancelDrag();
  }

  function keyboard(event: KeyboardEvent<HTMLCanvasElement>) {
    if (event.key === 'Escape' && drag.current) {
      event.preventDefault();
      event.stopPropagation();
      cancelDrag();
      return;
    }
    if (!editPreview || !images || !selectedObject || drag.current) return;
    const step = event.shiftKey ? 8 : 2;
    const { x, y } = placements[selectedObject];
    const next: Point = { x, y };
    if (event.key === 'ArrowLeft') next.x -= step;
    else if (event.key === 'ArrowRight') next.x += step;
    else if (event.key === 'ArrowUp') next.y -= step;
    else if (event.key === 'ArrowDown') next.y += step;
    else return;
    event.preventDefault();
    commit({ object: selectedObject, ...previewPosition(next, movement[theme]) });
  }

  return <div className={className ? `r5-sample ${className}` : 'r5-sample'} ref={host}>
    <canvas
      className="r5-sample__canvas"
      ref={output}
      role={editPreview ? undefined : 'img'}
      aria-label={editPreview ? `${label}。选中物件后可拖动或用方向键试摆。` : label}
      tabIndex={editPreview ? 0 : undefined}
      width={fit.pixelWidth}
      height={fit.pixelHeight}
      style={{ width: fit.cssWidth, height: fit.cssHeight }}
      data-theme={theme}
      data-density={density}
      data-zoom={zoom}
      data-focus={focus}
      data-pose={pose}
      data-editable={editPreview || undefined}
      data-dragging={Boolean(editPreview && dragPreview) || undefined}
      data-ready={Boolean(images) || undefined}
      data-reference-width-mm={art.referenceWidthMm}
      data-body-length-mm={animal.animal.lengthMm}
      data-body-length-css={sampleProjection(animal.animal.lengthMm, art.referenceWidthMm, fit.cssWidth, zoom).toFixed(2)}
      data-body-length-logical={sampleProjection(animal.animal.lengthMm, art.referenceWidthMm, density).toFixed(2)}
      data-viewport={`${viewport.x},${viewport.y},${viewport.width},${viewport.height}`}
      data-animal-anchor={`${animal.position.x},${animal.position.y}`}
      data-stone-anchor={`${displayed.stone.x},${displayed.stone.y}`}
      data-log-anchor={`${displayed.log.x},${displayed.log.y}`}
      data-waterline={water?.y}
      onPointerDown={startDrag}
      onPointerMove={moveDrag}
      onPointerUp={finishDrag}
      onPointerCancel={abandonDrag}
      onLostPointerCapture={abandonDrag}
      onKeyDown={keyboard}
    />
    {!images && <span className="r5-sample__status" role="status">
      {loadFailed ? '样板素材载入失败' : '正在载入样板'}
    </span>}
  </div>;
}

export function R5SampleScene(props: R5SampleProps) {
  return <Scene key={`${props.theme}/${props.resetKey ?? ''}`} {...props} />;
}
