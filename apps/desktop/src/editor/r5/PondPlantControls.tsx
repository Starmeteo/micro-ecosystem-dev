import { useState, type FormEvent } from 'react';
import hornwort from '../../render/assets/r5/pond-composed/hornwort-100.png';
import ribbon from '../../render/assets/r5/pond-composed/ribbon-180.png';
import { ribbonPlantDimensions, waterPlantDoseMgC, waterPlantLimit } from '../../render/r5/pondWaterPlant';
import type { MatterMgCnp, PondRibbonPlantPreset, PondWaterPlantPatch } from '../../world';
import { displayMass, matterSummary } from './formatMatter';

const ribbonPresets = ['small', 'medium', 'large'] as const;
const transmission = new Intl.NumberFormat('zh-CN', { style: 'percent', maximumFractionDigits: 1 });

function waterPlantName(patch: PondWaterPlantPatch) {
  return patch.ribbonPreset ? '带状叶水草' : '轮生细叶水草';
}

function plantMatter(matter: MatterMgCnp) {
  return matterSummary(matter).replaceAll(' mg ', '\u00a0mg\u00a0');
}

export function PondPlantCards({ enabled, wet, count, onAdd }: {
  enabled: boolean;
  wet: boolean;
  count: number;
  onAdd: (ribbonPreset?: PondRibbonPlantPreset) => void;
}) {
  const [preset, setPreset] = useState<PondRibbonPlantPreset>('medium');
  const full = count >= waterPlantLimit;
  const unavailable = !wet ? '补水后可投放' : full ? `已达到 ${waterPlantLimit} 丛` : '';
  const disabled = !enabled || Boolean(unavailable);
  return <>
    <button type="button" className="editor-object" data-pond-water-plant-add
      aria-label="投放轮生细叶水草" disabled={disabled} onClick={() => onAdd()}>
      <img src={hornwort} alt="" width="44" height="32" />
      <span><strong>轮生细叶水草</strong><small>{unavailable || `池底投放 · ${waterPlantDoseMgC} mg C`}</small></span>
    </button>
    <section className="editor-pond-plant-action" aria-label="带状叶水草（苦草型）">
      <label className="editor-pond-field">带状叶规格
        <select data-pond-ribbon-plant-preset value={preset} disabled={disabled}
          onChange={(event) => setPreset(event.currentTarget.value as PondRibbonPlantPreset)}>
          {ribbonPresets.map((choice) => <option key={choice} value={choice}>叶片弧长 {ribbonPlantDimensions[choice].leafArcMm} mm</option>)}
        </select>
      </label>
      <button type="button" className="editor-object" data-pond-ribbon-plant-add
        aria-label="投放带状叶水草" disabled={disabled} onClick={() => onAdd(preset)}>
        <img src={ribbon} alt="" width="44" height="32" />
        <span><strong>带状叶水草</strong><small>{unavailable || '池底投放 · 苦草型'}</small></span>
      </button>
    </section>
  </>;
}

function PlantActions({ patch, onPosition, onTrim, onRemove }: {
  patch: PondWaterPlantPatch;
  onPosition: () => void;
  onTrim: (amount: number) => void;
  onRemove: () => void;
}) {
  const [action, setAction] = useState<'trim' | 'remove' | null>(null);
  const [trim, setTrim] = useState('');
  const amount = Number(trim);
  const carbon = patch.stockMgCnp[0];
  const canTrim = trim.trim() !== '' && Number.isFinite(amount) && amount > 0 && amount < carbon;

  function trimPatch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canTrim) return;
    setAction(null);
    onTrim(amount);
  }

  if (action === 'trim') return <form className="editor-pond-plant-action" onSubmit={trimPatch}>
    <label className="editor-pond-field">修剪量（mg C）
      <input type="number" name="mgC" data-pond-water-plant-trim-amount required step="any" min="0" max={carbon}
        value={trim} onChange={(event) => setTrim(event.currentTarget.value)} />
    </label>
    <p className="editor-context__hint">修剪下来的碳、氮、磷会转入池底碎屑。清空一丛水草可用移除。</p>
    <div className="editor-details__options">
      <button type="submit" className="editor-tool" data-pond-water-plant-trim-confirm disabled={!canTrim}>确认修剪</button>
      <button type="button" className="editor-tool" onClick={() => setAction(null)}>取消修剪</button>
    </div>
  </form>;

  if (action === 'remove') return <div className="editor-pond-plant-action">
    <p className="editor-context__hint">移除后，这一丛水草的剩余碳、氮、磷会转入池底碎屑。</p>
    <div className="editor-details__options">
      <button type="button" className="editor-tool" data-pond-water-plant-remove-confirm
        onClick={() => { setAction(null); onRemove(); }}>确认移除水草</button>
      <button type="button" className="editor-tool" onClick={() => setAction(null)}>取消移除</button>
    </div>
  </div>;

  return <div className="editor-details__options">
    <button type="button" className="editor-tool" onClick={onPosition}>调整池底位置</button>
    <button type="button" className="editor-tool" data-pond-water-plant-trim disabled={carbon <= 0}
      onClick={() => { setTrim(''); setAction('trim'); }}>修剪水草</button>
    <button type="button" className="editor-tool" data-pond-water-plant-remove onClick={() => setAction('remove')}>移除水草</button>
  </div>;
}

