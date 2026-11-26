import { useState } from 'react';
import type { FormEvent } from 'react';
import type { PondBasinSnapshot, PondEdit } from '../world';

const actions = {
  addWater: { label: '补水', unit: 'L', min: 0.01, step: 0.01, initial: 0.8 },
  drainWater: { label: '排水', unit: 'L', min: 0.01, step: 'any', initial: 0.8 },
  reshape: { label: '调整水面面积', unit: 'cm²', min: 400, step: 1, initial: 2400 },
  addDetritus: { label: '加入碎屑', unit: 'mg C', min: 0.01, step: 'any', initial: 50 },
} as const;

type Action = keyof typeof actions;

const reading = (value: number | null, unit: string, decimals = 2) =>
  value === null ? '—' : `${value.toFixed(decimals)} ${unit}`;

export function PondInspector({ basin, basins, selectBasin, editing, canEdit, onEdit, onShape }: {
  basin: PondBasinSnapshot;
  basins: PondBasinSnapshot[];
  selectBasin: (id: string) => void;
  editing: boolean;
  canEdit: boolean;
  onEdit: (edit: PondEdit) => void;
  onShape?: () => void;
}) {
  const [action, setAction] = useState<Action>('addWater');
  const [amount, setAmount] = useState<string>('0.8');
  const { unit, min, step } = actions[action];

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canEdit) return;
    const value = Number(amount);
    if (!Number.isFinite(value) || value < min) return;
    const basinId = basin.id;
    switch (action) {
      case 'addWater': case 'drainWater': onEdit({ type: action, basinId, liters: value }); break;
      case 'reshape': onEdit({ type: action, basinId, surfaceAreaCm2: value }); break;
      case 'addDetritus': onEdit({ type: action, basinId, mgC: value }); break;
    }
  }

  return (
    <section className="editor-context__section">
      <h2>池塘环境</h2>
      {basins.length > 1 && <label className="editor-pond-field">观察水体
        <select value={basin.id} onChange={(event) => selectBasin(event.target.value)}>
          {basins.map((entry) => <option key={entry.id} value={entry.id}>{entry.label}</option>)}
        </select>
      </label>}
      {!basin.wet && <p className="editor-context__hint">水体已干涸</p>}
      <dl className="editor-stock">
        <div><dt>水量</dt><dd data-pond-water>{reading(basin.waterLiters, 'L')}</dd></div>
        <div><dt>平均水深</dt><dd>{reading(basin.depthCm, 'cm')}</dd></div>
        <div><dt>水面面积</dt><dd>{reading(basin.surfaceAreaCm2, 'cm²', 0)}</dd></div>
        <div><dt>水温</dt><dd>{reading(basin.temperatureCelsius, '°C', 1)}</dd></div>
        <div><dt>全池溶氧</dt><dd>{reading(basin.oxygenMgPerLiter, 'mg/L')}</dd></div>
        <div><dt>小虾</dt><dd>{(basin.staticSections ?? []).reduce((total, section) => total + (section.shrimp?.length ?? 0), basin.shrimp.length)} 只</dd></div>
        <div><dt>水蚤</dt><dd>{basin.daphniaCount} 只</dd></div>
      </dl>
      {editing && onShape && <button className="editor-tool" type="button" disabled={!canEdit} onClick={onShape}>编辑池岸形状</button>}
      {editing && <form className="editor-pond-form" aria-label="修改水体" onSubmit={submit}>
        <label className="editor-pond-field">操作
          <select value={action} disabled={!canEdit} onChange={(event) => {
            const next = event.target.value as Action;
            setAction(next);
            setAmount(String(next === 'reshape' ? basin.surfaceAreaCm2 : actions[next].initial));
          }}>
            {Object.entries(actions).map(([id, item]) => <option key={id} value={id}>
              {id === 'reshape' && onShape ? '按比例调整总水面面积' : item.label}
            </option>)}
          </select>
        </label>
        <label className="editor-pond-field">数值（{unit}）
          <input required type="number" min={min} step={step} disabled={!canEdit} value={amount} onChange={(event) => setAmount(event.target.value)} />
        </label>
        <div className="editor-details__options editor-pond-form__actions">
          <button className="editor-tool" type="submit" disabled={!canEdit}>应用</button>
          {action === 'drainWater' && <button className="editor-tool" type="button" disabled={!canEdit || !basin.wet}
            onClick={() => setAmount(String(basin.waterLiters))}>填入全部水量</button>}
        </div>
      </form>}
      <details className="editor-details editor-details--small">
        <summary>养分与有机质</summary>
        <dl className="editor-stock">
          <div><dt>可利用氮</dt><dd>{reading(basin.availableNitrogenMgPerLiter, 'mg N/L', 3)}</dd></div>
          <div><dt>可利用磷</dt><dd>{reading(basin.availablePhosphorusMgPerLiter, 'mg P/L', 3)}</dd></div>
          <div><dt>水草</dt><dd>{reading(basin.plantMgC, 'mg C')}</dd></div>
          <div><dt>悬浮藻</dt><dd>{reading(basin.algaeMgC, 'mg C')}</dd></div>
          <div><dt>生物膜</dt><dd>{reading(basin.biofilmMgC, 'mg C')}</dd></div>
          <div><dt>碎屑</dt><dd>{reading(basin.detritusMgC, 'mg C')}</dd></div>
        </dl>
      </details>
    </section>
  );
}
