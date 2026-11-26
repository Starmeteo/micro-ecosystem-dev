import { useState } from 'react';
import type { ForestSubstrate, ForestSubstrateCell, ForestSubstratePreset } from '../../world';
import { forestSoilColumns } from '../../render/r5/forestPlacement';
import { displayMass } from './formatMatter';

const names: Record<ForestSubstrate, string> = { native: '原底土', fineSoil: '细土', coarseHumus: '粗腐殖质' };
const format = new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 2 });

export function forestSubstrateName(substrate: ForestSubstrate) {
  return names[substrate];
}

export function ForestSubstrateCards({ presets, editable, onStart }: {
  presets: readonly ForestSubstratePreset[];
  editable: boolean;
  onStart: (substrate: ForestSubstrate) => void;
}) {
  return <>
    <h3 className="editor-forest-drawer-heading">土壤基质</h3>
    <div className="editor-content-options">
      {presets.map(({ substrate, waterCapacityMl, evaporationMultiplier }) => (
        <button key={substrate} className="editor-object" type="button" data-forest-substrate-brush={substrate}
          disabled={!editable} onClick={() => onStart(substrate)}>
          <span className="editor-substrate-swatch" data-substrate={substrate} aria-hidden />
          <span><strong>{substrate === 'native' ? '恢复原底土' : '刷' + names[substrate]}</strong>
            <small>每格 {waterCapacityMl} mL · 蒸发 ×{format.format(evaporationMultiplier)}</small>
          </span>
        </button>
      ))}
    </div>
    <p className="editor-context__hint">更换土格的储水上限和基础蒸发倍率，保留格内的水和养分。容量不足时整笔取消。枯叶投放和活体苔藓有各自的工具。</p>
  </>;
}

export function ForestSubstrateReadings({ cells }: { cells: readonly ForestSubstrateCell[] }) {
  const [selected, select] = useState<number | undefined>();
  const cell = cells.find(({ cellIndex }) => cellIndex === selected)
    ?? cells.find(({ substrate }) => substrate !== 'native') ?? cells[0];
  if (!cell) return null;
  return <section className="editor-context__section editor-context__properties">
    <h2>土格基质</h2>
    <label className="editor-forest-field editor-substrate-selector">
      <span>查看土格</span>
      <select data-forest-substrate-inspect value={cell.cellIndex} onChange={(event) => select(Number(event.currentTarget.value))}>
        {cells.map(({ cellIndex, substrate }) => <option key={cellIndex} value={cellIndex}>
          第 {Math.floor(cellIndex / forestSoilColumns) + 1} 排 · 第 {cellIndex % forestSoilColumns + 1} 列 · {names[substrate]}
        </option>)}
      </select>
    </label>
    <dl data-forest-substrate-reading={cell.cellIndex}>
      <div><dt>水量／容量</dt><dd>{format.format(cell.waterMl)}／{format.format(cell.waterCapacityMl)} mL</dd></div>
      <div><dt>基础蒸发倍率</dt><dd>×{format.format(cell.evaporationMultiplier)}</dd></div>
      {cell.decomposerMgC !== undefined && <div><dt>分解者库存</dt><dd>{displayMass(cell.decomposerMgC)} mg C</dd></div>}
    </dl>
    {cell.decomposerMgC !== undefined && <p className="editor-context__hint">分解者库存包括土格中的分解微生物。朽木表面的菌丝纹饰按其锚点土格的库存显示。</p>}
  </section>;
}