export function PondPlantProperties({ patches, selected, editable, onSelect, onPosition, onTrim, onRemove }: {
  patches: readonly PondWaterPlantPatch[];
  selected?: PondWaterPlantPatch;
  editable: boolean;
  onSelect: (id: number) => void;
  onPosition: () => void;
  onTrim: (id: number, amount: number) => void;
  onRemove: (id: number) => void;
}) {
  const ribbonDimensions = selected?.ribbonPreset ? ribbonPlantDimensions[selected.ribbonPreset] : undefined;
  return <section className="editor-context__section" data-pond-water-plants>
    <h2>活体水草</h2>
    {patches.length > 0 ? <label className="editor-pond-field">选择水草
      <select value={selected?.id ?? ''} data-pond-water-plant-selector onChange={(event) => {
        if (event.currentTarget.value) onSelect(Number(event.currentTarget.value));
      }}>
        <option value="" disabled>从画面或列表选择</option>
        {patches.map((patch) => <option key={patch.id} value={patch.id}>
          {waterPlantName(patch)} {patch.id} · {displayMass(patch.stockMgCnp[0])} mg C
        </option>)}
      </select>
    </label> : <p className="editor-context__hint">可在「内容」把水草种到池底。</p>}
    {selected && <div className="editor-context__properties editor-pond-plant-action"
      data-pond-water-plant-id={selected.id} data-pond-ribbon-plant={selected.ribbonPreset ?? undefined}>
      <dl>
        <div><dt>株型</dt><dd>{waterPlantName(selected)}</dd></div>
        {ribbonDimensions && <>
          <div><dt>规格叶片弧长</dt><dd>{ribbonDimensions.leafArcMm} mm</dd></div>
          <div><dt>规格叶宽</dt><dd>{ribbonDimensions.leafWidthMm} mm</dd></div>
          <div><dt>规格丛宽</dt><dd>{ribbonDimensions.spreadMm} mm</dd></div>
          {typeof selected.ribbonLightTransmission === 'number' && <div>
            <dt title="叶片中部获得的水面入射光比例，昼夜光照另计">水中透光</dt>
            <dd data-pond-ribbon-light-transmission={selected.ribbonLightTransmission}>{transmission.format(selected.ribbonLightTransmission)}</dd>
          </div>}
        </>}
        <div><dt>沿池底位置</dt><dd>{Math.round(selected.x * 100)}%</dd></div>
        <div><dt>状态</dt><dd>{selected.submerged ? '浸没' : '露出'}</dd></div>
        <div><dt>投放量</dt><dd>{plantMatter(selected.initialMgCnp)}</dd></div>
        <div><dt>当前库存</dt><dd data-pond-water-plant-stock={selected.stockMgCnp.join(',')}>{plantMatter(selected.stockMgCnp)}</dd></div>
      </dl>
      {editable && <PlantActions key={selected.id} patch={selected} onPosition={onPosition}
        onTrim={(amount) => onTrim(selected.id, amount)} onRemove={() => onRemove(selected.id)} />}
    </div>}
  </section>;
}
