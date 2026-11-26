import { useRef, useState, type KeyboardEvent } from 'react';
import type { PondBasinSnapshot, PondEdit, PondSedgePatch, PondSedgePreset, PondShapeSnapshot, PondShoreAnchor } from '../../world';
import { matterSummary } from './formatMatter';
import './PondShapeEditor.css';
import './PondSedgeEditor.css';

const sizes = { small: '小丛 · 150 mm', medium: '中丛 · 220 mm', large: '大丛 · 300 mm' } satisfies Record<PondSedgePreset, string>;
const maximumPatches = 8;

export function PondSedgeCard({ enabled, open }: { enabled: boolean; open: () => void }) {
  return <button type="button" className="editor-object" aria-label="编辑岸边莎草" data-pond-sedge-open disabled={!enabled} onClick={open}>
    <span className="pond-sedge-icon" aria-hidden="true"><i /><i /><i /><i /><i /></span>
    <span><strong>岸边莎草</strong><small>沿岸种植 · 根区随水位变化</small></span>
  </button>;
}

export function SedgeReadout({ patch }: { patch: PondSedgePatch }) {
  const fraction = patch.rootWetFraction;
  return <div className="pond-sedge-readout" data-pond-sedge-readout={patch.id} data-pond-sedge-root-wet={fraction}>
    <strong>岸边莎草 {patch.id}</strong>
    <small>株高 {patch.shootHeightMm} mm · 根团直径 {patch.rootDiameterMm} mm</small>
    <small>根区浸水 {Math.round(fraction * 100)}% · {fraction > 0 ? '可从水中吸收养分' : '根区休眠'}</small>
    <small>{matterSummary(patch.stockMgCnp)}</small>
  </div>;
}

function neighbors(cell: number, columns: number, rows: number) {
  const row = Math.floor(cell / columns);
  const column = cell % columns;
  return [row > 0 ? cell - columns : -1, row + 1 < rows ? cell + columns : -1,
    column > 0 ? cell - 1 : -1, column + 1 < columns ? cell + 1 : -1].filter(value => value >= 0);
}

