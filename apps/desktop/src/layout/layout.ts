export const LAYOUT_AREA = { width: 1180, height: 700 };
export const MAX_PIECES = 8;
export const pieceScaleLimits = { minimum: 75, maximum: 175 } as const;

export const PIECES = {
  ecology: { label: '生态画面', width: 384, height: 216 },
  clock: { label: '时钟', width: 172, height: 112 },
  environment: { label: '环境信息', width: 194, height: 112 },
} as const;

const dockSizes = {
  clock: { width: 148, height: 78 },
  environment: { width: 162, height: 78 },
};
const desktop = { ...LAYOUT_AREA, x: 0, y: 0 };
const gap = 10;
const attachMargin = 16;
const detachMargin = 18;
const latticeStep = 16;
const latticeRadius = 4;
const guideRadius = 6;
const neighboringDistance = 48;

export type PieceKind = keyof typeof PIECES;
export type CompositionMode = 'combined' | 'free' | 'mixed';

interface Point { x: number; y: number }
interface Size { width: number; height: number }
interface Rectangle extends Point, Size {}

export interface WorkspacePiece extends Point {
  id: string;
  kind: PieceKind;
  docked?: boolean;
  primary?: true;
  worldId?: string | null;
  scalePercent?: number;
}

export type EcologyBindings = Record<string, string | null>;

export interface PanelPlacement {
  monitorName: string | null;
  monitorX: number;
  monitorY: number;
  horizontal: number;
  vertical: number;
}

export interface WorkspaceLayout {
  version: 2;
  mode: CompositionMode;
  items: WorkspacePiece[];
  idle: { mode: 'clock' | 'text' | 'off'; text: string };
  panelWorldId?: string;
  panelPlacement?: PanelPlacement;
}

interface DragResult {
  layout: WorkspaceLayout;
  preview: WorkspaceLayout;
  portal: 'attach' | 'detach' | null;
}

export const DEFAULT_LAYOUT: WorkspaceLayout = {
  version: 2,
  mode: 'combined',
  items: [{ id: 'forest', kind: 'ecology', x: 772, y: 460 }],
  idle: { mode: 'clock', text: '' },
};

export function pieceSize(piece: WorkspacePiece): Size {
  const size = piece.kind !== 'ecology' && piece.docked ? dockSizes[piece.kind] : PIECES[piece.kind];
  const scale = (piece.scalePercent ?? 100) / 100;
  return { width: Math.round(size.width * scale), height: Math.round(size.height * scale) };
}

export function pieceWorld(piece: WorkspacePiece, defaultWorld?: string): string | undefined {
  return piece.worldId === undefined ? defaultWorld : piece.worldId ?? undefined;
}

export function applyEcologyBindings(layout: WorkspaceLayout, bindings: EcologyBindings): WorkspaceLayout {
  const items = layout.items.map((item) => item.kind === 'ecology' && Object.hasOwn(bindings, item.id)
    ? { ...item, worldId: bindings[item.id] } : item);
  return { ...layout, items };
}

function pieceBox(piece: WorkspacePiece): Rectangle {
  return { ...pieceSize(piece), x: piece.x, y: piece.y };
}

export function bounds(items: readonly WorkspacePiece[]): Rectangle {
  if (!items.length) throw new Error('面板至少需要一个组件');
  const rects = items.map(pieceBox);
  const x = Math.min(...rects.map((rect) => rect.x));
  const y = Math.min(...rects.map((rect) => rect.y));
  const width = Math.max(...rects.map((rect) => rect.x + rect.width)) - x;
  const height = Math.max(...rects.map((rect) => rect.y + rect.height)) - y;
  return { x, y, width, height };
}

export function ecologyAnchor(items: readonly WorkspacePiece[]): WorkspacePiece | undefined {
  const designated = items.find((piece) => piece.kind === 'ecology' && piece.primary);
  if (designated) return designated;
  // Older layouts establish the same anchor independently of layer order.
  return items.reduce<WorkspacePiece | undefined>((anchor, piece) => {
    if (piece.kind !== 'ecology') return anchor;
    return !anchor || piece.id === 'forest' || (anchor.id !== 'forest' && piece.id < anchor.id) ? piece : anchor;
  }, undefined);
}

