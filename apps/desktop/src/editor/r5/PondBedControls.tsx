import { useState, type ReactNode } from 'react';
import mainThumbnail from '../../render/assets/r5/pond-composed/rock-dome-100.png';
import pebbleThumbnail from '../../render/assets/r5/pond-composed/pebble-16.png';
import slopeThumbnail from '../../render/assets/r5/pond-composed/wood-ridged.png';
import caveThumbnail from '../../render/assets/r5/pond-composed/flat-stone-cave.png';
import hangingRootsThumbnail from '../../render/assets/r5/pond-composed/hanging-roots.png';
import rootNetworkThumbnail from '../../render/assets/r5/pond-composed/root-network.png';
import type { PondBasinSnapshot, PondMossPatch, PondScenery, PondShrimpAttachment, PondStone, PondStonePreset, PondWood, PondWoodPreset } from '../../world';
import { pondStructureProfile } from '../../render/r5/pondStructureProfiles';
import { stoneWaterHeight } from '../../render/r5/pondStoneGeometry';

export type PondBedKind = 'stone' | 'wood';
export type PondBedChoice = { kind: 'stone'; preset: PondStonePreset } | { kind: 'wood'; preset: PondWoodPreset };
export type PondBedObject = (PondStone & { kind: 'stone' }) | (PondWood & { kind: 'wood' });

const stones = {
  main: { name: '主石', size: 80, thumbnail: mainThumbnail },
  pebble: { name: '卵石', size: 25, thumbnail: pebbleThumbnail },
  flatStoneCave: { name: '扁石洞', size: 110, thumbnail: caveThumbnail, label: '跨度' },
} as const;
const woods = {
  branch: { name: '主枝沉木', size: 300, thumbnail: slopeThumbnail },
  root: { name: '短根沉木', size: 150, thumbnail: rootNetworkThumbnail },
  slope: { name: '斜沉木', size: 420, thumbnail: slopeThumbnail },
  hangingRoots: { name: '稀疏悬根', size: 120, thumbnail: hangingRootsThumbnail, label: '跨度' },
  rootNetwork: { name: '盘根网', size: 180, thumbnail: rootNetworkThumbnail, label: '跨度' },
} as const;
const stoneChoices = [{ kind: 'stone', preset: 'main' }, { kind: 'stone', preset: 'pebble' }] as const;
const woodChoices = [{ kind: 'wood', preset: 'branch' }, { kind: 'wood', preset: 'root' }] as const;
const slopeChoice = { kind: 'wood', preset: 'slope' } as const;
const caveChoice = { kind: 'stone', preset: 'flatStoneCave' } as const;
const rootChoices = [{ kind: 'wood', preset: 'hangingRoots' }, { kind: 'wood', preset: 'rootNetwork' }] as const;
const percent = new Intl.NumberFormat('zh-CN', { style: 'percent', maximumFractionDigits: 1 });

function bedDetail(choice: PondBedChoice) {
  return choice.kind === 'stone' ? stones[choice.preset] : woods[choice.preset];
}

function bedSize(choice: PondBedChoice) {
  const detail = bedDetail(choice);
  return `${'label' in detail ? detail.label : choice.kind === 'stone' ? '长径' : '轴长'} ${detail.size} mm`;
}

export function pondBedName(choice: PondBedChoice) {
  return bedDetail(choice).name;
}

