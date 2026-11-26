import { useState, type FormEvent } from 'react';
import type { FixtureAnimal } from '../../world';

interface LifeReading {
  id: string;
  stage: 'juvenile' | 'adult';
  ageSeconds: number;
  biomassMgC?: number;
  broodProgress?: number | null;
}

const activityLabels: Record<string, string> = {
  moving: '移动', resting: '停驻', sheltering: '躲藏', foraging: '觅食', feeding: '进食',
};

function ageLabel(seconds: number) {
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor(seconds / 3_600) % 24;
  if (days) return `${days} 天 ${hours} 小时`;
  const minutes = Math.floor(seconds / 60) % 60;
  return hours ? `${hours} 小时 ${minutes} 分钟` : minutes ? `${minutes} 分钟` : `${Math.floor(seconds)} 秒`;
}

export function AnimalIndividuals({ kind, animals, life, selectedId, enabled, onSelect, onLocate, onRemove }: {
  kind: 'woodlouse' | 'shrimp';
  animals: readonly FixtureAnimal[];
  life?: readonly LifeReading[];
  selectedId?: string;
  enabled: boolean;
  onSelect: (id: string) => void;
  onLocate: () => void;
  onRemove: (id: string) => void;
}) {
  const [confirmation, setConfirmation] = useState('');
  const selected = animals.find(({ id }) => id === selectedId);
  const reading = life?.find(({ id }) => id === selected?.id);
  const name = kind === 'woodlouse' ? '鼠妇' : '米虾';

  return <section className="editor-context__section" data-animal-inspector={kind}>
    <h2>{name}个体</h2>
    {animals.length > 0 ? <label className="editor-pond-field">个体编号
      <select data-animal-select value={selected?.id ?? ''} onChange={(event) => {
        setConfirmation('');
        onSelect(event.target.value);
      }}>
        <option value="">选择个体</option>
        {animals.map(({ id, activity }) => <option key={id} value={id}>{id} · {activityLabels[activity] ?? '活动状态待更新'}</option>)}
      </select>
    </label> : <p className="editor-context__hint">当前{kind === 'woodlouse' ? '世界' : '剖面'}没有{name}。</p>}
    {selected && <div data-animal-id={selected.id}>
      <dl className="editor-stock">
        <div><dt>当前活动</dt><dd>{activityLabels[selected.activity] ?? '暂无读数'}</dd></div>
        <div><dt>生命阶段</dt><dd>{reading ? reading.stage === 'adult' ? '成体' : '幼体' : '暂无读数'}</dd></div>
        {reading && <div><dt>模拟年龄</dt><dd>{ageLabel(reading.ageSeconds)}</dd></div>}
        {reading?.biomassMgC !== undefined && <div><dt>躯体碳</dt><dd>{reading.biomassMgC.toFixed(3)} mg C</dd></div>}
        {reading?.broodProgress != null && <div><dt>育幼进度</dt><dd>{Math.round(100 * reading.broodProgress)}%</dd></div>}
      </dl>
      <div className="editor-details__options">
        <button type="button" className="editor-tool" data-animal-locate onClick={onLocate}>放大定位</button>
        {confirmation !== selected.id && <button type="button" className="editor-tool" data-animal-remove
          disabled={!enabled} onClick={() => setConfirmation(selected.id)}>取出{name}</button>}
        {confirmation === selected.id && <>
          <button type="button" className="editor-tool" data-animal-remove-confirm disabled={!enabled} onClick={() => {
            onRemove(selected.id);
            setConfirmation('');
          }}>确认取出</button>
          <button type="button" className="editor-tool" onClick={() => setConfirmation('')}>取消</button>
        </>}
      </div>
      <p className="editor-context__hint">{enabled ? '取出会带走该个体的碳、氮、磷库存。' : '完成放置后，或进入暂停编辑，可取出个体。'}</p>
    </div>}
  </section>;
}

export function DaphniaRemoval({ count, enabled, onRemove }: { count: number; enabled: boolean; onRemove: (quantity: number) => void }) {
  const [quantity, setQuantity] = useState('1');
  const amount = Number(quantity);
  const allowed = enabled && Number.isSafeInteger(amount) && amount >= 1 && amount <= count;

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (allowed) onRemove(amount);
  }

  return <details className="editor-details" data-daphnia-removal>
    <summary>按数量取出水蚤</summary>
    <p className="editor-context__hint">整池水蚤按群体建模。取出按数量比例带走碳、氮和磷。</p>
    <form onSubmit={submit}>
      <label className="editor-pond-field">取出数量
        <input type="number" min={1} max={Math.max(count, 1)} step={1} value={quantity} data-daphnia-remove-count
          disabled={!enabled || count === 0} onChange={(event) => setQuantity(event.target.value)} required />
      </label>
      <button type="submit" className="editor-tool" data-daphnia-remove disabled={!allowed}>取出水蚤</button>
    </form>
  </details>;
}