export function PondSedgeEditor({ basin, basins, shape, disabled, selectedId, onSelect, onEdit, onClose }: {
  basin: PondBasinSnapshot;
  basins: readonly PondBasinSnapshot[];
  shape: PondShapeSnapshot;
  disabled: boolean;
  selectedId?: number;
  onSelect: (id: number) => void;
  onEdit: (edit: PondEdit) => Promise<boolean>;
  onClose: () => void;
}) {
  const [mode, setMode] = useState<'add' | 'move'>('add');
  const [preset, setPreset] = useState<PondSedgePreset>('medium');
  const [focused, setFocused] = useState<number>();
  const [confirmRemoval, setConfirmRemoval] = useState(false);
  const grid = useRef<HTMLDivElement>(null);
  const inFlight = useRef(false);
  const plants = basin.shoreSedges ?? [];
  const selected = plants.find(({ id }) => id === selectedId) ?? plants.at(-1);
  const currentCells = new Set(shape.cells);
  const otherBasins = basins.filter(({ id }) => id !== basin.id);
  const otherCells = new Set(otherBasins.flatMap(({ shape }) => shape?.cells ?? []));
  const otherSedges = new Set(otherBasins.flatMap(({ shoreSedges }) => shoreSedges?.map(({ anchor }) => anchor.shoreCell) ?? []));
  const byShore = new Map(plants.map(plant => [plant.anchor.shoreCell, plant]));
  const { columns, rows } = shape;
  const count = columns * rows;
  const full = plants.length >= maximumPatches;
  const cells = Array.from({ length: count }, (_, cell) => cell);
  const possible = new Map<number, PondShoreAnchor>();
  for (const shoreCell of cells) {
    if (currentCells.has(shoreCell) || otherCells.has(shoreCell) || otherSedges.has(shoreCell)) continue;
    const edge = neighbors(shoreCell, columns, rows).filter(cell => currentCells.has(cell));
    if (!edge.length) continue;
    edge.sort((a, b) => Math.abs(Math.floor(a / columns) - shape.protectedRow) - Math.abs(Math.floor(b / columns) - shape.protectedRow) || a - b);
    possible.set(shoreCell, { waterCell: edge[0], shoreCell });
  }
  const focusCell = focused !== undefined && possible.has(focused) ? focused : possible.keys().next().value;

  function navigate(event: KeyboardEvent<HTMLDivElement>) {
    const offsets: Record<string, readonly [number, number]> = {
      ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1],
    };
    const offset = offsets[event.key];
    if (!offset || focusCell === undefined) return;
    event.preventDefault();
    let x = focusCell % columns + offset[0];
    let y = Math.floor(focusCell / columns) + offset[1];
    while (x >= 0 && x < columns && y >= 0 && y < rows) {
      const cell = y * columns + x;
      if (possible.has(cell)) {
        grid.current?.querySelector<HTMLButtonElement>(`[data-pond-sedge-cell="${cell}"]`)?.focus();
        break;
      }
      x += offset[0];
      y += offset[1];
    }
  }

  async function submit(edit: PondEdit) {
    if (disabled || inFlight.current) return;
    inFlight.current = true;
    try { if (await onEdit(edit)) setConfirmRemoval(false); }
    finally { inFlight.current = false; }
  }

  function choose(cell: number) {
    const occupant = byShore.get(cell);
    if (occupant) { onSelect(occupant.id); return; }
    const anchor = possible.get(cell);
    if (!anchor) return;
    if (mode === 'add' && !full) void submit({ type: 'addSedge', basinId: basin.id, anchor, preset });
    if (mode === 'move' && selected) void submit({ type: 'moveSedge', basinId: basin.id, id: selected.id, anchor });
  }

  return <section className="pond-shape pond-sedge" aria-label="岸边莎草俯视编辑" data-pond-sedge-editor>
    <div className="pond-shape__map-slot">
      <div ref={grid} className="pond-shape__map" aria-label="当前水域和岸边" onKeyDown={navigate}
        style={{ gridTemplateColumns: `repeat(${columns}, 1fr)`, gridTemplateRows: `repeat(${rows}, 1fr)` }}>
        {cells.map(cell => {
          const patch = byShore.get(cell);
          const anchor = possible.get(cell);
          if (!anchor) return <span key={cell} className="pond-shape__cell" data-shape-state={currentCells.has(cell) ? 'water' : otherCells.has(cell) ? 'other' : 'land'} />;
          return <button key={cell} type="button" className="pond-shape__cell pond-sedge__bank"
            tabIndex={focusCell === cell ? 0 : -1} data-pond-sedge-cell={cell} data-pond-sedge-water-cell={anchor.waterCell}
            data-pond-sedge-id={patch?.id} data-pond-sedge-selected={patch && patch.id === selected?.id || undefined}
            aria-label={patch ? `选择莎草 ${patch.id}` : `岸边第 ${Math.floor(cell / columns) + 1} 行第 ${cell % columns + 1} 列`}
            onFocus={() => setFocused(cell)} onClick={() => choose(cell)}>{patch?.id}</button>;
        })}
      </div>
    </div>
    <div className="pond-shape__controls">
      <header className="pond-shape__header"><h2>岸边莎草</h2><button type="button" className="editor-tool" onClick={onClose}>返回场景</button></header>
      <div className="pond-shape__tools">
        <button type="button" className="editor-tool" aria-pressed={mode === 'add'} onClick={() => setMode('add')}>种植</button>
        <button type="button" className="editor-tool" aria-pressed={mode === 'move'} disabled={!selected} onClick={() => setMode('move')}>移动所选</button>
      </div>
      {mode === 'add' && <label className="editor-pond-field">规格
        <select data-pond-sedge-size value={preset} disabled={disabled} onChange={event => setPreset(event.currentTarget.value as PondSedgePreset)}>
          {Object.entries(sizes).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
        </select>
      </label>}
      <p className="pond-shape__help">{full && mode === 'add' ? '当前水域已达到八丛。' : '浅绿色为可用岸格，蓝色格是当前水域。可用方向键和回车选落点。'}</p>
      <p className="pond-shape__help">植株固定在岸边。根区接触池水后可吸收水中养分，叶片主要与空气交换。</p>
      {selected && <>
        <SedgeReadout patch={selected} />
        {confirmRemoval ? <div className="pond-shape__tools">
          <button type="button" className="editor-tool" data-pond-sedge-remove-confirm disabled={disabled} onClick={() => void submit({ type: 'removeSedge', basinId: basin.id, id: selected.id })}>移出世界</button>
          <button type="button" className="editor-tool" onClick={() => setConfirmRemoval(false)}>取消</button>
        </div> : <button type="button" className="editor-tool" data-pond-sedge-remove disabled={disabled} onClick={() => setConfirmRemoval(true)}>移除所选莎草</button>}
      </>}
    </div>
  </section>;
}