export function PondBedDrawer({ hasWoods, hasSlope, editable, supplement, scenery, onAdd, onClose }: {
  hasWoods: boolean;
  hasSlope?: boolean;
  editable: boolean;
  supplement?: ReactNode;
  scenery?: ReactNode;
  onAdd: (choice: PondBedChoice) => void;
  onClose: () => void;
}) {
  const choices: readonly PondBedChoice[] = hasWoods
    ? [...stoneChoices, ...(hasSlope ? [caveChoice] : []), ...woodChoices, ...(hasSlope ? [slopeChoice, ...rootChoices] : [])] : stoneChoices;
  return (
    <section className="editor-content-drawer" data-with-pond-wood={hasWoods || undefined}
      data-with-pond-moss={Boolean(supplement) || undefined} data-with-pond-scenery={Boolean(scenery) || undefined}
      aria-labelledby="editor-pond-rock-drawer">
      <header>
        <h2 id="editor-pond-rock-drawer">{scenery ? '浅塘造景' : supplement ? '池底造景与附着' : hasWoods ? '池底石材与沉木' : '池底石材'}</h2>
        <button type="button" className="editor-tool" onClick={onClose}>收起内容</button>
      </header>
      {scenery}
      {scenery && <h3 className="editor-pond-drawer-heading">池底石材与沉木</h3>}
      <div className="editor-content-options">
        {choices.map((choice) => {
          const item = bedDetail(choice);
          return (
            <button key={choice.preset} type="button" className="editor-object"
              data-pond-rock-preset={choice.kind === 'stone' ? choice.preset : undefined}
              data-pond-wood-preset={choice.kind === 'wood' ? choice.preset : undefined}
              aria-label={'放置' + item.name} onClick={() => onAdd(choice)} disabled={!editable}>
              <img src={item.thumbnail} alt="" width="44" height="32" />
              <span><strong>{item.name}</strong><small>{bedSize(choice)}</small></span>
            </button>
          );
        })}
        {supplement}
      </div>
      {!editable && <p className="editor-context__hint">进入编辑后可放置。</p>}
    </section>
  );
}

function PondCarrierRemoval({ kind, name, occupantCount, patchCount, decorationCount, staticSection, onRemove }: {
  kind: PondBedKind;
  name: string;
  occupantCount?: number;
  patchCount: number;
  decorationCount: number;
  staticSection: boolean;
  onRemove: () => void;
}) {
  const [confirm, setConfirm] = useState(false);
  return confirm ? (
    <div className={kind === 'wood' ? 'editor-pond-wood-removal' : 'editor-pond-carrier-removal'}>
      {kind === 'wood' && !staticSection && <p className="editor-context__hint">{occupantCount
        ? `移除沉木后，停靠的 ${occupantCount} 只小虾会离开木面。`
        : '移除沉木会让停靠的小虾离开木面。'}</p>}
      {patchCount > 0 && <p className="editor-context__hint">移除承载物后，附着斑块的剩余碳、氮、磷将转入这片水体的碎屑库存。</p>}
      {decorationCount > 0 && <p className="editor-context__hint">这段沉木上的附着装饰将一同移除。</p>}
      <div className="editor-details__options">
        <button type="button" className="editor-tool" onClick={() => { setConfirm(false); onRemove(); }}>确认移除{kind === 'wood' ? '沉木' : name}</button>
        <button type="button" className="editor-tool" onClick={() => setConfirm(false)}>取消移除</button>
      </div>
    </div>
  ) : <button type="button" className="editor-tool" onClick={() => setConfirm(true)}>移除{name}</button>;
}