export function effectiveMode(layout: WorkspaceLayout): CompositionMode {
  return ecologyAnchor(layout.items) ? layout.mode : 'free';
}

const clip = (value: number, low: number, high: number) => Math.round(Math.max(low, Math.min(value, high)));

function contains(outer: Rectangle, inner: Rectangle): boolean {
  return inner.x >= outer.x && inner.y >= outer.y
    && inner.x + inner.width <= outer.x + outer.width && inner.y + inner.height <= outer.y + outer.height;
}

function containsPoint(rect: Rectangle, point: Point, margin: number): boolean {
  return point.x >= rect.x + margin && point.x <= rect.x + rect.width - margin
    && point.y >= rect.y + margin && point.y <= rect.y + rect.height - margin;
}

function fitPiece(piece: WorkspacePiece, area: Rectangle): WorkspacePiece {
  const { width, height } = pieceSize(piece);
  return {
    ...piece,
    x: clip(piece.x, area.x, area.x + area.width - width),
    y: clip(piece.y, area.y, area.y + area.height - height),
  };
}

function snapAxis(value: number, min: number, max: number, guides: number[], scale: number): number {
  const bounded = Math.max(min, Math.min(value, max));
  const candidates = [min, max, ...guides];
  let nearest: number | undefined;
  let distance = guideRadius * scale;
  for (const candidate of candidates) {
    if (candidate < min || candidate > max) continue;
    const difference = Math.abs(candidate - bounded);
    if (difference > distance || (difference === distance && nearest !== undefined && candidate > nearest)) continue;
    nearest = candidate;
    distance = difference;
  }
  if (nearest !== undefined) return clip(nearest, min, max);

  const step = latticeStep * scale;
  const lattice = min + Math.round((bounded - min) / step) * step;
  return clip(Math.abs(lattice - bounded) <= latticeRadius * scale ? lattice : bounded, min, max);
}

function alignments(start: number, extent: number, movingExtent: number): number[] {
  const end = start + extent;
  return [start, end - movingExtent, start + (extent - movingExtent) / 2, start - movingExtent, end];
}

export function snapPlacement(
  requested: Rectangle,
  area: Rectangle,
  movement: Point,
  neighbors: readonly Rectangle[] = [],
  scale = 1,
): Point {
  const xMax = Math.max(area.x, area.x + area.width - requested.width);
  const yMax = Math.max(area.y, area.y + area.height - requested.height);
  const x = clip(requested.x, area.x, xMax);
  const y = clip(requested.y, area.y, yMax);
  const padding = neighboringDistance * scale;
  const xGuides = neighbors.filter((other) => (
    other.y <= y + requested.height + padding && other.y + other.height >= y - padding
  )).flatMap((other) => alignments(other.x, other.width, requested.width));
  const yGuides = neighbors.filter((other) => (
    other.x <= x + requested.width + padding && other.x + other.width >= x - padding
  )).flatMap((other) => alignments(other.y, other.height, requested.height));

  return {
    x: movement.x ? snapAxis(requested.x, area.x, xMax, xGuides, scale) : x,
    y: movement.y ? snapAxis(requested.y, area.y, yMax, yGuides, scale) : y,
  };
}

function shiftToFit(items: readonly WorkspacePiece[], dx: number, dy: number): WorkspacePiece[] {
  const rect = bounds(items);
  const x = clip(dx, -rect.x, LAYOUT_AREA.width - rect.x - rect.width);
  const y = clip(dy, -rect.y, LAYOUT_AREA.height - rect.y - rect.height);
  return items.map((piece) => ({ ...piece, x: piece.x + x, y: piece.y + y }));
}

