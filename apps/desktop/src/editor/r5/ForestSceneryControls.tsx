import { useState } from 'react';
import definition from '../../render/assets/r5/forest-composed/definition.json';
import { seedShellGroup, seedShellGroups, type SeedShellDensity } from '../../render/r5/forestSeedShells';
import type { ForestScenery, ForestSceneryParent, ScenePoint } from '../../world';
import { forestTerrainName, type ForestTerrainObject } from './ForestTerrainControls';

type SceneryStyle = {
  id: string;
  label: string;
  type: string;
  placeable?: boolean;
  attachedOnly?: boolean;
  parentKinds?: string[];
  parentTypes?: string[];
  defaultLocal?: number[];
  seedShellGroup?: string;
};

const styles: readonly SceneryStyle[] = definition.styles;
const placeableStyles = styles.filter(({ placeable, seedShellGroup }) => placeable === true && !seedShellGroup);
const artwork: Record<string, { file: string }> = definition.art;
const thumbnails = import.meta.glob<string>('../../render/assets/r5/forest-composed/**/*.png', {
  eager: true, query: '?url', import: 'default',
});
const coordinate = new Intl.NumberFormat('zh-CN', { style: 'percent', maximumFractionDigits: 1 });

type Carrier = ForestSceneryParent & { label: string };
const styleById = new Map(styles.map((style) => [style.id, style]));
const shellDensities = Object.keys(seedShellGroups) as SeedShellDensity[];

export function forestSceneryName(id: string) {
  return styleById.get(id)?.label ?? '林下摆件';
}

export function forestSceneryIsAttachment(id: string) {
  const style = styleById.get(id);
  return Boolean(style?.attachedOnly || style?.parentKinds?.length);
}

export function forestSceneryOffset(id: string): ScenePoint {
  const offset = styleById.get(id)?.defaultLocal;
  return { x: offset?.[0] ?? 0, y: offset?.[1] ?? -.09 };
}

function carrierKey({ kind, id }: ForestSceneryParent) {
  return `${kind}:${id}`;
}

function carriersFor(terrain: readonly ForestTerrainObject[], scenery: readonly ForestScenery[]): Carrier[] {
  return [
    ...terrain.map((item) => ({ kind: 'terrain' as const, id: item.id, label: `${forestTerrainName(item)} ${item.id}` })),
    ...scenery.map((item) => ({ kind: 'scenery' as const, id: item.id, label: `${forestSceneryName(item.style)} ${item.id}` })),
  ];
}

export function forestSceneryCarriers(id: string, terrain: readonly ForestTerrainObject[], scenery: readonly ForestScenery[]) {
  const style = styleById.get(id);
  if (!style) return [];
  const compatibleTerrain = terrain.filter((item) => (!style.parentKinds || style.parentKinds.includes('terrain'))
    && (!style.parentTypes || style.parentTypes.includes(item.kind) || ('preset' in item && style.parentTypes.includes(item.preset))));
  const compatibleScenery = scenery.filter((item) => (!style.parentKinds || style.parentKinds.includes('scenery'))
    && (!style.parentTypes || style.parentTypes.includes(item.style) || style.parentTypes.includes(styleById.get(item.style)?.type ?? '')));
  return carriersFor(compatibleTerrain, compatibleScenery);
}

export function ForestSceneryCards({ editable, onAdd }: { editable: boolean; onAdd: (style: string) => void }) {
  const [density, setDensity] = useState<SeedShellDensity>('sparse');
  const shellStyles = styles.filter(({ seedShellGroup }) => seedShellGroup === density);
  function card(style: SceneryStyle, hint: string) {
    const attachment = forestSceneryIsAttachment(style.id);
    const file = artwork[style.type]?.file;
    const url = file ? thumbnails['../../render/assets/r5/forest-composed/' + file] : undefined;
    return <button key={style.id} type="button" className="editor-object" data-forest-scenery-style={style.id}
      aria-label={attachment ? style.label : '放置' + style.label} disabled={!editable} onClick={() => onAdd(style.id)}>
      {url ? <img src={url} alt="" width="44" height="32" /> : <span aria-hidden="true" />}
      <span><strong>{style.label}</strong><small>{hint}</small></span>
    </button>;
  }
  return (
    <div className="editor-forest-scenery-cards" data-forest-scenery-cards>
      <h3>画面造景</h3>
      <div className="editor-content-options">
        {placeableStyles.map((style) => card(style, forestSceneryIsAttachment(style.id) ? '附着装饰' : '独立摆件'))}
      </div>
      <h3>地表点缀</h3>
      <label className="editor-forest-field">种壳组合
        <select data-forest-seed-shell-density value={density} disabled={!editable}
          onChange={(event) => setDensity(event.target.value as SeedShellDensity)}>
          {shellDensities.map((id) => <option value={id} key={id}>
            {seedShellGroups[id].label} · {seedShellGroups[id].spanMm} mm · {seedShellGroups[id].pieces.length} 枚
          </option>)}
        </select>
      </label>
      <p className="editor-context__hint">放在林下地面作点缀，种壳不计入食物或碎屑库存。</p>
      <div className="editor-content-options">
        {shellStyles.map((style) => card(style, '地表点缀'))}
      </div>
    </div>
  );
}

