import { useState } from 'react';
import type { ReactNode } from 'react';
import rockThumbnail from '../../render/assets/r5/forest-composed/boulder-round.png';
import logThumbnail from '../../render/assets/r5/forest-composed/ground-log.png';
import slabThumbnail from '../../render/assets/r5/forest-composed/rock.png';
import lichenStone from '../../render/assets/r5/forest-composed/lichen-rock.png';
import archThumbnail from '../../render/assets/r5/forest-composed/log-thick.png';
import rootArchThumbnail from '../../render/assets/r5/forest-composed/bridge-root.png';
import branchBridgeThumbnail from '../../render/assets/r5/forest-composed/bridge-branch.png';
import leavesThumbnail from '../../render/assets/r5/forest-composed/leaf.png';
import heapThumbnail from '../../render/assets/r5/forest-composed/gold-leaf.png';
import fragmentsThumbnail from '../../render/assets/r5/forest-composed/litter-fragments.png';
import forestComposition from '../../render/assets/r5/forest-composed/definition.json';
import type {
  BarkShelterPreset, ForestBarkShelter, ForestLeafPatch, ForestLeafPreset, ForestRock, ForestStonework, LeafPatchMatter,
  ForestSubstrateCell, LeafPatchPreset, StoneworkPreset,
} from '../../world';
import { observeForestMycelium } from '../../render/r5/forestMycelium';
import { forestSoilColumns } from '../../render/r5/forestPlacement';
import { displayMass, nutrients } from './formatMatter';

const percent = new Intl.NumberFormat('zh-CN', { style: 'percent', maximumFractionDigits: 1 });

export type ForestTerrainChoice =
  | { kind: 'rock' | 'log' }
  | { kind: 'stonework'; preset: StoneworkPreset }
  | { kind: 'barkShelter'; preset: BarkShelterPreset }
  | { kind: 'leafPatch'; preset: LeafPatchPreset };
export type ForestTerrainObject =
  | (ForestRock & { kind: 'rock' | 'log' })
  | (ForestStonework & { kind: 'stonework' })
  | (ForestBarkShelter & { kind: 'barkShelter' })
  | (ForestLeafPatch & { kind: 'leafPatch' });

const objects = {
  rock: { name: '苔斑岩石', hint: '50 mm · 放置到场景', thumbnail: rockThumbnail },
  log: { name: '朽木段', hint: '260 mm · 贴地横放', thumbnail: logThumbnail },
} as const;

const stonework = {
  slab: { name: '扁石', hint: '40 mm · 单片', size: '长度 40 mm', thumbnail: slabThumbnail },
  pair: { name: '两石组', hint: '100 mm · 两块', size: '跨度 100 mm', thumbnail: rockThumbnail },
  cluster: { name: '三石组', hint: '140 mm · 三块', size: '跨度 140 mm', thumbnail: rockThumbnail },
} as const;
const stoneworkPresets = ['slab', 'pair', 'cluster'] as const;
const barkShelters = {
  arch: { name: '短拱树皮', hint: '90 mm · 前后贯通', size: '轴长 90 mm', thumbnail: archThumbnail },
  split: { name: '裂口半筒', hint: '120 mm · 前后贯通', size: '轴长 120 mm', thumbnail: logThumbnail },
  rootArch: { name: '弯曲根拱', hint: '140 mm · 桥下贯通', size: '参考跨度 140 mm', thumbnail: rootArchThumbnail },
  branchBridge: { name: '短枝桥', hint: '200 mm · 桥下贯通', size: '参考跨度 200 mm', thumbnail: branchBridgeThumbnail },
} as const;
const barkPresets = ['arch', 'split'] as const;
const bridgePresets = ['rootArch', 'branchBridge'] as const;
const leafPatches = {
  leaves: { name: '落叶簇', hint: '55 mm · 薄铺', size: '叶片长 55 mm', thumbnail: leavesThumbnail },
  heap: { name: '落叶堆', hint: '105 mm · 小堆', size: '堆跨度 105 mm', thumbnail: heapThumbnail },
  fragments: { name: '碎叶', hint: '8–25 mm · 薄铺', size: '碎叶长 8–25 mm', thumbnail: fragmentsThumbnail },
} as const;
const leafPresets = ['leaves', 'heap'] as const;