function dragGroup(items: WorkspacePiece[], dx: number, dy: number, neighbors: WorkspacePiece[] = []): WorkspacePiece[] {
  const rect = bounds(items);
  const requested = { ...rect, x: rect.x + dx, y: rect.y + dy };
  const position = snapPlacement(requested, desktop, { x: dx, y: dy }, neighbors.map(pieceBox));
  return shiftToFit(items, position.x - rect.x, position.y - rect.y);
}

function center(piece: WorkspacePiece): Point {
  const size = pieceSize(piece);
  return { x: piece.x + size.width / 2, y: piece.y + size.height / 2 };
}

function atCenter(piece: WorkspacePiece, point: Point): WorkspacePiece {
  const size = pieceSize(piece);
  return { ...piece, x: point.x - size.width / 2, y: point.y - size.height / 2 };
}

function undock(piece: WorkspacePiece, point = center(piece)): WorkspacePiece {
  const { docked: _docked, ...free } = piece;
  return fitPiece(atCenter(free, point), desktop);
}

function placeDock(piece: WorkspacePiece, host: Rectangle, ordinal: number): WorkspacePiece {
  const docked = { ...piece, docked: true };
  const size = pieceSize(docked);
  const cascade = Math.min(ordinal, 3) * 5;
  const point = piece.kind === 'clock'
    ? { x: host.x + host.width - size.width - gap - cascade, y: host.y + gap + cascade }
    : { x: host.x + gap + cascade, y: host.y + host.height - size.height - gap - cascade };
  return fitPiece({ ...docked, ...point }, host);
}

function placeCombined(layout: WorkspaceLayout, host: WorkspacePiece, keepSlots: boolean): WorkspaceLayout {
  const slots = { clock: 0, environment: 0 };
  const hostRect = pieceBox(host);
  const items = layout.items.map((piece) => {
    if (piece.kind === 'ecology') return piece;
    const ordinal = slots[piece.kind]++;
    return keepSlots && piece.docked ? piece : placeDock(piece, hostRect, ordinal);
  });
  return raisePiece({ ...layout, items }, host.id);
}

export function setComposition(layout: WorkspaceLayout, mode: CompositionMode): WorkspaceLayout {
  if (layout.mode === mode) return layout;
  const next = { ...layout, mode };
  const host = ecologyAnchor(layout.items);
  if (!host || mode === 'mixed') return next;
  if (mode === 'combined') return placeCombined(next, host, true);
  const placed = next.items.filter((piece) => !piece.docked);
  const detached = new Map<string, WorkspacePiece>();
  for (const piece of next.items) {
    if (!piece.docked) continue;
    const free = placeFree(placed, undock(piece));
    placed.push(free);
    detached.set(free.id, free);
  }
  return { ...next, items: next.items.map((piece) => detached.get(piece.id) ?? piece) };
}

function placeFree(items: WorkspacePiece[], piece: WorkspacePiece): WorkspacePiece {
  const group = bounds(items);
  const size = pieceSize(piece);
  const spots = [
    { x: group.x + group.width + gap, y: group.y },
    { x: group.x, y: group.y + group.height + gap },
    { x: group.x - size.width - gap, y: group.y },
    { x: group.x, y: group.y - size.height - gap },
  ];
  const position = spots.find((point) => contains(desktop, { ...point, ...size }));
  return position ? { ...piece, ...position } : fitPiece({ ...piece, x: group.x, y: group.y }, desktop);
}

export function togglePieceKind(layout: WorkspaceLayout, kind: PieceKind, id: string): WorkspaceLayout {
  const existing = layout.items.filter((piece) => piece.kind === kind);
  if (existing.length) {
    if (existing.length === layout.items.length) return layout;
    return replaceAfterRemoval(layout, layout.items.filter((piece) => piece.kind !== kind));
  }
  if (layout.items.length >= MAX_PIECES || layout.items.some((piece) => piece.id === id)) return layout;

  const piece: WorkspacePiece = { id, kind, x: 0, y: 0 };
  const host = ecologyAnchor(layout.items);
  if (host && layout.mode === 'combined') {
    return { ...layout, items: [...layout.items, placeDock(piece, pieceBox(host), 0)] };
  }
  const placed = placeFree(layout.items, piece);
  const next = { ...layout, items: [...layout.items, placed] };
  return kind === 'ecology' && layout.mode === 'combined' ? placeCombined(next, placed, false) : next;
}