export function ForestSceneryAttachment({ style, carriers, parent, editable, onSelect, onPlace, onCancel }: {
  style: string;
  carriers: readonly Carrier[];
  parent: ForestSceneryParent | null;
  editable: boolean;
  onSelect: (parent: ForestSceneryParent | null) => void;
  onPlace: () => void;
  onCancel: () => void;
}) {
  const chosen = parent && carriers.some((option) => carrierKey(option) === carrierKey(parent)) ? carrierKey(parent) : '';
  return (
    <form className="editor-forest-attachment" data-forest-attachment-form onSubmit={(event) => {
      event.preventDefault();
      if (editable && chosen) onPlace();
    }}>
      <h3>{forestSceneryName(style)}</h3>
      <label className="editor-forest-field">承载物
        <select value={chosen} data-forest-attachment-carrier disabled={!editable || !carriers.length} onChange={(event) => {
          const carrier = carriers.find((option) => carrierKey(option) === event.target.value);
          onSelect(carrier ? { kind: carrier.kind, id: carrier.id } : null);
        }}>
          <option value="">选择承载物</option>
          {carriers.map((carrier) => <option key={carrierKey(carrier)} value={carrierKey(carrier)}>{carrier.label}</option>)}
        </select>
      </label>
      <p className="editor-context__hint">{carriers.length
        ? '附着后可以拖动装饰微调。移动承载物时，装饰会跟随。'
        : '先摆放可以承载该装饰的石块、木材或其他摆件。'}</p>
      <div className="editor-details__options">
        <button type="submit" className="editor-tool" data-forest-attachment-confirm disabled={!editable || !chosen}>确认附着</button>
        <button type="button" className="editor-tool" onClick={onCancel}>取消</button>
      </div>
    </form>
  );
}

function SceneryRemoval({ hasChildren, label, onRemove }: { hasChildren: boolean; label: string; onRemove: () => void }) {
  const [confirm, setConfirm] = useState(false);
  if (!confirm) return <button type="button" className="editor-tool" onClick={hasChildren ? () => setConfirm(true) : onRemove}>移除{label}</button>;
  return <div className="editor-forest-attachment">
    <p className="editor-context__hint">附着在上面的装饰将一同移除。可以撤销。</p>
    <div className="editor-details__options">
      <button type="button" className="editor-tool" onClick={() => { setConfirm(false); onRemove(); }}>确认移除{label}</button>
      <button type="button" className="editor-tool" onClick={() => setConfirm(false)}>取消移除</button>
    </div>
  </div>;
}

export function ForestSceneryProperties({ items, terrain, selected, editable, onSelect, onRemove }: {
  items: readonly ForestScenery[];
  terrain: readonly ForestTerrainObject[];
  selected?: ForestScenery;
  editable: boolean;
  onSelect: (id: number) => void;
  onRemove: (id: number) => void;
}) {
  const roots = items.filter(({ parent }) => !parent);
  const attachments = items.filter(({ parent }) => parent);
  const label = selected ? forestSceneryName(selected.style) : '';
  const density = selected && styleById.get(selected.style)?.seedShellGroup;
  const group = seedShellGroup(density);
  const parent = selected?.parent;
  const carrier = parent && carriersFor(terrain, items).find((option) => carrierKey(option) === carrierKey(parent));
  const hasChildren = selected != null && items.some(({ parent }) => parent?.kind === 'scenery' && parent.id === selected.id);
  const options = (list: readonly ForestScenery[]) => list.map((item) => (
    <option key={item.id} value={item.id}>{forestSceneryName(item.style)} {item.id}</option>
  ));
  return (
    <section className="editor-context__section" data-forest-scenery-inspector>
      <h2>画面造景</h2>
      {items.length > 0 ? <label className="editor-forest-field">选择摆件
        <select value={selected?.id ?? ''} data-forest-scenery-selector onChange={(event) => {
          if (event.target.value) onSelect(Number(event.target.value));
        }}>
          <option value="" disabled>从画面或列表选择</option>
          {roots.length > 0 && <optgroup label="独立摆件">{options(roots)}</optgroup>}
          {attachments.length > 0 && <optgroup label="附着装饰">{options(attachments)}</optgroup>}
        </select>
      </label> : <p className="editor-context__hint">从「内容」选择场景摆件。</p>}
      {selected && <div className="editor-context__properties" data-forest-scenery-id={selected.id}
        data-forest-scenery-parent={parent ? carrierKey(parent) : undefined}>
        <h3>{label} {selected.id}</h3>
        <dl>
          {carrier && <div><dt>承载物</dt><dd>{carrier.label}</dd></div>}
          {group && <div data-forest-seed-shell-group={density}><dt>地表点缀</dt><dd>{group.pieces.length} 枚 · 跨度 {group.spanMm} mm</dd></div>}
          <div><dt>{parent ? '横向偏移' : '距画面左侧'}</dt><dd>{coordinate.format(selected.x)}</dd></div>
          <div><dt>{parent ? '纵向偏移' : '距画面上侧'}</dt><dd>{coordinate.format(selected.y)}</dd></div>
        </dl>
        {editable && <div className="editor-details__options">
          <SceneryRemoval key={selected.id} hasChildren={hasChildren} label={label} onRemove={() => onRemove(selected.id)} />
        </div>}
      </div>}
      <p className="editor-context__hint">造景摆件只影响画面。遮蔽、通道和落叶库存由地形与资源决定。</p>
    </section>
  );
}
