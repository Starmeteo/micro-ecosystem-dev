import { useState } from 'react';
import definition from '../../render/assets/r5/pond-composed/definition.json';
import type { PondScenery } from '../../world';

type SceneryStyle = { id: string; label: string; type: string; placeable?: boolean; mm?: number; rings?: boolean; bedAnchor?: unknown };

const styles: readonly SceneryStyle[] = definition.styles;
const placeableStyles = styles.filter(({ placeable }) => placeable === true);
const groups = [
  { label: '底床材质', items: placeableStyles.filter(({ bedAnchor }) => bedAnchor) },
  { label: '场景摆件', items: placeableStyles.filter(({ bedAnchor }) => !bedAnchor) },
];
const artwork: Record<string, { file: string }> = definition.art;
const thumbnails = import.meta.glob<string>('../../render/assets/r5/pond-composed/**/*.png', {
  eager: true, query: '?url', import: 'default',
});
const coordinate = new Intl.NumberFormat('zh-CN', { style: 'percent', maximumFractionDigits: 1 });

export function pondSceneryName(id: string) {
  return styles.find((style) => style.id === id)?.label ?? '场景摆件';
}

export function pondSceneryIsBed(id: string) {
  return Boolean(styles.find((style) => style.id === id)?.bedAnchor);
}

function SceneThumbnail({ style }: { style: SceneryStyle }) {
  const file = artwork[style.type === definition.gravelPatch.type ? definition.gravelPatch.source : style.type]?.file;
  const url = file ? thumbnails['../../render/assets/r5/pond-composed/' + file] : undefined;
  return url ? <img src={url} alt="" width="44" height="32" /> : null;
}

export function PondSceneryCards({ editable, wet, onAdd }: { editable: boolean; wet: boolean; onAdd: (style: string) => void }) {
  return (
    <div className="editor-pond-scenery-cards" data-pond-scenery-cards>
      {groups.map(({ label, items }) => (
        <section key={label} className="editor-pond-scenery-cards" aria-label={label}>
          <h3>{label}</h3>
          <div className="editor-content-options">
            {items.map((style) => (
              <button key={style.id} type="button" className="editor-object" data-pond-scenery-style={style.id}
                aria-label={'放置' + style.label} disabled={!editable || (style.rings && !wet)}
                title={style.rings && !wet ? '补水后可放置浮叶' : style.bedAnchor ? '沿底床摆放，不改变生态库存' : undefined} onClick={() => onAdd(style.id)}>
                <SceneThumbnail style={style} />
                <span><strong>{style.label}</strong>
                  {style.rings && !wet ? <small>补水后可放置</small> : style.mm != null && <small>{style.mm} mm</small>}
                </span>
              </button>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

function SceneryRemoval({ count, label, onRemove }: { count: number; label: string; onRemove: () => void }) {
  const [confirm, setConfirm] = useState(false);
  if (!count) return <button type="button" className="editor-tool" onClick={onRemove}>移除{label}</button>;
  return confirm ? (
    <>
      <p className="editor-context__hint">此摆件的附着装饰将一同移除。</p>
      <div className="editor-details__options">
        <button type="button" className="editor-tool" onClick={() => { setConfirm(false); onRemove(); }}>确认移除{label}</button>
        <button type="button" className="editor-tool" onClick={() => setConfirm(false)}>取消移除</button>
      </div>
    </>
  ) : <button type="button" className="editor-tool" onClick={() => setConfirm(true)}>移除{label}</button>;
}

export function PondSceneryProperties({ items, selected, editable, onSelect, onRemove }: {
  items: readonly PondScenery[];
  selected?: PondScenery;
  editable: boolean;
  onSelect: (id: number) => void;
  onRemove: (id: number) => void;
}) {
  const roots = items.filter(({ parent }) => !parent);
  const attachments = items.filter(({ parent }) => parent);
  const label = selected ? pondSceneryName(selected.style) : '';
  const children = selected ? items.filter(({ parent }) => parent?.kind === 'scenery' && parent.id === selected.id).length : 0;
  const localParent = selected?.parent?.kind === 'scenery' ? items.find(({ id }) => id === selected.parent?.id) : undefined;
  return (
    <section className="editor-context__section" data-pond-scenery-inspector>
      <h2>场景摆件</h2>
      {items.length > 0 ? (
        <label className="editor-pond-field">选择摆件
          <select value={selected?.id ?? ''} onChange={(event) => { if (event.target.value) onSelect(Number(event.target.value)); }}>
            <option value="" disabled>从画面或列表选择</option>
            {roots.length > 0 && <optgroup label="独立摆件">{roots.map((item) => (
              <option key={item.id} value={item.id}>{pondSceneryName(item.style)} {item.id}</option>
            ))}</optgroup>}
            {attachments.length > 0 && <optgroup label="附着装饰">{attachments.map((item) => (
              <option key={item.id} value={item.id}>{pondSceneryName(item.style)} {item.id}</option>
            ))}</optgroup>}
          </select>
        </label>
      ) : <p className="editor-context__hint">从「内容」选择场景摆件。</p>}
      {selected && <div className="editor-context__properties" data-pond-scenery-id={selected.id}>
        <h3>{label} {selected.id}</h3>
        <dl>
          <div><dt>{selected.parent ? '承载物横向位置' : '距画面左侧'}</dt><dd>{coordinate.format(selected.x)}</dd></div>
          <div><dt>{selected.parent ? '承载物纵向位置' : '距画面上侧'}</dt><dd>{coordinate.format(selected.y)}</dd></div>
          {selected.parent && <div><dt>承载物</dt><dd>{localParent ? pondSceneryName(localParent.style) : '沉木'} {selected.parent.id}</dd></div>}
        </dl>
        {editable && <div className="editor-details__options">
          <SceneryRemoval key={selected.id} count={children} label={label} onRemove={() => onRemove(selected.id)} />
        </div>}
      </div>}
      <p className="editor-context__hint">摆件用于调整画面构图。附着装饰随承载物移动。</p>
    </section>
  );
}