export function addEcologyPiece(layout: WorkspaceLayout, id: string): WorkspaceLayout {
  if (layout.items.length >= MAX_PIECES || layout.items.some((piece) => piece.id === id)) return layout;
  const piece: WorkspacePiece = { id, kind: 'ecology', x: 0, y: 0, worldId: null };
  const placed = placeFree(layout.items, piece);
  const host = ecologyAnchor(layout.items);
  const items = host ? layout.items.map((item) => item === host ? { ...item, primary: true as const } : item) : layout.items;
  const next = { ...layout, items: [...items, placed] };
  return !host && layout.mode === 'combined' ? placeCombined(next, placed, false) : next;
}

export function removePiece(layout: WorkspaceLayout, id: string): WorkspaceLayout {
  if (layout.items.length < 2 || !layout.items.some((item) => item.id === id)) return layout;
  return replaceAfterRemoval(layout, layout.items.filter((item) => item.id !== id));
}

export function resizePiece(layout: WorkspaceLayout, id: string, percent: number): WorkspaceLayout {
  const piece = layout.items.find((item) => item.id === id);
  if (!piece || !Number.isFinite(percent)) return layout;
  const scalePercent = clip(percent, pieceScaleLimits.minimum, pieceScaleLimits.maximum);
  if (scalePercent === (piece.scalePercent ?? 100)) return layout;
  const { scalePercent: _old, ...unscaled } = piece;
  const requested = scalePercent === 100 ? unscaled : { ...unscaled, scalePercent };
  const host = ecologyAnchor(layout.items);
  const resized = fitPiece(requested, piece.docked && host ? pieceBox(host) : desktop);
  const next = replacePiece(layout, resized);
  if (host !== piece) return next;
  const previous = pieceBox(piece);
  const area = pieceBox(resized);
  const items = next.items.map((item) => {
    if (!item.docked) return item;
    const { width, height } = pieceSize(item);
    const xRatio = (item.x - previous.x) / Math.max(1, previous.width - width);
    const yRatio = (item.y - previous.y) / Math.max(1, previous.height - height);
    return fitPiece({ ...item, x: area.x + xRatio * (area.width - width), y: area.y + yRatio * (area.height - height) }, area);
  });
  return { ...next, items };
}

function replacePiece(layout: WorkspaceLayout, replacement: WorkspacePiece): WorkspaceLayout {
  return { ...layout, items: layout.items.map((piece) => piece.id === replacement.id ? replacement : piece) };
}

export function dragPiece(layout: WorkspaceLayout, id: string, dx: number, dy: number): DragResult {
  const moving = layout.items.find((piece) => piece.id === id);
  if (!moving || !Number.isFinite(dx) || !Number.isFinite(dy) || (!dx && !dy)) {
    return { layout, preview: layout, portal: null };
  }
  const fixed = (next: WorkspaceLayout): DragResult => ({ layout: next, preview: next, portal: null });
  const host = ecologyAnchor(layout.items);
  const requested = { ...moving, x: moving.x + dx, y: moving.y + dy };
  const free = layout.items.filter((piece) => piece !== moving && !piece.docked);
  const docked = layout.items.filter((piece) => piece !== moving && piece.docked);
  function snapped(piece: WorkspacePiece, area: Rectangle, neighbors: WorkspacePiece[]): WorkspacePiece {
    return { ...piece, ...snapPlacement(pieceBox(piece), area, { x: dx, y: dy }, neighbors.map(pieceBox)) };
  }
  if (!host) return fixed(replacePiece(layout, snapped(requested, desktop, free)));

  if (moving === host) {
    const shifted = dragGroup([host, ...docked], dx, dy, free);
    const changed = new Map(shifted.map((piece) => [piece.id, piece]));
    return fixed({ ...layout, items: layout.items.map((piece) => changed.get(piece.id) ?? piece) });
  }

  const hostRect = pieceBox(host);
  const desiredCenter = center(requested);
  if (moving.docked) {
    if (layout.mode === 'mixed' && !containsPoint(hostRect, desiredCenter, -detachMargin)) {
      const detached = replacePiece(layout, snapped(undock(requested), desktop, free));
      return { layout: detached, preview: detached, portal: 'detach' };
    }
    return fixed(replacePiece(layout, snapped(requested, hostRect, docked)));
  }

  const freePreview = snapped(requested, desktop, free);
  const preview = replacePiece(layout, freePreview);
  if (layout.mode === 'mixed' && moving.kind !== 'ecology' && containsPoint(hostRect, desiredCenter, attachMargin)) {
    const attached = fitPiece(atCenter({ ...requested, docked: true }, center(freePreview)), hostRect);
    return { layout: raisePiece(replacePiece(layout, attached), id), preview, portal: 'attach' };
  }
  return fixed(preview);
}

