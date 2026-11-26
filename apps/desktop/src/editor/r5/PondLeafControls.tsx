import { useState } from 'react';
import type { PondLeafDeposit, PondLeafDepositPreset, PondLeafDepositStage } from '../../world';
import { displayMass } from './formatMatter';

const names: Record<PondLeafDepositPreset, string> = { midground: '中景沉叶', foreground: '大沉叶' };
const stages: Record<PondLeafDepositStage, string> = { whole: '整叶', fragments: '残叶', fine: '细碎屑' };
export const leafDepositLimit = 16;

export function PondLeafDepositCard({ editable, count, begin }: {
  editable: boolean;
  count: number;
  begin: (preset: PondLeafDepositPreset, mgC: number) => void;
}) {
  const [preset, setPreset] = useState<PondLeafDepositPreset>('midground');
  const [amount, setAmount] = useState('50');
  const carbon = Number(amount);
  const full = count >= leafDepositLimit;
  const ready = editable && !full && Number.isFinite(carbon) && carbon >= .01 && carbon <= 50000;
  return <section className="editor-pond-scenery-cards" aria-label="投放沉水落叶">
    <h3 className="editor-pond-drawer-heading">沉水落叶</h3>
    <form className="editor-pond-form" onSubmit={(event) => { event.preventDefault(); if (ready) begin(preset, carbon); }}>
      <label className="editor-pond-field">叶片样式
        <select value={preset} onChange={(event) => setPreset(event.target.value as PondLeafDepositPreset)} disabled={!editable || full}>
          <option value="midground">{names.midground} · 55 mm</option>
          <option value="foreground">{names.foreground} · 105 mm</option>
        </select>
      </label>
      <label className="editor-pond-field">投放量（mg C）
        <input data-pond-leaf-dose type="number" min="0.01" max="50000" step="any" required value={amount}
          disabled={!editable || full} onChange={(event) => setAmount(event.target.value)} />
      </label>
      <div className="editor-details__options editor-pond-form__actions">
        <button type="submit" className="editor-tool" data-pond-leaf-add disabled={!ready}>到池底放置沉叶</button>
      </div>
    </form>
    <p className="editor-context__hint">{full ? '当前剖面已达沉叶展示上限，可在主剖面的环境面板添加整池碎屑。' : '投放计入整池碎屑，落点保存在当前剖面。'}</p>
  </section>;
}

function LeafRemoval({ leaf, remove }: { leaf: PondLeafDeposit; remove: (id: number) => Promise<boolean> }) {
  const [confirm, setConfirm] = useState(false);
  const [pending, setPending] = useState(false);

  async function takeOut() {
    setPending(true);
    try {
      if (await remove(leaf.id)) setConfirm(false);
    } finally {
      setPending(false);
    }
  }

  if (!confirm) return <button type="button" className="editor-tool" data-pond-leaf-remove onClick={() => setConfirm(true)}>取出沉叶</button>;
  return <div>
    <p className="editor-context__hint">将 {displayMass(leaf.remainingMgC)} mg C 和对应的氮磷移出池塘。已分解的部分留在生态中。</p>
    <div className="editor-details__options">
      <button type="button" className="editor-tool" data-pond-leaf-remove-confirm disabled={pending} onClick={() => void takeOut()}>确认取出</button>
      <button type="button" className="editor-tool" disabled={pending} onClick={() => setConfirm(false)}>取消</button>
    </div>
  </div>;
}

export function PondLeafDepositProperties({ deposits, selected, editable, select, move, remove }: {
  deposits: readonly PondLeafDeposit[];
  selected?: PondLeafDeposit;
  editable: boolean;
  select: (id: number) => void;
  move: (id: number) => void;
  remove: (id: number) => Promise<boolean>;
}) {
  return <section className="editor-context__section" data-pond-leaf-inspector>
    <h2>沉水落叶</h2>
    <label className="editor-pond-field">选择沉叶
      <select data-pond-leaf-select value={selected?.id ?? ''} onChange={(event) => select(Number(event.target.value))}>
        <option value="" disabled>从画面或列表选择</option>
        {deposits.map(({ id, preset }) => <option key={id} value={id}>{names[preset]} {id}</option>)}
      </select>
    </label>
    {selected && <>
      <dl className="editor-stock" data-pond-leaf-id={selected.id}>
        <div><dt>投放剂量</dt><dd>{displayMass(selected.initialMgC)} mg C</dd></div>
        <div><dt>剩余份额</dt><dd data-pond-leaf-remaining={selected.remainingMgC}>{displayMass(selected.remainingMgC)} mg C</dd></div>
        <div><dt>形态</dt><dd>{stages[selected.stage]}</dd></div>
      </dl>
      {editable && <>
        <div className="editor-details__options">
          <button type="button" className="editor-tool" data-pond-leaf-move onClick={() => move(selected.id)}>移动沉叶显示位置</button>
          <LeafRemoval key={selected.id} leaf={selected} remove={remove} />
        </div>
      </>}
    </>}
  </section>;
}