export function PondBedProperties({ items, selected, water, hasWoods, attachments, mossPatches, scenery, editable, staticSection = false, onSelect, onRemove }: {
  items: PondBedObject[];
  selected: PondBedObject | undefined;
  water: PondBasinSnapshot;
  hasWoods: boolean;
  attachments?: PondShrimpAttachment[] | null;
  mossPatches?: PondMossPatch[];
  scenery?: readonly PondScenery[];
  editable: boolean;
  staticSection?: boolean;
  onSelect: (id: number, kind: PondBedKind) => void;
  onRemove: (id: number, kind: PondBedKind) => void;
}) {
  const occupantCount = selected?.kind === 'wood' && attachments
    ? attachments.filter(({ woodId }) => woodId === selected.id).length : undefined;
  const selectedName = selected ? pondBedName(selected) : '';
  const patchCount = selected ? mossPatches?.filter(({ parent }) => parent.kind === selected.kind && parent.id === selected.id).length ?? 0 : 0;
  const decorationCount = selected?.kind === 'wood' ? scenery?.filter(({ parent }) => parent?.kind === 'wood' && parent.id === selected.id).length ?? 0 : 0;
  const structure = selected && pondStructureProfile(selected.preset);
  const column = stoneWaterHeight(water);
  const immersion = !structure || column <= structure.bottom ? '露出' : column >= structure.top ? '浸没' : '部分浸没';
  return (
    <>
      <section className="editor-context__section">
        <h2>{hasWoods ? '池底物件' : '池底石材'}</h2>
        {items.length === 0 && <p className="editor-context__hint">从「内容」选择{hasWoods ? '石材或沉木' : '石材'}沿池底放置。</p>}
        {(['stone', 'wood'] as const).map((kind) => {
          const group = items.filter((item) => item.kind === kind);
          if (group.length === 0) return null;
          return (
            <div key={kind} className={kind === 'stone' ? 'editor-pond-rock-list' : 'editor-pond-wood-list'}>
              {group.map((item) => {
                const detail = bedDetail(item);
                return (
                  <button key={item.id} type="button" className="editor-object"
                    data-pond-rock-id={kind === 'stone' ? item.id : undefined}
                    data-pond-rock-preset={kind === 'stone' ? item.preset : undefined}
                    data-pond-wood-id={kind === 'wood' ? item.id : undefined}
                    data-pond-wood-preset={kind === 'wood' ? item.preset : undefined}
                    aria-pressed={selected?.kind === kind && selected.id === item.id}
                    onClick={() => onSelect(item.id, kind)}>
                    <img src={detail.thumbnail} alt="" width="44" height="32" />
                    <span><strong>{detail.name} {item.id}</strong><small>{bedSize(item)}</small></span>
                  </button>
                );
              })}
            </div>
          );
        })}
        <p className="editor-context__hint">{staticSection ? '石木和附着装饰属于当前剖面。'
          : `池底石影响虾的通行。${hasWoods ? '沉木提供停靠面。' : ''}水量和全池水质由环境模型计算。`}</p>
      </section>
      {selected && (
        <section className="editor-context__section editor-context__properties">
          <h2>{selectedName} {selected.id} 的池底位置</h2>
          <dl>
            <div><dt>距池底左侧</dt><dd>{percent.format(selected.x)}</dd></div>
            {structure && <div><dt>水线状态</dt><dd data-pond-structure-immersion={immersion}>{immersion}</dd></div>}
            {selected.kind === 'wood' && !structure && <>
              {!staticSection && <div className="editor-pond-wood-occupants" data-occupant-count={occupantCount}>
                <dt>停靠小虾</dt><dd>{occupantCount === undefined ? '—' : `${occupantCount} 只`}</dd>
              </div>}
              <div><dt>附着装饰</dt><dd>{selected.preset === 'slope' ? `${decorationCount} 件` : '苔藓纹饰'}</dd></div>
            </>}
          </dl>
          {structure && <p className="editor-context__hint">{structure.variant === 'flatStoneCave'
            ? '顶石与石脚决定洞口净空。洞口浸水时，米虾可从留出的水隙通过。'
            : '米虾可沿根枝之间的水隙通过。'} </p>}
          {editable && <div className="editor-details__options">
            {selected.kind === 'wood' || patchCount > 0
              ? <PondCarrierRemoval key={`${selected.kind}:${selected.id}`} kind={selected.kind} name={selectedName}
                  occupantCount={occupantCount} patchCount={patchCount} decorationCount={decorationCount} staticSection={staticSection}
                  onRemove={() => onRemove(selected.id, selected.kind)} />
              : <button type="button" className="editor-tool" onClick={() => onRemove(selected.id, 'stone')}>移除{selectedName}</button>}
          </div>}
        </section>
      )}
    </>
  );
}
