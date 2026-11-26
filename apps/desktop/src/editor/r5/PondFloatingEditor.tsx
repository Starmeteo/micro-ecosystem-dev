import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent, MouseEvent } from 'react';
import floatingLeaf from '../../render/assets/r5/pond-composed/floating-45.png';
import type { PondBasinSnapshot, PondEdit, PondFloatingPlantPreset, PondShapeSnapshot, ScenePoint } from '../../world';
import { displayMass, matterSummary } from './formatMatter';
import './PondShapeEditor.css';
import './PondFloatingEditor.css';

const diameters = { small: 30, medium: 45, large: 60 } as const;
const sizes = Object.keys(diameters) as PondFloatingPlantPreset[];
const maxFloatingPlants = 8;
const percent = new Intl.NumberFormat('zh-CN', { style: 'percent', maximumFractionDigits: 2 });

export function PondFloatingCard({ basin, enabled, open }: {
  basin: PondBasinSnapshot;
  enabled: boolean;
  open: () => void;
}) {
  const count = basin.floatingPlants?.length ?? 0;
  return <button type="button" className="editor-object" aria-label="编辑活体浮叶" data-pond-floating-open
    disabled={!enabled || (!basin.wet && !count)} onClick={open}>
    <img src={floatingLeaf} alt="" width="44" height="32" />
    <span><strong>活体浮叶</strong><small>{basin.wet ? '水面俯视摆放 · 计算重叠遮光' : count ? '已搁浅 · 可调整位置' : '补水后可投放'}</small></span>
  </button>;
}

export function PondFloatingReadout({ basin }: { basin: PondBasinSnapshot }) {
  const canopy = basin.floatingCanopy;
  const coverage = canopy?.coverage ?? 0;
  const transmission = canopy?.lightTransmission ?? 1;
  return <dl className="editor-stock pond-shape__metrics">
    <div><dt>水面覆盖</dt><dd data-pond-floating-coverage={coverage}>{percent.format(coverage)}</dd></div>
    <div><dt>遮光后入水比例</dt><dd data-pond-floating-transmission={transmission}>{basin.wet ? percent.format(transmission) : '—'}</dd></div>
    <div><dt>覆盖面积</dt><dd data-pond-floating-area={canopy?.coveredAreaCm2 ?? 0}>{displayMass(canopy?.coveredAreaCm2 ?? 0)} cm²</dd></div>
  </dl>;
}