function terrainDetail(choice: ForestTerrainChoice) {
  switch (choice.kind) {
    case 'stonework': return stonework[choice.preset];
    case 'barkShelter': return barkShelters[choice.preset];
    case 'leafPatch': return leafPatches[choice.preset];
    default: return objects[choice.kind];
  }
}

export function forestTerrainName(choice: ForestTerrainChoice) {
  return choice.kind === 'rock' ? '石块' : terrainDetail(choice).name;
}

function composedArtwork(item: ForestTerrainObject) {
  if (item.kind === 'rock') return { label: '圆灰岩', mm: forestComposition.physical['rock-main'].mm };
  if (item.kind === 'barkShelter' && item.preset === 'arch') return { label: '厚壁朽木', mm: forestComposition.physical['log-main'].mm };
  if (item.kind === 'leafPatch' && item.preset === 'heap') return { label: '金褐卷叶', mm: forestComposition.physical['leaf-gold'].mm };
  return undefined;
}

function TerrainThumbnail({ choice, mirrored }: { choice: ForestTerrainChoice; mirrored?: boolean }) {
  const group = choice.kind === 'stonework' && choice.preset !== 'slab' ? choice.preset : undefined;
  return (
    <span className="editor-terrain-thumb" data-stone-group={group} data-mirrored={mirrored || undefined} aria-hidden="true">
      <img src={terrainDetail(choice).thumbnail} alt="" width="44" height="32" />
      {group && <img src={slabThumbnail} alt="" width="23" height="18" />}
      {group === 'cluster' && <img src={lichenStone} alt="" width="23" height="18" />}
    </span>
  );
}

function LeafPatchLedger({ patch }: { patch: ForestLeafPatch }) {
  const stores: { key: 'initial' | 'remaining'; label: string; matter: LeafPatchMatter }[] = [
    { key: 'initial', label: '投放', matter: patch.initialMgCnp },
    { key: 'remaining', label: '剩余', matter: patch.remainingMgCnp },
  ];
  return (
    <>
      <table className="editor-forest editor-leaf-ledger" aria-label="落叶碳氮磷质量，单位毫克">
        <thead><tr><th scope="col">质量（mg）</th><th scope="col">碳 C</th><th scope="col">氮 N</th><th scope="col">磷 P</th></tr></thead>
        <tbody>
          {stores.map(({ key, label, matter }) => <tr key={key} data-leaf-stock={key}>
            <th scope="row">{label}</th>
            {matter.map((amount, index) => <td key={nutrients[index]} data-nutrient={nutrients[index]} data-mg={amount}>
              {displayMass(amount)}
            </td>)}
          </tr>)}
        </tbody>
      </table>
      <p className="editor-context__hint">落叶计入所在的一个土格。移动只转移剩余库存。</p>
    </>
  );
}

function LeafPatchRemoval({ name, onRemove }: { name: string; onRemove: () => void }) {
  const [confirm, setConfirm] = useState(false);
  return confirm ? (
    <div className="editor-leaf-removal">
      <p className="editor-context__hint">剩余落叶将被取出世界，已分解或被取食的部分不会被收回。</p>
      <div className="editor-details__options">
        <button type="button" className="editor-tool" onClick={() => { setConfirm(false); onRemove(); }}>确认取出</button>
        <button type="button" className="editor-tool" onClick={() => setConfirm(false)}>取消取出</button>
      </div>
    </div>
  ) : <button type="button" className="editor-tool" onClick={() => setConfirm(true)}>移除{name}</button>;
}

