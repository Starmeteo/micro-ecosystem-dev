import { type KeyboardEvent, type PointerEvent, useEffect, useRef, useState } from 'react';
import type { ForestSubstrateCell, ForestSubstratePreset, LeafPatchPreset } from '../../world';
import { forestHeight, forestWidth } from './forestComposedArt';
import { forestPosition, forestStage } from './forestComposition';
import { forestPlacementHint, forestSoilCell, forestSoilColumns, forestSoilIndex, forestSoilRows, type ForestObject, type PlacementHint } from './forestPlacement';
import { clamp, pointerOnSample, type Point, type Rect } from './geometry';

export type ForestBrushResult = { positions: Point[]; changedCells: number[]; skipped: number; outside: boolean };
export type R5ForestBrush = ({
  kind: 'leaves'; preset: LeafPatchPreset; capacity: number;
} | {
  kind: 'substrate'; preset: ForestSubstratePreset; soil: readonly ForestSubstrateCell[];
}) & {
  onFinish: (result: ForestBrushResult) => void;
  onCancel: () => void;
};

type BrushCell = PlacementHint['cells'][number] & { unchanged?: boolean; overCapacity?: boolean };
type Stroke = {
  pointer: number;
  target: HTMLCanvasElement;
  authority: string;
  viewport: Rect;
  last: Point;
  cells: Map<number, BrushCell>;
  result: ForestBrushResult;
};

export function drawForestBrush(ctx: CanvasRenderingContext2D, cells: readonly BrushCell[] = []) {
  ctx.save();
  for (const cell of cells) {
    const { x, y } = forestStage(cell);
    const width = cell.width * forestWidth - 2;
    const height = cell.height * forestHeight - 2;
    ctx.strokeStyle = cell.overCapacity ? '#ffc0b3' : cell.occupied ? '#ffd096' : cell.unchanged ? '#edf0ef9c' : '#bbf3d9';
    ctx.fillStyle = cell.overCapacity ? '#e1614b55' : cell.occupied ? '#ffd09624' : cell.unchanged ? '#e4eeec20' : '#bbf3d940';
    ctx.fillRect(x + 1, y + 1, width, height);
    ctx.strokeRect(x + 1, y + 1, width, height);
  }
  ctx.restore();
}