export function movePanel(layout: WorkspaceLayout, dx: number, dy: number): WorkspaceLayout {
  if (!Number.isFinite(dx) || !Number.isFinite(dy) || (!dx && !dy)) return layout;
  return { ...layout, items: dragGroup(layout.items, dx, dy) };
}

export function shiftToPanelOrigin(layout: WorkspaceLayout, x: number, y: number): WorkspaceLayout {
  if (!Number.isFinite(x) || !Number.isFinite(y)) return layout;
  const rect = bounds(layout.items);
  return { ...layout, items: shiftToFit(layout.items, x - rect.x, y - rect.y) };
}

export function raisePiece(layout: WorkspaceLayout, id: string): WorkspaceLayout {
  const selected = layout.items.find((piece) => piece.id === id);
  if (!selected) return layout;
  const docked = selected === ecologyAnchor(layout.items) ? layout.items.filter((piece) => piece.docked) : [];
  const stack = [selected, ...docked];
  const items = [...layout.items.filter((piece) => !stack.includes(piece)), ...stack];
  return items.every((piece, index) => piece === layout.items[index]) ? layout : { ...layout, items };
}

function replaceAfterRemoval(layout: WorkspaceLayout, items: WorkspacePiece[]): WorkspaceLayout {
  const next = { ...layout, items };
  const host = ecologyAnchor(items);
  if (host === ecologyAnchor(layout.items)) return next;
  if (host && layout.mode === 'combined') return placeCombined(next, host, false);
  return { ...next, items: next.items.map((piece) => piece.docked ? undock(piece) : piece) };
}

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown> : null;
}

function isMode(value: unknown): value is CompositionMode {
  return value === 'combined' || value === 'free' || value === 'mixed';
}

function readPanelPlacement(value: unknown): PanelPlacement | undefined {
  const placement = record(value);
  if (!placement) return;
  const { monitorName, monitorX, monitorY, horizontal, vertical } = placement;
  const isCoordinate = (entry: unknown): entry is number => typeof entry === 'number' && Number.isInteger(entry)
    && entry >= -2147483648 && entry <= 2147483647;
  const isRatio = (entry: unknown): entry is number => typeof entry === 'number' && entry >= 0 && entry <= 1;
  if ((monitorName !== null && (typeof monitorName !== 'string' || monitorName.length > 128))
    || !isCoordinate(monitorX) || !isCoordinate(monitorY) || !isRatio(horizontal) || !isRatio(vertical)) return;
  return { monitorName, monitorX, monitorY, horizontal, vertical };
}

function migrate(items: WorkspacePiece[], idle: WorkspaceLayout['idle']): WorkspaceLayout {
  const host = ecologyAnchor(items);
  if (!host) return { version: 2, mode: 'free', items, idle };
  const hostRect = pieceBox(host);
  const migrated = items.map((piece) => piece.kind !== 'ecology' && contains(hostRect, pieceBox(piece))
    ? { ...piece, docked: true } : piece);
  const dockCount = migrated.filter((piece) => piece.docked).length;
  const peripheralCount = migrated.filter((piece) => piece.kind !== 'ecology').length;
  const mode = dockCount === peripheralCount ? 'combined' : dockCount === 0 ? 'free' : 'mixed';
  return { version: 2, mode, items: migrated, idle };
}

