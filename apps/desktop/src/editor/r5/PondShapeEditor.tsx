import { type KeyboardEvent, type PointerEvent, useEffect, useRef, useState } from 'react';
import type { PondBasinSnapshot, PondShapeMode, PondShapeSnapshot } from '../../world';
import { floatingPlantCell, pondProtectedRows, previewPondTopology } from './pondTopologyPreview';
import { PondTopologyResults } from './PondTopologyResults';
import './PondShapeEditor.css';

const areaLimit = { min: 400, max: 10000 };
const significant = new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 2 });
const directions: Record<string, readonly [number, number]> = {
  ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1],
};

function neighbors(cell: number, { columns, rows }: PondShapeSnapshot) {
  const x = cell % columns;
  return [
    ...(x > 0 ? [cell - 1] : []), ...(x < columns - 1 ? [cell + 1] : []),
    ...(cell >= columns ? [cell - columns] : []), ...(cell < columns * (rows - 1) ? [cell + columns] : []),
  ];
}

function linked(cells: Set<number>, shape: PondShapeSnapshot) {
  const first = cells.values().next().value;
  if (first === undefined) return false;
  const visited = new Set([first]);
  for (const cell of visited) {
    for (const other of neighbors(cell, shape)) if (cells.has(other)) visited.add(other);
  }
  return visited.size === cells.size;
}

function traceStroke(cells: Set<number>, from: number, to: number, columns: number) {
  let x = from % columns;
  let y = Math.floor(from / columns);
  const targetX = to % columns;
  const targetY = Math.floor(to / columns);
  const stepsX = Math.abs(targetX - x);
  const stepsY = Math.abs(targetY - y);
  let traversedX = 0;
  let traversedY = 0;
  cells.add(from);
  while (x !== targetX || y !== targetY) {
    const nextVertical = (2 * traversedX + 1) * stepsY;
    const nextHorizontal = (2 * traversedY + 1) * stepsX;
    if (x !== targetX && (y === targetY || nextVertical <= nextHorizontal)) {
      x += Math.sign(targetX - x);
      traversedX++;
    } else {
      y += Math.sign(targetY - y);
      traversedY++;
    }
    cells.add(y * columns + x);
  }
}

function preview(shape: PondShapeSnapshot, occupied: Set<number>, otherBasins: Set<number>, mode: PondShapeMode,
  touched: readonly number[], water: number, maxDepth: number, floatingCells: ReadonlySet<number>,
  protectedRows: ReadonlySet<number>) {
  const changed = touched.filter(cell => occupied.has(cell) === (mode === 'remove'));
  const proposed = new Set(occupied);
  for (const cell of changed) {
    if (mode === 'add') proposed.add(cell);
    else proposed.delete(cell);
  }
  const area = proposed.size * shape.cellAreaCm2;
  const capacity = area * maxDepth / 1000;
  const depth = area ? water * 1000 / area : 0;
  let reason = '';
  if (mode === 'remove' && changed.some(cell => protectedRows.has(Math.floor(cell / shape.columns)))) {
    reason = '请保留虚线标记的主、副观景剖面整行。';
  } else if (mode === 'remove' && changed.some(cell => floatingCells.has(cell))) {
    reason = '准备收回的格中有活体浮叶。请先移动或移除浮叶。';
  } else if (changed.length && !linked(proposed, shape)) {
    reason = '这一笔会留下分离的水格。水格必须通过共边连在一起。';
  } else if (changed.length && [...proposed].some(cell => otherBasins.has(cell) || neighbors(cell, shape).some(other => otherBasins.has(other)))) {
    reason = '这一笔会接触另一个水域。请在水域之间留一格陆地。';
  } else if (area < areaLimit.min || area > areaLimit.max) {
    reason = '水面面积须在 400 至 10,000 cm² 之间。';
  } else if (water > capacity) {
    reason = '收岸后容不下现有水量。可以先排水，再缩小池岸。';
  }
  return { changed, area, capacity, depth, reason };
}

type Stroke = { pointer: number; revision: string; mode: PondShapeMode; last: number | null; cells: Set<number> };

