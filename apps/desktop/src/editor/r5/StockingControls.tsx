import { useState } from 'react';

const speciesNames = { woodlice: '鼠妇', shrimp: '米虾', daphnia: '水蚤' } as const;
export type StockSpecies = keyof typeof speciesNames;
export const stockSpeciesName = (species: StockSpecies) => speciesNames[species];

export function StockingControls({ options, disabled, needsWater, onStart }: {
  options: { species: StockSpecies; capacity: number }[];
  disabled: boolean;
  needsWater: boolean;
  onStart: (species: StockSpecies, count: number) => void;
}) {
  const [species, setSpecies] = useState<StockSpecies>(options[0].species);
  const [quantity, setQuantity] = useState('1');
  const option = options.find((entry) => entry.species === species) ?? options[0];
  const count = Number(quantity);
  const ready = !disabled && !needsWater && Number.isInteger(count) && count >= 1 && count <= option.capacity;
  const hint = needsWater ? '补水后可以投放。'
    : disabled ? '进入编辑后可以投放。'
    : option.capacity === 0 ? '已达这个世界的数量上限。'
    : option.species === 'daphnia' ? `水蚤加入这个水域的群体。还可投放 ${option.capacity} 只。`
    : `将投放成体并保留各自身份。还可投放 ${option.capacity} 只。`;

  return <section className="editor-context__section">
    <h2>投放生物</h2>
    <form className="editor-stocking" onSubmit={(event) => { event.preventDefault(); if (ready) onStart(option.species, count); }}>
      <label className="editor-pond-field">种类
        <select value={option.species} onChange={(event) => { setSpecies(event.target.value as StockSpecies); setQuantity('1'); }}>
          {options.map(({ species }) => <option key={species} value={species}>{stockSpeciesName(species)}</option>)}
        </select>
      </label>
      <label className="editor-pond-field">投放数量
        <input type="number" value={quantity} min={1} max={Math.max(1, option.capacity)} step={1}
          onChange={(event) => setQuantity(event.target.value)} disabled={disabled || needsWater} required />
      </label>
      <p className="editor-context__hint">{hint}</p>
      <button type="submit" className="editor-tool" disabled={!ready}>{option.species === 'daphnia' ? '加入当前水域' : '到场景选择落点'}</button>
    </form>
  </section>;
}