export function useForestBrush(tool: R5ForestBrush | undefined, revision: string, viewport: Rect,
  objects: readonly ForestObject[], animals: readonly Point[]) {
  const stroke = useRef<Stroke | null>(null);
  const keyboardCell = useRef<number | null>(null);
  const [preview, setPreview] = useState<{ authority: string; cells: BrushCell[] } | null>(null);
  const authority = tool ? `${revision}/${tool.kind}/${tool.kind === 'leaves' ? tool.preset : tool.preset.substrate}` : '';

  function clear(releaseCapture = true) {
    const active = stroke.current;
    stroke.current = null;
    if (releaseCapture && active?.target.hasPointerCapture(active.pointer)) active.target.releasePointerCapture(active.pointer);
    setPreview(null);
  }

  useEffect(() => {
    keyboardCell.current = null;
    clear();
  }, [authority]);

  function location(event: PointerEvent<HTMLCanvasElement>, view: Rect) {
    const bounds = event.currentTarget.getBoundingClientRect();
    const x = clamp(event.clientX, bounds.left - 1, bounds.right + 1);
    const y = clamp(event.clientY, bounds.top - 1, bounds.bottom + 1);
    return forestPosition(pointerOnSample({ x, y }, bounds, view));
  }

  function substrateCell(index: number): BrushCell | undefined {
    if (tool?.kind !== 'substrate') return;
    const soil = tool.soil.find((entry) => entry.cellIndex === index);
    if (!soil) return;
    const overCapacity = soil.waterMl > tool.preset.waterCapacityMl;
    return { ...forestSoilCell(index), role: 'substrate', occupied: overCapacity, overCapacity,
      unchanged: soil.substrate === tool.preset.substrate };
  }

  function sample(point: Point): BrushCell | undefined {
    if (tool?.kind === 'substrate') {
      const index = forestSoilIndex(point);
      return index === undefined ? undefined : substrateCell(index);
    }
    if (tool?.kind === 'leaves') {
      return forestPlacementHint({ kind: 'leafPatch', preset: tool.preset, id: null, mirrored: false, position: point }, objects, animals).cells[0];
    }
  }

  function visit(active: Stroke, position: Point) {
    const cell = sample(position);
    if (!cell) {
      active.result.outside = true;
      return;
    }
    if (active.cells.has(cell.index)) return;
    const blocked = cell.occupied || (tool?.kind === 'leaves' && active.result.positions.length >= tool.capacity);
    active.cells.set(cell.index, { ...cell, occupied: blocked });
    if (blocked) active.result.skipped++;
    else if (!cell.unchanged) {
      active.result.positions.push(position);
      active.result.changedCells.push(cell.index);
    }
  }

  function extend(active: Stroke, end: Point) {
    const start = active.last;
    const dx = end.x - start.x;
    const dy = end.y - start.y;
    const steps = Math.max(1, Math.ceil(Math.max(Math.abs(dx), Math.abs(dy)) / .018));
    const previousCount = active.cells.size;
    for (let index = 1; index <= steps; index++) {
      visit(active, { x: start.x + dx * index / steps, y: start.y + dy * index / steps });
    }
    active.last = end;
    if (active.cells.size !== previousCount) setPreview({ authority, cells: [...active.cells.values()] });
  }

  function press(event: PointerEvent<HTMLCanvasElement>) {
    if (!tool || stroke.current || !event.isPrimary || event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.focus({ preventScroll: true });
    event.currentTarget.setPointerCapture(event.pointerId);
    const start = location(event, viewport);
    const active: Stroke = {
      pointer: event.pointerId, target: event.currentTarget, authority, viewport, last: start,
      cells: new Map(), result: { positions: [], changedCells: [], skipped: 0, outside: false },
    };
    stroke.current = active;
    visit(active, start);
    setPreview({ authority, cells: [...active.cells.values()] });
  }

  function move(event: PointerEvent<HTMLCanvasElement>) {
    if (!tool || !event.isPrimary) return;
    const active = stroke.current;
    if (active) {
      if (active.pointer === event.pointerId && active.authority === authority) extend(active, location(event, active.viewport));
      return;
    }
    const cell = sample(location(event, viewport));
    setPreview(cell ? { authority, cells: [{ ...cell, occupied: cell.occupied || (tool.kind === 'leaves' && tool.capacity === 0) }] } : null);
  }

  function release(event: PointerEvent<HTMLCanvasElement>) {
    const active = stroke.current;
    if (!active || active.pointer !== event.pointerId) return;
    if (tool && active.authority === authority) extend(active, location(event, active.viewport));
    clear(false);
    if (tool && active.authority === authority) tool.onFinish(active.result);
  }

  function abort(event: PointerEvent<HTMLCanvasElement>) {
    if (stroke.current?.pointer === event.pointerId) clear(false);
  }

  function onKeyDown(event: KeyboardEvent<HTMLCanvasElement>) {
    if (!tool) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      clear();
      tool.onCancel();
      return;
    }
    if (tool.kind !== 'substrate' || stroke.current || event.altKey || event.ctrlKey || event.metaKey
      || !['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Enter'].includes(event.key)) return;
    event.preventDefault();
    event.stopPropagation();
    const current = keyboardCell.current ?? forestSoilColumns * 2 + 5;
    const column = clamp(current % forestSoilColumns + Number(event.key === 'ArrowRight') - Number(event.key === 'ArrowLeft'), 0, forestSoilColumns - 1);
    const row = clamp(Math.floor(current / forestSoilColumns) + Number(event.key === 'ArrowDown') - Number(event.key === 'ArrowUp'), 0, forestSoilRows - 1);
    const index = row * forestSoilColumns + column;
    const cell = substrateCell(index);
    if (!cell) return;
    keyboardCell.current = index;
    setPreview({ authority, cells: [cell] });
    if (event.key === 'Enter' && !event.repeat) {
      tool.onFinish({ positions: [], changedCells: cell.occupied || cell.unchanged ? [] : [index], skipped: Number(cell.occupied), outside: false });
    }
  }

  return {
    cells: preview?.authority === authority ? preview.cells : undefined,
    handlers: {
      onPointerDown: press, onPointerMove: move, onPointerUp: release, onPointerCancel: abort, onLostPointerCapture: abort,
      onPointerLeave: () => { if (!stroke.current) setPreview(null); },
      onKeyDown,
    },
  };
}