export function PondShapeEditor({ basin, shape, basins, revision, disabled, onPaint, onApplyTopology, onClose }: {
  basin: PondBasinSnapshot;
  shape: PondShapeSnapshot;
  basins: readonly PondBasinSnapshot[];
  revision: string;
  disabled: boolean;
  onPaint: (mode: PondShapeMode, cells: number[]) => void;
  onApplyTopology?: (mode: PondShapeMode, cells: number[]) => Promise<boolean>;
  onClose: () => void;
}) {
  const grid = useRef<HTMLDivElement>(null);
  const stroke = useRef<Stroke | null>(null);
  const applyInFlight = useRef(false);
  const [mode, setMode] = useState<PondShapeMode>('add');
  const [topologyMode, setTopologyMode] = useState(false);
  const [focused, setFocused] = useState(Math.max(0, shape.protectedRow - 2) * shape.columns + Math.floor(shape.columns / 2));
  const [touched, setTouched] = useState<number[]>([]);
  const [savedDraft, setSavedDraft] = useState<number[]>([]);
  const [note, setNote] = useState('');
  const [applying, setApplying] = useState(false);
  const busy = disabled || applying;
  const occupied = new Set(shape.cells);
  const protectedRows = pondProtectedRows(basin, shape);
  const floatingCells = new Set(basin.floatingPlants?.map(leaf => floatingPlantCell(leaf, shape)));
  const otherBasins = new Set(basins.flatMap(other => other.id === basin.id ? [] : other.shape?.cells ?? []));
  const proposal = preview(shape, occupied, otherBasins, mode, touched, basin.waterLiters, basin.maxDepthCm, floatingCells, protectedRows);
  const topologyCells = topologyMode ? [...new Set([...savedDraft, ...touched])] : [];
  const topology = topologyCells.length ? previewPondTopology(basin, shape, basins, mode, topologyCells) : null;
  const previewRegions = new Map(topology?.components.flatMap(({ cells }, index) =>
    cells.map((cell, offset) => [cell, { index, first: offset === 0 }] as const)));
  const previewsSectionRouting = topology?.kind === 'merge' || topology?.kind === 'split';
  const floatingCounts = new Map<number, number>();
  if (topologyMode) {
    for (const water of basins) {
      for (const leaf of water.floatingPlants ?? []) {
        const cell = floatingPlantCell(leaf, shape);
        floatingCounts.set(cell, (floatingCounts.get(cell) ?? 0) + 1);
      }
    }
  }

  function clear() {
    const current = stroke.current;
    stroke.current = null;
    if (current && grid.current?.hasPointerCapture(current.pointer)) grid.current.releasePointerCapture(current.pointer);
    setTouched([]);
  }

  useEffect(() => { clear(); setNote(''); setSavedDraft([]); }, [revision, basin.id, mode, topologyMode]);
  useEffect(() => { if (busy) clear(); }, [busy]);
  useEffect(() => { grid.current?.querySelector<HTMLButtonElement>('button[tabindex="0"]')?.focus({ preventScroll: true }); }, []);

  function point(event: PointerEvent<HTMLDivElement>) {
    const { left, top, width, height } = event.currentTarget.getBoundingClientRect();
    if (!width || !height) return null;
    const x = Math.floor((event.clientX - left) * shape.columns / width);
    const y = Math.floor((event.clientY - top) * shape.rows / height);
    if (x < 0 || x >= shape.columns || y < 0 || y >= shape.rows) return null;
    return y * shape.columns + x;
  }

  function focusCell(cell: number) {
    setFocused(cell);
    grid.current?.querySelector<HTMLButtonElement>(`button[data-pond-shape-cell="${cell}"]`)?.focus({ preventScroll: true });
  }

  function start(event: PointerEvent<HTMLDivElement>) {
    if (busy || stroke.current || !event.isPrimary || event.button !== 0) return;
    const cell = point(event);
    if (cell === null) return;
    event.preventDefault();
    focusCell(cell);
    event.currentTarget.setPointerCapture(event.pointerId);
    stroke.current = { pointer: event.pointerId, revision, mode, last: cell, cells: new Set([cell]) };
    setTouched([cell]);
    setNote('');
  }

  function extend(current: Stroke, cell: number | null) {
    if (cell !== null) traceStroke(current.cells, current.last ?? cell, cell, shape.columns);
    current.last = cell;
  }

  function update(event: PointerEvent<HTMLDivElement>) {
    const current = stroke.current;
    if (busy || !current || current.pointer !== event.pointerId || current.revision !== revision || current.mode !== mode) return;
    const before = current.cells.size;
    extend(current, point(event));
    if (current.cells.size !== before) setTouched([...current.cells]);
  }

  function commit(cells: number[]) {
    if (topologyMode) {
      setNote('');
      setSavedDraft(previous => [...new Set([...previous, ...cells])]);
      return;
    }
    const result = preview(shape, occupied, otherBasins, mode, cells, basin.waterLiters, basin.maxDepthCm, floatingCells, protectedRows);
    if (result.reason) { setNote(result.reason); return; }
    if (!result.changed.length) { setNote('这一笔没有改变水域。'); return; }
    onPaint(mode, result.changed);
  }

  function finish(event: PointerEvent<HTMLDivElement>) {
    const current = stroke.current;
    if (!current || current.pointer !== event.pointerId) return;
    if (!busy && current.revision === revision && current.mode === mode) {
      extend(current, point(event));
      commit([...current.cells]);
    }
    clear();
  }

  function cancel(event: PointerEvent<HTMLDivElement>) {
    if (stroke.current?.pointer === event.pointerId) clear();
  }

  function escape(event: KeyboardEvent<HTMLElement>) {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    event.stopPropagation();
    if (applyInFlight.current) return;
    clear();
    if (topologyMode) setTopologyMode(false);
    else onClose();
  }

  function keyboard(event: KeyboardEvent<HTMLDivElement>) {
    if (busy || stroke.current) return;
    const direction = directions[event.key];
    if (direction) {
      const x = Math.max(0, Math.min(shape.columns - 1, focused % shape.columns + direction[0]));
      const y = Math.max(0, Math.min(shape.rows - 1, Math.floor(focused / shape.columns) + direction[1]));
      focusCell(y * shape.columns + x);
      event.preventDefault();
    } else if ((event.key === ' ' || event.key === 'Enter') && !event.repeat) {
      event.preventDefault();
      commit([focused]);
    }
  }

  async function applyTopology() {
    if (!onApplyTopology || !topology?.canApply || busy || stroke.current || applyInFlight.current) return;
    applyInFlight.current = true;
    setApplying(true);
    setNote('');
    let applied = false;
    try {
      applied = await onApplyTopology(mode, topology.changedCells);
    } catch {
      // Preserve the draft so the same edit can be retried after a transport failure.
    } finally {
      applyInFlight.current = false;
      setApplying(false);
    }
    if (applied) {
      setSavedDraft([]);
      setNote('水域布局已更新，可撤销。');
    } else {
      setNote('预览已保留，可以调整后重试。');
    }
  }

  const pendingCells = new Set(topologyMode ? topology?.changedCells : proposal.changed);
  return <section className="pond-shape" aria-label="浅塘池岸俯视编辑" onKeyDown={escape}
    data-pond-shape-editor data-pond-topology-preview={topologyMode || undefined}>
    <div className="pond-shape__map-slot">
      <div ref={grid} className="pond-shape__map" role="group" aria-label="池岸格网" aria-describedby="pond-shape-help"
        style={{ gridTemplateColumns: `repeat(${shape.columns}, 1fr)`, gridTemplateRows: `repeat(${shape.rows}, 1fr)` }}
        onPointerDown={start} onPointerMove={update} onPointerUp={finish} onPointerCancel={cancel} onLostPointerCapture={cancel}
        onKeyDown={keyboard} data-current-cells={shape.cells.join(',')}>
        {Array.from({ length: shape.columns * shape.rows }, (_, cell) => {
          const column = cell % shape.columns;
          const row = Math.floor(cell / shape.columns);
          const region = previewRegions.get(cell);
          const previewSection = previewsSectionRouting && region ? topology?.components[region.index] : undefined;
          const primary = row === (previewSection ? previewSection.observationRow : shape.protectedRow);
          const secondary = previewSection ? previewSection.staticSectionRows.includes(row) : protectedRows.has(row) && !primary;
          const state = primary || secondary ? 'section' : otherBasins.has(cell) ? 'other' : occupied.has(cell) ? 'water' : 'land';
          const label = secondary ? '保留的副观景剖面'
            : { section: '保留的主观景剖面', other: '其他水域', water: '当前水域', land: '岸地' }[state];
          const invalid = topologyMode ? mode === 'remove' && (protectedRows.has(row) || floatingCells.has(cell)) : Boolean(proposal.reason);
          const planned = pendingCells.has(cell) ? invalid ? 'invalid' : mode : undefined;
          const regionLabel = region ? `，预览水域 ${region.index + 1}` : '';
          const leaves = floatingCounts.get(cell);
          const floatingLabel = leaves ? `，${leaves} 片活体浮叶` : '';
          return <button key={cell} type="button" className="pond-shape__cell" aria-label={`第 ${row + 1} 行，第 ${column + 1} 格，${label}${regionLabel}${floatingLabel}`}
            tabIndex={focused === cell ? 0 : -1} onFocus={() => setFocused(cell)} data-pond-shape-cell={cell}
            data-shape-state={state} data-shape-preview={planned} data-topology-tone={region ? region.index % 4 : undefined}
            data-pond-topology-floating-cell={leaves} data-pond-topology-secondary-row={secondary ? row : undefined}>
            {region?.first && <span aria-hidden="true" className="pond-topology__pin">{region.index + 1}</span>}
            {leaves && <span aria-hidden="true" className="pond-topology__float">{leaves > 1 ? leaves : ''}</span>}
          </button>;
        })}
      </div>
    </div>
    <div className="pond-shape__controls">
      <header className="pond-shape__header"><h2>池岸笔刷</h2><button className="editor-tool" type="button" disabled={applying}
        onClick={onClose}>返回场景</button></header>
      <div className="pond-shape__tools" role="group" aria-label="笔刷动作">
        <button className="editor-tool" type="button" disabled={busy} aria-pressed={topologyMode}
          onClick={() => setTopologyMode(value => !value)}>分合预览</button>
        <button className="editor-tool" type="button" disabled={busy} aria-pressed={mode === 'add'} onClick={() => setMode('add')}>{topologyMode ? '画水道' : '扩展水域'}</button>
        <button className="editor-tool" type="button" disabled={busy} aria-pressed={mode === 'remove'} onClick={() => setMode('remove')}>{topologyMode ? '填岸地' : '收回水域'}</button>
        {topologyMode && <button className="editor-tool" type="button" disabled={busy || !topologyCells.length}
          onClick={() => { clear(); setNote(''); setSavedDraft([]); }}>清除预览</button>}
      </div>
      <p id="pond-shape-help" className="pond-shape__help">{topologyMode
        ? '拖画或按方向键选格后按 Enter，可分多笔。检查水量和生物去向后应用布局，切换画法会清除草稿。'
        : `俯视画格，每格 ${significant.format(shape.cellAreaCm2)} cm²。虚线标记保留的剖面。拖动或用方向键选格后按 Enter。`}</p>
      {topologyMode ? <>
        {topology && <PondTopologyResults result={topology} action={onApplyTopology && (topology.kind === 'split' || topology.kind === 'merge')
          ? <button className="editor-tool pond-topology__apply" type="button" data-pond-topology-apply
            disabled={!topology.canApply || busy || Boolean(stroke.current)} onClick={() => void applyTopology()}>
            {applying ? '正在应用…' : topology.kind === 'split' ? '应用分池' : '应用合池'}
          </button> : undefined} />}
        {note && <p className="pond-shape__status" role="status">{note}</p>}
      </> : <>
        <dl className="editor-stock pond-shape__metrics">
        <div><dt>水面面积</dt><dd data-pond-shape-area={proposal.area}>{significant.format(proposal.area)} cm²</dd></div>
        <div><dt>平均水深</dt><dd data-pond-shape-depth={proposal.depth}>{significant.format(proposal.depth)} cm</dd></div>
        <div><dt>水量</dt><dd data-pond-shape-water={basin.waterLiters}>{significant.format(basin.waterLiters)} L</dd></div>
        <div><dt>池岸容量</dt><dd>{significant.format(proposal.capacity)} L</dd></div>
        </dl>
        <p className="pond-shape__status" role="status" data-pond-shape-error={Boolean(proposal.reason || note) || undefined}>
          {proposal.reason || note || (touched.length ? `预览 ${proposal.changed.length} 格，松开提交。` : '一笔可撤销，水量保持不变。')}
        </p>
      </>}
    </div>
  </section>;
}
