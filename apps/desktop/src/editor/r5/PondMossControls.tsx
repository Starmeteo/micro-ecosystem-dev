import { useEffect, useState, type FormEvent } from 'react';
import matThumbnail from '../../render/assets/r5/pond-composed/moss-ledge.png';
import frondThumbnail from '../../render/assets/r5/pond-composed/moss-drape.png';
import type { PondMossDefinition, PondMossParent, PondMossPatch, PondMossPreset } from '../../world';
import { pondBedName, type PondBedObject } from './PondBedControls';
import { displayMass, matterSummary, nutrients } from './formatMatter';

const moss = {
  mat: { name: '苔藓小垫', size: 30, thumbnail: matThumbnail },
  frond: { name: '分枝绒簇', size: 50, thumbnail: frondThumbnail },
} as const;
const presets = ['mat', 'frond'] as const;

export function pondMossName(preset: PondMossPreset) {
  return moss[preset].name;
}

export function pondMossParentKey(parent: PondMossParent) {
  return `${parent.kind}:${parent.id}`;
}

export function PondMossCards({ enabled, definitions, onAdd }: {
  enabled: boolean;
  definitions?: PondMossDefinition[] | null;
  onAdd: (preset: PondMossPreset) => void;
}) {
  return presets.map((preset) => {
    const item = moss[preset];
    const dose = definitions?.find((entry) => entry.preset === preset)?.initialMgCnp;
    return (
      <button key={preset} type="button" className="editor-object" data-pond-plant-preset={preset}
        aria-label={'附着' + item.name} onClick={() => onAdd(preset)} disabled={!enabled}>
        <img src={item.thumbnail} alt="" width="44" height="32" />
        <span><strong>{item.name}</strong><small>跨度 {item.size} mm{dose ? ` · ${displayMass(dose[0])} mg C` : ''}</small></span>
      </button>
    );
  });
}

function offsetLabel(offset: number) {
  return offset === 0 ? '中央' : `${offset < 0 ? '左' : '右'}侧 ${Math.round(Math.abs(offset) * 100)}%`;
}

function OffsetField({ value, disabled, onChange }: { value: number; disabled?: boolean; onChange: (offset: number) => void }) {
  return (
    <label className="editor-pond-moss-offset">
      <span>沿承载面位置</span>
      <input type="range" name="offset" min="-1" max="1" step=".05" value={value}
        disabled={disabled} aria-valuetext={offsetLabel(value)} onChange={(event) => onChange(event.currentTarget.valueAsNumber)} />
      <output>{offsetLabel(value)}</output>
    </label>
  );
}

export function PondMossPlacementForm({ preset, carriers, parent, offset, enabled, definitions, chooseParent, chooseOffset, confirm, cancel }: {
  preset: PondMossPreset;
  carriers: PondBedObject[];
  parent: PondMossParent | null;
  offset: number;
  enabled: boolean;
  definitions?: PondMossDefinition[] | null;
  chooseParent: (parent: PondMossParent | null) => void;
  chooseOffset: (offset: number) => void;
  confirm: () => void;
  cancel: () => void;
}) {
  const carrierKey = parent ? pondMossParentKey(parent) : '';
  const dose = definitions?.find((entry) => entry.preset === preset)?.initialMgCnp;
  const canConfirm = enabled && carriers.some((item) => pondMossParentKey(item) === carrierKey);
  return (
    <section className="editor-content-drawer editor-pond-plant-placement" data-with-pond-moss="true"
      data-placement-kind="pondPlant" data-placement-preset={preset} aria-labelledby="editor-pond-plant-place">
      <header><h2 id="editor-pond-plant-place">附着{pondMossName(preset)}</h2></header>
      <form onSubmit={(event) => { event.preventDefault(); if (canConfirm) confirm(); }}>
        <label className="editor-pond-field">承载物
          <select name="carrier" required value={carrierKey} disabled={!enabled || carriers.length === 0} onChange={(event) => {
            const selected = carriers.find((item) => pondMossParentKey(item) === event.currentTarget.value);
            chooseParent(selected ? { kind: selected.kind, id: selected.id } : null);
          }}>
            <option value="">选择承载物</option>
            {carriers.map((item) => (
              <option value={pondMossParentKey(item)} key={pondMossParentKey(item)}>{pondBedName(item)} {item.id}</option>
            ))}
          </select>
        </label>
        <OffsetField value={offset} disabled={!canConfirm} onChange={chooseOffset} />
        {dose && <p className="editor-context__hint" data-pond-plant-dose>投放 {matterSummary(dose)}</p>}
        <p className="editor-context__hint">{carriers.length === 0 ? '先放置一件可附着的主石或沉木。'
          : '每件承载物可附着一个斑块。选浸没的表面位置。'}</p>
        <div className="editor-details__options">
          <button type="submit" className="editor-tool" disabled={!canConfirm}>确认附着</button>
          <button type="button" className="editor-tool" onClick={cancel}>取消附着</button>
        </div>
      </form>
    </section>
  );
}

