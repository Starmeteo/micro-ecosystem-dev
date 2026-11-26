import { type KeyboardEvent, type PointerEvent, useEffect, useRef, useState } from 'react';
import { clamp, pointerOnSample, type Point, type Rect } from './geometry';

export type StockPositionTool = {
  onPlace: (position: Point) => void;
  onCancel: () => void;
};

type StockSite = 'clear' | 'outside' | 'blocked';

export function useStockPosition(tool: StockPositionTool | undefined, revision: string, camera: Rect,
  fromStage: (point: Point) => Point, center: Point, site: (point: Point) => StockSite) {
  const [draft, setDraft] = useState<{ revision: string; point: Point } | null>(null);
  const press = useRef<{ pointer: number; screen: Point; revision: string } | null>(null);
  const enabled = Boolean(tool);
  const point = enabled && draft?.revision === revision ? draft.point : undefined;
  const state = point ? site(point) : undefined;

  useEffect(() => { setDraft(null); press.current = null; }, [revision, enabled]);

  function read(event: PointerEvent<HTMLCanvasElement>) {
    return fromStage(pointerOnSample({ x: event.clientX, y: event.clientY }, event.currentTarget.getBoundingClientRect(), camera));
  }

  function show(point: Point) { setDraft({ revision, point }); }

  return {
    point, state,
    handlers: {
      onPointerDown: (event: PointerEvent<HTMLCanvasElement>) => {
        if (!tool || event.button !== 0 || !event.isPrimary) return;
        event.preventDefault();
        event.currentTarget.focus({ preventScroll: true });
        press.current = { pointer: event.pointerId, screen: { x: event.clientX, y: event.clientY }, revision };
        show(read(event));
      },
      onPointerMove: (event: PointerEvent<HTMLCanvasElement>) => { if (tool && event.isPrimary) show(read(event)); },
      onPointerUp: (event: PointerEvent<HTMLCanvasElement>) => {
        const start = press.current;
        press.current = null;
        if (!tool || start?.pointer !== event.pointerId || start.revision !== revision
          || Math.hypot(event.clientX - start.screen.x, event.clientY - start.screen.y) > 6) return;
        const position = read(event);
        if (site(position) === 'clear') tool.onPlace(position);
      },
      onPointerCancel: () => { press.current = null; setDraft(null); },
      onPointerLeave: () => { press.current = null; setDraft(null); },
      onKeyDown: (event: KeyboardEvent<HTMLCanvasElement>) => {
        if (!tool) return;
        if (event.key === 'Escape') {
          press.current = null;
          setDraft(null);
          tool.onCancel();
          event.preventDefault();
          event.stopPropagation();
          return;
        }
        if (event.key === 'Enter') {
          if (point && state === 'clear') tool.onPlace(point);
          else if (!point) show(center);
          event.preventDefault();
          return;
        }
        const offset = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[event.key];
        if (!offset) return;
        const current = point ?? center;
        show({ x: clamp(current.x + offset[0] * .02, 0, 1), y: clamp(current.y + offset[1] * .02, 0, 1) });
        event.preventDefault();
      },
    },
  };
}

export function drawStockMarker(ctx: CanvasRenderingContext2D, point: Point, valid: boolean) {
  ctx.save();
  ctx.strokeStyle = valid ? '#bbf3d9' : '#ffd096';
  ctx.beginPath();
  ctx.arc(point.x, point.y, 7, 0, 2 * Math.PI);
  ctx.moveTo(point.x - 3, point.y);
  ctx.lineTo(point.x + 3, point.y);
  ctx.moveTo(point.x, point.y - 3);
  ctx.lineTo(point.x, point.y + 3);
  ctx.stroke();
  ctx.restore();
}