export function PondFloatingEditor({ basin, basins, shape, disabled, selectedId, onSelect, onEdit, onClose }: {
  basin: PondBasinSnapshot;
  basins: readonly PondBasinSnapshot[];
  shape: PondShapeSnapshot;
  disabled: boolean;
  selectedId?: number;
  onSelect: (id: number) => void;
  onEdit: (edit: PondEdit) => Promise<boolean>;
  onClose: () => void;
}) {
  const plants = basin.floatingPlants ?? [];
  const [mode, setMode] = useState<'add' | 'move'>(basin.wet ? 'add' : 'move');
  const [preset, setPreset] = useState<PondFloatingPlantPreset>('medium');
  const [focused, setFocused] = useState(shape.protectedRow * shape.columns + Math.floor(shape.columns / 2));
  const [confirmRemoval, setConfirmRemoval] = useState(false);
  const grid = useRef<HTMLDivElement>(null);
  const inFlight = useRef(false);
  const selected = plants.find(({ id }) => id === selectedId) ?? plants.at(-1);
  const waterCells = new Set(shape.cells);
  const otherCells = new Set(basins.filter(({ id }) => id !== basin.id).flatMap(({ shape }) => shape?.cells ?? []));
  const full = plants.length >= maxFloatingPlants;
  const focusCell = waterCells.has(focused) ? focused : shape.cells[0];
  const cellSideMm = 10 * Math.sqrt(shape.cellAreaCm2);

  useEffect(() => {
    grid.current?.querySelector<HTMLButtonElement>('button[tabindex="0"]')?.focus({ preventScroll: true });
  }, []);

  async function submit(edit: PondEdit) {
    if (disabled || inFlight.current) return;
    inFlight.current = true;
    try {
      if (!await onEdit(edit)) return;
      setConfirmRemoval(false);
    } finally {
      inFlight.current = false;
    }
  }

  function chooseSite(event: MouseEvent<HTMLButtonElement>, cell: number) {
    if (disabled || !waterCells.has(cell)) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const relative = (pixel: number, start: number, length: number) =>
      event.detail > 0 && length > 0 ? Math.max(.001, Math.min(.999, (pixel - start) / length)) : .5;
    const position: ScenePoint = {
      x: (cell % shape.columns + relative(event.clientX, rect.left, rect.width)) / shape.columns,
      y: (Math.floor(cell / shape.columns) + relative(event.clientY, rect.top, rect.height)) / shape.rows,
    };
    if (mode === 'add' && basin.wet && !full) {
      void submit({ type: 'addFloatingPlant', basinId: basin.id, preset, position });
    } else if (mode === 'move' && selected) {
      void submit({ type: 'moveFloatingPlant', basinId: basin.id, id: selected.id, position });
    }
  }

  function navigate(event: KeyboardEvent<HTMLDivElement>) {
    if ((event.key === 'Enter' || event.key === ' ') && event.target instanceof HTMLButtonElement) {
      event.preventDefault();
      if (!event.repeat) event.target.click();
      return;
    }
    const directions: Record<string, readonly [number, number]> = {
      ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1],
    };
    const direction = directions[event.key];
    if (!direction || focusCell === undefined) return;
    event.preventDefault();
    let column = focusCell % shape.columns + direction[0];
    let row = Math.floor(focusCell / shape.columns) + direction[1];
    while (column >= 0 && column < shape.columns && row >= 0 && row < shape.rows) {
      const cell = row * shape.columns + column;
      if (waterCells.has(cell)) {
        grid.current?.querySelector<HTMLButtonElement>(`[data-pond-floating-cell="${cell}"]`)?.focus();
        return;
      }
      column += direction[0];
      row += direction[1];
    }
  }

  return <section className="pond-shape pond-floating" aria-label="活体浮叶俯视编辑" data-pond-floating-editor
    data-pond-floating-mode={mode} data-pond-floating-selected={selected?.id}>
    <div className="pond-shape__map-slot">
      <div ref={grid} className="pond-shape__map pond-floating__map" role="group" aria-label="水面格网"
        aria-describedby="pond-floating-help" onKeyDown={navigate}
        style={{ gridTemplateColumns: `repeat(${shape.columns}, 1fr)`, gridTemplateRows: `repeat(${shape.rows}, 1fr)` }}>
        {Array.from({ length: shape.columns * shape.rows }, (_, cell) => {
          const row = Math.floor(cell / shape.columns);
          const column = cell % shape.columns;
          const state = waterCells.has(cell) ? row === shape.protectedRow ? 'section' : 'water' : otherCells.has(cell) ? 'other' : 'land';
          const label = `第 ${row + 1} 行，第 ${column + 1} 格，${basin.wet ? '当前水域' : '已干涸底床'}`;
          return waterCells.has(cell)
            ? <button key={cell} type="button" className="pond-shape__cell" data-shape-state={state}
              aria-label={label} aria-disabled={disabled || (mode === 'add' ? !basin.wet || full : !selected)}
              data-pond-floating-cell={cell} tabIndex={focusCell === cell ? 0 : -1}
              onFocus={() => setFocused(cell)} onClick={(event) => chooseSite(event, cell)} />
            : <span key={cell} aria-hidden="true" className="pond-shape__cell" data-shape-state={state} />;
        })}
        {plants.map((plant) => <span key={plant.id} aria-hidden="true" className="pond-floating__marker"
          data-pond-floating-marker={plant.id} data-selected={plant.id === selected?.id || undefined}
          data-stranded={!plant.floating || undefined}
          style={{ left: `${plant.position.x * 100}%`, top: `${plant.position.y * 100}%`,
            width: `${plant.diameterMm * 100 / (cellSideMm * shape.columns)}%`,
            height: `${plant.diameterMm * 100 / (cellSideMm * shape.rows)}%` }} />)}
      </div>
    </div>
    <div className="pond-shape__controls">
      <header className="pond-shape__header">
        <h2>活体浮叶</h2>
        <button className="editor-tool" type="button" onClick={onClose}>返回场景</button>
      </header>
      <div className="pond-shape__tools" role="group" aria-label="浮叶摆放">
        <button className="editor-tool" type="button" aria-pressed={mode === 'add'} data-pond-floating-add-mode
          disabled={!basin.wet || full || disabled} onClick={() => { setMode('add'); setConfirmRemoval(false); }}>放置浮叶</button>
        <button className="editor-tool" type="button" aria-pressed={mode === 'move'} data-pond-floating-move-mode
          disabled={!selected || disabled} onClick={() => { setMode('move'); setConfirmRemoval(false); }}>移动浮叶</button>
      </div>
      {mode === 'add' && <label className="editor-pond-field">浮叶规格
        <select value={preset} data-pond-floating-preset disabled={!basin.wet || full || disabled}
          onChange={(event) => setPreset(event.currentTarget.value as PondFloatingPlantPreset)}>
          {sizes.map((size) => <option key={size} value={size}>叶径 {diameters[size]} mm</option>)}
        </select>
      </label>}
      {selected && <label className="editor-pond-field">选择浮叶
        <select value={selected.id} data-pond-floating-selection disabled={disabled} onChange={(event) => {
          onSelect(Number(event.currentTarget.value));
          setConfirmRemoval(false);
        }}>
          {plants.map((plant) => <option key={plant.id} value={plant.id}>
            浮叶 {plant.id} · {diameters[plant.preset]} mm
          </option>)}
        </select>
      </label>}
      <p id="pond-floating-help" className="pond-shape__help">{!basin.wet ? '已干涸，保留现有浮叶库存，补水后可放置。'
        : mode === 'add' && full ? `当前水域已达到 ${maxFloatingPlants} 组浮叶。`
        : '点击水格，或用方向键选格后按 Enter。重叠只计一次，岸地不计遮光。'}</p>
      <PondFloatingReadout basin={basin} />
      {selected && <>
        <p className="pond-shape__help" data-pond-floating-stock={selected.stockMgCnp.join(',')}>
          浮叶 {selected.id} · 叶径 {displayMass(selected.diameterMm)} mm · {matterSummary(selected.stockMgCnp)}
        </p>
        {confirmRemoval ? <div className="pond-shape__tools">
          <button className="editor-tool" type="button" data-pond-floating-remove-confirm disabled={disabled}
            onClick={() => void submit({ type: 'removeFloatingPlant', basinId: basin.id, id: selected.id })}>移除并转入碎屑</button>
          <button className="editor-tool" type="button" onClick={() => setConfirmRemoval(false)}>取消</button>
        </div> : <button className="editor-tool pond-floating__remove" type="button" data-pond-floating-remove disabled={disabled}
          onClick={() => setConfirmRemoval(true)}>移除所选浮叶</button>}
      </>}
    </div>
  </section>;
}
