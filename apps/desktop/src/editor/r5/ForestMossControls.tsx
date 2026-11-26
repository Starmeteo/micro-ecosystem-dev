import { useState } from 'react';
import type { ForestFernPatch, ForestMossPatch, MatterMgCnp } from '../../world';
import { displayMass, matterSummary } from './formatMatter';
import { fernFrame } from '../../render/r5/forestFern';

type PlantSpecies = 'moss' | 'fern';
const plantName: Record<PlantSpecies, string> = { moss: '地表苔藓', fern: '小型蕨丛' };
const plantShortName: Record<PlantSpecies, string> = { moss: '苔藓', fern: '蕨丛' };
const fernStageName = { tender: '新叶', small: '小丛', open: '展开', stressed: '胁迫' };
type PlantCard = {
  kind: PlantSpecies;
  dose: MatterMgCnp;
  editable: boolean;
  onPlant: () => void;
};

export function ForestPlantCards({ plants }: { plants: PlantCard[] }) {
  return <>
    <h3 className="editor-forest-drawer-heading">活体植被</h3>
    <div className="editor-content-options">
      {plants.map(({ kind, dose, editable, onPlant }) => (
        <button key={kind} className="editor-object" type="button" disabled={!editable} onClick={onPlant}
          data-forest-fern-plant={kind === 'fern' || undefined}>
          <span><strong>种植{plantName[kind]}</strong><small>{matterSummary(dose)}</small></span>
        </button>
      ))}
    </div>
  </>;
}

function PlantActions({ kind, patch, move, trim, remove }: {
  kind: PlantSpecies;
  patch: ForestMossPatch;
  move: () => void;
  trim: (mgC: number) => void;
  remove: () => void;
}) {
  const [amount, setAmount] = useState('');
  const [confirm, setConfirm] = useState(false);
  const mgC = Number(amount);
  const valid = Number.isFinite(mgC) && mgC > 0 && mgC < patch.stockMgCnp[0];
  return <>
    <button className="editor-tool" type="button" onClick={move}>移动{plantName[kind]}</button>
    <form className="editor-stocking" onSubmit={(event) => {
      event.preventDefault();
      if (valid) { trim(mgC); setAmount(''); }
    }}>
      <label className="editor-forest-field">修剪量（mg C）
        <input type="number" step="any" value={amount} onChange={(event) => setAmount(event.target.value)} />
      </label>
      <button className="editor-tool" type="submit" disabled={!valid}>修剪并留在土格</button>
    </form>
    {confirm ? <>
      <p className="editor-context__hint">剩余{plantShortName[kind]}库存会随{plantShortName[kind]}取出世界。</p>
      <div className="editor-details__options">
        <button className="editor-tool" type="button" onClick={() => { setConfirm(false); remove(); }}>确认取出{plantShortName[kind]}</button>
        <button className="editor-tool" type="button" onClick={() => setConfirm(false)}>取消取出</button>
      </div>
    </> : <button className="editor-tool" type="button" onClick={() => setConfirm(true)}>取出{plantName[kind]}</button>}
  </>;
}

export function ForestPlantProperties({ kind, patches, selected, initialMgC, editable, select, move, trim, remove }: {
  kind: PlantSpecies;
  patches: ForestMossPatch[];
  selected?: ForestMossPatch | ForestFernPatch;
  initialMgC?: number;
  editable: boolean;
  select: (id: number) => void;
  move: (id: number) => void;
  trim: (id: number, mgC: number) => void;
  remove: (id: number) => void;
}) {
  const fern = selected && 'canopyShadeFraction' in selected ? selected : undefined;
  return <section className="editor-context__section" data-forest-moss-properties={kind === 'moss' || undefined}
    data-forest-fern-properties={kind === 'fern' || undefined}>
    <h2>活体{plantName[kind]}</h2>
    <label className="editor-forest-field">选择斑块
      <select value={selected?.id ?? ''} onChange={(event) => select(Number(event.target.value))}>
        <option value="" disabled>从画面或列表选择</option>
        {patches.map(({ id }) => <option key={id} value={id}>{plantName[kind]} {id}</option>)}
      </select>
    </label>
    {selected && <>
      <dl className="editor-stock" data-forest-moss-id={kind === 'moss' ? selected.id : undefined}
        data-forest-fern-id={kind === 'fern' ? selected.id : undefined} data-stock-mg-c={selected.stockMgCnp[0]}>
        <div><dt>所在土格</dt><dd>{selected.cellIndex + 1}</dd></div>
        {['C', 'N', 'P'].map((unit, index) => <div key={unit}><dt>库存 {unit}</dt><dd>{displayMass(selected.stockMgCnp[index])} mg</dd></div>)}
        <div><dt>所在土格胁迫</dt><dd>{Math.round(selected.stressFraction * 100)}%</dd></div>
        {fern && <div><dt title="可能被更强的遮阴覆盖，以环境读数为准">叶冠遮阴潜力</dt><dd>{Math.round(fern.canopyShadeFraction * 100)}%</dd></div>}
        {fern && initialMgC !== undefined && <div><dt>当前长势</dt><dd>{fernStageName[fernFrame(fern, initialMgC)]}</dd></div>}
      </dl>
      {editable && <PlantActions key={selected.id} kind={kind} patch={selected} move={() => move(selected.id)}
        trim={(mgC) => trim(selected.id, mgC)} remove={() => remove(selected.id)} />}
    </>}
  </section>;
}