export function ForestTerrainProperties({ items, selected, hasEditableLogs, hasStonework, hasBarkShelters, hasLeafPatches,
  hasScenery, carriesScenery, substrateCells, editable, onSelect, onMirror, onRemove }: {
  items: ForestTerrainObject[];
  selected: ForestTerrainObject | undefined;
  hasEditableLogs: boolean;
  hasStonework: boolean;
  hasBarkShelters: boolean;
  hasLeafPatches: boolean;
  hasScenery?: boolean;
  carriesScenery?: boolean;
  substrateCells?: readonly ForestSubstrateCell[];
  editable: boolean;
  onSelect: (id: number) => void;
  onMirror: (id: number) => void;
  onRemove: (id: number) => void;
}) {
  const selectedName = selected ? forestTerrainName(selected) : '';
  const hasGroundObjects = hasEditableLogs || hasStonework || hasBarkShelters;
  const artwork = hasScenery && selected ? composedArtwork(selected) : undefined;
  const mycelium = selected && observeForestMycelium(substrateCells, selected);
  const sizeLabel = hasScenery ? '模拟规格' : '规格';
  return (
    <>
      <section className="editor-context__section">
        <h2>{hasScenery ? '地形与资源' : hasLeafPatches ? '场景对象' : hasGroundObjects ? '地面遮蔽物' : '遮蔽石块'}</h2>
        {items.length === 0 && <p className="editor-context__hint">{hasLeafPatches ? '场景内还没有可编辑对象。'
          : hasGroundObjects ? '场景内还没有地面遮蔽物。' : '场景内还没有石块。'}</p>}
        <div className="editor-rock-list">
          {items.map((item) => (
            <button key={item.id} type="button" className="editor-object" data-terrain-kind={item.kind} data-terrain-id={item.id}
              data-rock-id={item.kind === 'rock' ? item.id : undefined} data-log-id={item.kind === 'log' ? item.id : undefined}
              data-stonework-id={item.kind === 'stonework' ? item.id : undefined}
              data-stonework-preset={item.kind === 'stonework' ? item.preset : undefined}
              data-bark-shelter-id={item.kind === 'barkShelter' ? item.id : undefined}
              data-bark-preset={item.kind === 'barkShelter' ? item.preset : undefined}
              data-leaf-patch-id={item.kind === 'leafPatch' ? item.id : undefined}
              data-leaf-preset={item.kind === 'leafPatch' ? item.preset : undefined}
              aria-pressed={selected?.id === item.id} onClick={() => onSelect(item.id)}>
              <TerrainThumbnail choice={item} mirrored={item.mirrored} />
              <span>
                <strong>{forestTerrainName(item)} {item.id}</strong>
                {(item.kind === 'rock' || item.kind === 'log') && <small>{item.kind === 'log' ? '贴地木段' : objects.rock.name}</small>}
              </span>
            </button>
          ))}
        </div>
      </section>
      {selected && (
        <section className="editor-context__section editor-context__properties">
          <h2>{selectedName} {selected.id} 的位置与朝向</h2>
          <dl>
            {artwork && <div data-forest-terrain-art-mm={artwork.mm}><dt>画面素材</dt><dd>{artwork.label} · {artwork.mm} mm</dd></div>}
            {hasScenery && selected.kind === 'rock' && <div><dt>{sizeLabel}</dt><dd>标称 50 mm</dd></div>}
            {hasScenery && selected.kind === 'log' && <div><dt>{sizeLabel}</dt><dd>长度 260 mm</dd></div>}
            {selected.kind === 'stonework' && <div><dt>{sizeLabel}</dt><dd>{stonework[selected.preset].size}</dd></div>}
            {selected.kind === 'barkShelter' && <div><dt>{sizeLabel}</dt><dd>{barkShelters[selected.preset].size}</dd></div>}
            {selected.kind === 'barkShelter' && selected.bridgeClearanceMm &&
              <div><dt>桥下净宽、净高</dt><dd>{selected.bridgeClearanceMm.join(' × ')} mm</dd></div>}
            {selected.kind === 'leafPatch' && <div><dt>{hasScenery ? '模拟初始外形' : '初始外形'}</dt><dd>{leafPatches[selected.preset].size}</dd></div>}
            <div><dt>距画幅左侧</dt><dd>{percent.format(selected.position.x)}</dd></div>
            <div><dt>距画幅上侧</dt><dd>{percent.format(selected.position.y)}</dd></div>
            <div><dt>朝向</dt><dd>{selected.mirrored ? '镜像' : '原向'}</dd></div>
            {selected.kind === 'barkShelter' && <div className="editor-bark-occupants" data-occupant-count={selected.occupantIds.length}>
              <dt>{selected.bridgeClearanceMm ? '桥下鼠妇' : '洞内鼠妇'}</dt><dd>{selected.occupantIds.length} 只</dd>
            </div>}
          </dl>
          {mycelium && <details className="editor-context__detail" data-forest-mycelium-object={selected.id}
            data-forest-mycelium-cell={mycelium.cellIndex} data-forest-mycelium-span={mycelium.spanMm}>
            <summary>表面菌丝表现</summary>
            <dl>
              <div><dt>木材锚点土格</dt><dd>第 {Math.floor(mycelium.cellIndex / forestSoilColumns) + 1} 排 · 第 {mycelium.cellIndex % forestSoilColumns + 1} 列</dd></div>
              <div><dt>土格分解者库存</dt><dd>{displayMass(mycelium.decomposerMgC)} mg C</dd></div>
              <div><dt>纹饰跨度</dt><dd>{mycelium.spanMm ? `${mycelium.spanMm} mm` : '隐去'}</dd></div>
            </dl>
            <p className="editor-context__hint">以土格现有的分解者库存驱动木面纹饰。库存归土格，纹饰是画面示意。</p>
          </details>}
          {selected.kind === 'leafPatch' && <LeafPatchLedger patch={selected} />}
          {editable && carriesScenery && <p className="editor-context__hint">移除时会一同取下附着的造景。可以撤销。</p>}
          {editable && <div className="editor-details__options">
            <button type="button" className="editor-tool" onClick={() => onMirror(selected.id)}>水平翻转</button>
            {selected.kind === 'leafPatch'
              ? <LeafPatchRemoval key={selected.id} name={selectedName} onRemove={() => onRemove(selected.id)} />
              : <button type="button" className="editor-tool" onClick={() => onRemove(selected.id)}>移除{selectedName}</button>}
          </div>}
        </section>
      )}
    </>
  );
}