export function readLayout(value: unknown): WorkspaceLayout {
  if (value == null) return DEFAULT_LAYOUT;
  const source = record(value);
  const idle = record(source?.idle);
  if ((source?.version !== 1 && source?.version !== 2) || !Array.isArray(source.items) || !source.items.length || source.items.length > MAX_PIECES
    || !idle || (idle.mode !== 'clock' && idle.mode !== 'text' && idle.mode !== 'off')
    || typeof idle.text !== 'string' || idle.text.length > 30) {
    throw new Error('无法读取此版本的组件布局');
  }
  const version = source.version;
  const ids = new Set<string>();
  let primary = false;
  const items = source.items.map((item): WorkspacePiece => {
    const entry = record(item);
    if (!entry || typeof entry.id !== 'string' || !entry.id || entry.id.length > 70 || ids.has(entry.id)
      || typeof entry.kind !== 'string' || !Object.hasOwn(PIECES, entry.kind)
      || typeof entry.x !== 'number' || !Number.isInteger(entry.x)
      || typeof entry.y !== 'number' || !Number.isInteger(entry.y)
      || (version === 2 && entry.docked !== undefined && typeof entry.docked !== 'boolean')) {
      throw new Error('组件布局包含无效的组件');
    }
    const piece: WorkspacePiece = { id: entry.id, kind: entry.kind as PieceKind, x: entry.x, y: entry.y };
    if (version === 2 && entry.docked === true) piece.docked = true;
    if (version === 2 && entry.primary !== undefined) {
      if (entry.primary !== true || primary || piece.kind !== 'ecology') throw new Error('主生态组件的标记无效');
      piece.primary = true;
      primary = true;
    }
    if (version === 2 && entry.worldId !== undefined) {
      if (piece.kind !== 'ecology' || (entry.worldId !== null && (typeof entry.worldId !== 'string' || !/^[a-z0-9_-]{1,80}$/i.test(entry.worldId)))) {
        throw new Error('生态组件绑定的世界无效');
      }
      piece.worldId = entry.worldId;
    }
    if (version === 2 && entry.scalePercent !== undefined) {
      if (typeof entry.scalePercent !== 'number' || !Number.isInteger(entry.scalePercent)
        || entry.scalePercent < pieceScaleLimits.minimum || entry.scalePercent > pieceScaleLimits.maximum) {
        throw new Error('组件尺寸无效');
      }
      piece.scalePercent = entry.scalePercent;
    }
    if (!contains(desktop, pieceBox(piece))) throw new Error('组件位置超出布局范围');
    ids.add(piece.id);
    return piece;
  });

  const idleConfig: WorkspaceLayout['idle'] = { mode: idle.mode, text: idle.text };
  if (version === 1) return migrate(items, idleConfig);
  const mode = source.mode;
  if (!isMode(mode)) throw new Error('无法读取组件组合方式');
  const host = ecologyAnchor(items);
  const hostRect = host && pieceBox(host);
  for (const piece of items) {
    if (piece.docked && (!hostRect || piece.kind === 'ecology' || mode === 'free' || !contains(hostRect, pieceBox(piece)))) {
      throw new Error('叠入生态画面的组件位置无效');
    }
    if (mode === 'combined' && host && piece.kind !== 'ecology' && !piece.docked) {
      throw new Error('组合式布局包含未叠入生态画面的组件');
    }
  }
  const panelWorldId = typeof source.panelWorldId === 'string' && /^[a-z0-9_-]{1,80}$/i.test(source.panelWorldId)
    ? source.panelWorldId : undefined;
  const panelPlacement = readPanelPlacement(source.panelPlacement);
  return { version: 2, mode, items, idle: idleConfig, ...(panelWorldId ? { panelWorldId } : {}),
    ...(panelPlacement ? { panelPlacement } : {}) };
}