function PondMossActions({ patch, onPreview, onMove, onTrim, onRemove }: {
  patch: PondMossPatch;
  onPreview: (offset: number | null) => void;
  onMove: (offset: number) => void;
  onTrim: (mgC: number) => void;
  onRemove: () => void;
}) {
  const [action, setAction] = useState<'move' | 'trim' | 'remove' | null>(null);
  const [draft, setDraft] = useState(patch.offset);
  const [trim, setTrim] = useState('');
  const amount = Number(trim);
  const canTrim = Number.isFinite(amount) && amount > 0 && amount < patch.stockMgCnp[0];
  const cancel = () => { setAction(null); onPreview(null); };
  useEffect(() => () => onPreview(null), [onPreview]);
  function trimMoss(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canTrim) return;
    cancel();
    onTrim(amount);
  }
  if (action === 'move') return (
    <div className="editor-pond-moss-action">
      <OffsetField value={draft} onChange={(offset) => { setDraft(offset); onPreview(offset); }} />
      <div className="editor-details__options">
        <button type="button" className="editor-tool" disabled={draft === patch.offset} onClick={() => {
          cancel();
          onMove(draft);
        }}>确认位置</button>
        <button type="button" className="editor-tool" onClick={cancel}>取消调整</button>
      </div>
    </div>
  );
  if (action === 'trim') return (
    <form className="editor-pond-moss-action" onSubmit={trimMoss}>
      <label className="editor-pond-field">修剪量（mg C）
        <input type="number" name="mgC" required min="0" max={patch.stockMgCnp[0]} step="any"
          value={trim} onChange={(event) => setTrim(event.currentTarget.value)} />
      </label>
      <p className="editor-context__hint">修剪量和对应的氮、磷将转入这片水体的碎屑库存。清空斑块可用移除。</p>
      <div className="editor-details__options">
        <button type="submit" className="editor-tool" disabled={!canTrim}>确认修剪</button>
        <button type="button" className="editor-tool" onClick={cancel}>取消修剪</button>
      </div>
    </form>
  );
  if (action === 'remove') return (
    <div className="editor-pond-moss-action">
      <p className="editor-context__hint">移除后，斑块的剩余碳、氮、磷将转入这片水体的碎屑库存。</p>
      <div className="editor-details__options">
        <button type="button" className="editor-tool" onClick={() => { cancel(); onRemove(); }}>确认移除斑块</button>
        <button type="button" className="editor-tool" onClick={cancel}>取消移除</button>
      </div>
    </div>
  );
  return (
    <div className="editor-details__options">
      <button type="button" className="editor-tool" onClick={() => { setDraft(patch.offset); setAction('move'); }}>调整位置</button>
      <button type="button" className="editor-tool" disabled={patch.stockMgCnp[0] <= 0} onClick={() => { setTrim(''); setAction('trim'); }}>修剪</button>
      <button type="button" className="editor-tool" onClick={() => setAction('remove')}>移除斑块</button>
    </div>
  );
}

export function PondMossProperties({ patches, selected, carriers, editable, select, preview, move, trim, remove }: {
  patches: PondMossPatch[];
  selected: PondMossPatch | undefined;
  carriers: PondBedObject[];
  editable: boolean;
  select: (id: number) => void;
  preview: (offset: number | null) => void;
  move: (id: number, offset: number) => void;
  trim: (id: number, mgC: number) => void;
  remove: (id: number) => void;
}) {
  const parent = selected && carriers.find((item) => pondMossParentKey(item) === pondMossParentKey(selected.parent));
  return (
    <>
      <section className="editor-context__section">
        <h2>附着斑块</h2>
        {patches.length === 0 && <p className="editor-context__hint">从「内容」选择斑块与承载物。</p>}
        <div className="editor-pond-plant-list">
          {patches.map((patch) => (
            <button key={patch.id} type="button" className="editor-object" data-pond-plant-id={patch.id}
              data-pond-plant-preset={patch.preset} aria-pressed={patch.id === selected?.id} onClick={() => select(patch.id)}>
              <img src={moss[patch.preset].thumbnail} alt="" width="44" height="32" />
              <span><strong>{pondMossName(patch.preset)} {patch.id}</strong><small>{patch.submerged ? '浸没' : '露出'}</small></span>
            </button>
          ))}
        </div>
      </section>
      {selected && <section className="editor-context__section editor-context__properties" data-submerged={selected.submerged}>
        <h2>{pondMossName(selected.preset)} {selected.id}</h2>
        <dl className="editor-pond-moss-stock">
          <div><dt>承载物</dt><dd>{parent ? `${pondBedName(parent)} ${parent.id}` : '—'}</dd></div>
          <div><dt>沿承载面位置</dt><dd>{offsetLabel(selected.offset)}</dd></div>
          <div><dt>水线状态</dt><dd>{selected.submerged ? '浸没' : '露出'}</dd></div>
          {selected.stockMgCnp.map((value, index) => (
            <div key={nutrients[index]}><dt>{['当前碳', '当前氮', '当前磷'][index]}</dt>
              <dd data-nutrient={nutrients[index]} data-mg={value}>{displayMass(value)} mg {nutrients[index].toUpperCase()}</dd>
            </div>
          ))}
        </dl>
        {editable && <PondMossActions key={selected.id} patch={selected} onPreview={preview} onMove={(offset) => move(selected.id, offset)}
          onTrim={(mgC) => trim(selected.id, mgC)} onRemove={() => remove(selected.id)} />}
      </section>}
    </>
  );
}