export function ForestTerrainDrawer({ hasEditableLogs, hasStonework, hasBarkShelters, hasLeafPatches, leafDefinitions,
  scenery, substrate, attachment, editable, leafBrush, onAdd, onClose }: {
  hasEditableLogs: boolean;
  hasStonework: boolean;
  hasBarkShelters: boolean;
  hasLeafPatches: boolean;
  leafDefinitions?: ForestLeafPreset[] | null;
  scenery?: ReactNode;
  substrate?: ReactNode;
  attachment?: ReactNode;
  editable: boolean;
  leafBrush?: { capacity: number; onStart: (preset: LeafPatchPreset, totalMgC?: number) => void };
  onAdd: (choice: ForestTerrainChoice) => void;
  onClose: () => void;
}) {
  const [fragmentDose, setFragmentDose] = useState('24');
  const totalMgC = Number(fragmentDose);
  const choices: ForestTerrainChoice[] = [{ kind: 'rock' }];
  if (hasStonework) choices.push(...stoneworkPresets.map((preset) => ({ kind: 'stonework' as const, preset })));
  if (hasEditableLogs) choices.push({ kind: 'log' });
  if (hasBarkShelters) choices.push(...barkPresets.map((preset) => ({ kind: 'barkShelter' as const, preset })));
  if (hasBarkShelters && scenery) choices.push(...bridgePresets.map((preset) => ({ kind: 'barkShelter' as const, preset })));
  if (hasLeafPatches) choices.push(...leafPresets.map((preset) => ({ kind: 'leafPatch' as const, preset })));
  return (
    <section className="editor-content-drawer" data-with-stonework={hasStonework || undefined}
      data-with-bark={hasBarkShelters || undefined} data-with-leaves={hasLeafPatches || undefined}
      data-with-forest-scenery={Boolean(scenery) || undefined} aria-labelledby="editor-content-title">
      <header>
        <h2 id="editor-content-title">{scenery ? '添加内容' : hasLeafPatches ? '石材、木材与落叶' : hasEditableLogs || hasBarkShelters ? '石材与木材' : '石材'}</h2>
        <button type="button" className="editor-tool" onClick={onClose}>收起内容</button>
      </header>
      {attachment ?? <>
        {scenery && <h3 className="editor-forest-drawer-heading">地形与资源</h3>}
        <div className="editor-content-options">
          {choices.map((choice) => {
            const detail = terrainDetail(choice);
            const dose = choice.kind === 'leafPatch' ? leafDefinitions?.find((definition) => definition.preset === choice.preset)?.initialMgCnp : undefined;
            const hint = choice.kind === 'leafPatch' && dose ? `${leafPatches[choice.preset].size} · ${displayMass(dose[0])} mg C` : detail.hint;
            return (
              <button key={detail.name} className="editor-object" type="button" aria-label={'放置' + detail.name}
                data-stonework-preset={choice.kind === 'stonework' ? choice.preset : undefined}
                data-bark-preset={choice.kind === 'barkShelter' ? choice.preset : undefined}
                data-leaf-preset={choice.kind === 'leafPatch' ? choice.preset : undefined}
                onClick={() => onAdd(choice)} disabled={!editable}>
                <TerrainThumbnail choice={choice} />
                <span><strong>{detail.name}</strong><small>{hint}</small></span>
              </button>
            );
          })}
        </div>
        {substrate}
        {leafBrush && <>
          <h3 className="editor-forest-drawer-heading">落叶笔刷</h3>
          <div className="editor-content-options">
            {leafPresets.map((preset) => {
              const detail = leafPatches[preset];
              const dose = leafDefinitions?.find((definition) => definition.preset === preset)?.initialMgCnp[0];
              return <button key={preset} type="button" className="editor-object" disabled={!editable || !leafBrush.capacity}
                onClick={() => leafBrush.onStart(preset)}>
                <TerrainThumbnail choice={{ kind: 'leafPatch', preset }} />
                <span><strong>刷{detail.name}</strong><small>{dose == null ? '每格一处' : `每格 ${displayMass(dose)} mg C`}</small></span>
              </button>;
            })}
          </div>
          <form className="editor-pond-form" onSubmit={(event) => {
            event.preventDefault();
            if (editable && leafBrush.capacity && totalMgC >= .1 && totalMgC <= 240) leafBrush.onStart('fragments', totalMgC);
          }}>
            <label className="editor-pond-field">碎叶总投放量（mg C）
              <input data-forest-fragment-dose type="number" value={fragmentDose} step="any" min="0.1" max="240" required
                onChange={(event) => setFragmentDose(event.currentTarget.value)} disabled={!editable || !leafBrush.capacity} />
            </label>
            <button data-forest-fragment-brush type="submit" className="editor-object" aria-label="铺设碎叶"
              disabled={!editable || !leafBrush.capacity || !(totalMgC >= .1 && totalMgC <= 240)}>
              <TerrainThumbnail choice={{ kind: 'leafPatch', preset: 'fragments' }} />
              <span><strong>铺设碎叶</strong><small>每笔的总量平分到有效土格</small></span>
            </button>
          </form>
          {!leafBrush.capacity && <p className="editor-context__hint">已达到落叶上限，取出一处后可继续铺设。</p>}
        </>}
        {scenery}
        {!editable && <p className="editor-context__hint">进入编辑后可放置。</p>}
      </>}
    </section>
  );
}
