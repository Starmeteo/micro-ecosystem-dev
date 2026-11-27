import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, useState } from 'react';
import type { FormEvent, KeyboardEvent } from 'react';
import { WorldScene } from '../render/WorldScene';
import rockThumbnail from '../render/assets/r5/forest-composed/boulder-round.png';
import type { R5SampleFocus, R5SampleZoom } from '../render/r5/geometry';
import type { ForestBrushResult } from '../render/r5/forestBrushInput';
import { errorText, native, worldName } from '../world';
import type {
  ForestReadings, ForestRockAction, ForestRockEdit, ForestSceneryParent, ForestSubstrate, ForestTerrainKind, LeafPatchPreset, PondBasinSnapshot, PondEdit,
  PondLeafDepositPreset, PondMossParent, PondMossPreset, PondRibbonPlantPreset, PondStaticEdit, PondStaticSection, ScenePoint, SolarReadings, WorldLocation, WorldSnapshot,
} from '../world';
import { PondInspector } from './PondInspector';
import { PondBiofilmReadout } from './r5/PondBiofilmReadout';
import { PondDaphniaObservation } from './r5/PondDaphniaObservation';
import { ObservationTrends } from './r5/ObservationTrends';
import { AnimalIndividuals, DaphniaRemoval } from './r5/AnimalIndividuals';
import { SnailCandidateObservation } from './r5/SnailCandidateObservation';
import { PondWoodFeedingReadout } from './r5/PondWoodFeedingReadout';
import { PondStoneFeedingReadout } from './r5/PondStoneFeedingReadout';
import { PondShapeEditor } from './r5/PondShapeEditor';
import { PondFloatingCard, PondFloatingEditor, PondFloatingReadout } from './r5/PondFloatingEditor';
import { PondSedgeCard, PondSedgeEditor, SedgeReadout } from './r5/PondSedgeEditor';
import { WorldLibrary, type LibraryAction, type PresetEntry } from './WorldLibrary';
import {
  ForestTerrainDrawer, ForestTerrainProperties, forestTerrainName, type ForestTerrainChoice, type ForestTerrainObject,
} from './r5/ForestTerrainControls';
import {
  ForestSceneryAttachment, ForestSceneryCards, ForestSceneryProperties, forestSceneryCarriers, forestSceneryIsAttachment,
  forestSceneryName, forestSceneryOffset,
} from './r5/ForestSceneryControls';
import { PondBedDrawer, PondBedProperties, pondBedName, type PondBedChoice, type PondBedKind, type PondBedObject } from './r5/PondBedControls';
import { PondMossCards, PondMossPlacementForm, PondMossProperties, pondMossName, pondMossParentKey } from './r5/PondMossControls';
import { PondPlantCards, PondPlantProperties } from './r5/PondPlantControls';
import { leafDepositLimit, PondLeafDepositCard, PondLeafDepositProperties } from './r5/PondLeafControls';
import { waterPlantLimit } from '../render/r5/pondWaterPlant';
import { pondStructureProfile } from '../render/r5/pondStructureProfiles';
import { PondSceneryCards, PondSceneryProperties, pondSceneryIsBed, pondSceneryName } from './r5/PondSceneryControls';
import { displayMass, matterSummary } from './r5/formatMatter';
import { R5ObservationControls } from './r5/R5ObservationControls';
import { ForestSpringtailObservation } from './r5/ForestSpringtailObservation';
import type { ForestSpringtailView } from '../render/r5/forestSpringtails';
import { StockingControls, stockSpeciesName, type StockSpecies } from './r5/StockingControls';
import { ForestPlantCards, ForestPlantProperties } from './r5/ForestMossControls';
import { ForestSubstrateCards, ForestSubstrateReadings, forestSubstrateName } from './r5/ForestSubstrateControls';
import './WorldEditor.css';

interface WorldEditorProps {
  snapshot: WorldSnapshot | null;
  connectionError: string;
  viewActive: boolean;
}

type Authority = Pick<WorldSnapshot, 'worldId' | 'runGeneration'>;
type EditStamp = Authority & Pick<WorldSnapshot, 'simulationTick' | 'editRevision'>;
type EditHistory = { stamp: EditStamp; undo: ScenePoint[]; redo: ScenePoint[] };
type EditAction = 'move' | 'undo' | 'redo';
type LightPreview = 'scene' | 'day' | 'dusk' | 'night';
type Preset = 'forest' | 'pond';
type WorldCommand = 'set_paused' | 'set_edit_paused' | 'edit_rock' | 'save_world' | 'restore_world'
  | 'start_forest_trial' | 'advance_forest_trial' | 'advance_trial_scaled' | 'discard_forest_trial'
  | 'set_world_location' | 'create_forest_from_preset' | 'create_pond_from_preset'
  | 'edit_pond' | 'edit_pond_history' | 'switch_world' | 'create_world' | 'copy_world'
  | 'rename_world' | 'archive_world' | 'adopt_trial' | 'save_trial_as_world' | 'add_starter_life';
type EditorCommand = WorldCommand | 'hide_window' | 'save_world_preset' | 'edit_forest_rocks';
type Notice = { text: string; error?: boolean };
type Trial = { source: WorldSnapshot; snapshot: WorldSnapshot };
type TrialAction = 'adopt' | 'save' | null;
type TrialRate = '1x' | '10x' | '60x';
type LocationDraft = { name: string; latitude: string; longitude: string };
type InspectorView = 'object' | 'environment' | 'biology';
type EditorTask = 'scene' | 'inspector' | 'content';
type PondObjectKind = PondBedKind | 'plant' | 'waterPlant' | 'floatingPlant' | 'sedge' | 'scenery' | 'leafDeposit';
type PondMossPlacement = { basinId: string; preset: PondMossPreset; parent: PondMossParent | null; offset: number };
type ForestSceneryPlacement = { style: string } & ({ attachment: false } | { attachment: true; parent: ForestSceneryParent | null });
type StockPlacement = { species: 'woodlice'; count: number } | { species: 'shrimp'; basinId: string; count: number };
type ForestPlantKind = 'moss' | 'fern';
type ScenePlacement = { authority: Authority } & (
  { choice: ForestTerrainChoice } | { pond: PondBedChoice & { basinId: string } } | { moss: PondMossPlacement }
  | { scenery: { basinId: string; style: string } } | { forestScenery: ForestSceneryPlacement } | { leafBrush: LeafPatchPreset; totalMgC?: number }
  | { stocking: StockPlacement }
  | { forestPlant: { kind: ForestPlantKind; id?: number } }
  | { substrateBrush: ForestSubstrate }
  | { waterPlant: { basinId: string; ribbonPreset?: PondRibbonPlantPreset } }
  | { pondShape: { basinId: string } }
  | { floatingPlants: { basinId: string } }
  | { shoreSedges: { basinId: string } }
  | { leafDeposit: { basinId: string } & ({ preset: PondLeafDepositPreset; mgC: number } | { id: number }) }
);

const beijing: WorldLocation = { name: '北京（示例地点）', latitudeDegrees: 39.9042, longitudeDegrees: 116.4074 };
const seasonLabels = { winter: '冬', spring: '春', summer: '夏', autumn: '秋' };
const presetLabels = { forest: '林下', pond: '浅塘' };
const trialBatchSeconds: Record<TrialRate, number> = { '1x': 2, '10x': 20, '60x': 120 };
const inspectors = [
  { id: 'object', label: '属性', icon: 'M4 5.5h16v13H4z M8 9.5h8 M8 14.5h5 M11 7.5v4 M14 12.5v4' },
  { id: 'environment', label: '环境', icon: 'M3 16.5q3-2.6 6 0t6 0t6 0 M3 11.5q3-2.6 6 0t6 0t6 0 M12 4.2a2.2 2.2 0 1 0 0 4.4a2.2 2.2 0 1 0 0-4.4' },
  { id: 'biology', label: '生物', icon: 'M19.5 4.5c-8.5 0-14 3.8-14 9.5a4.8 4.8 0 0 0 4.8 4.8c5.7 0 9.2-5.5 9.2-14.3Z M4.5 19.5l10.5-10.5' },
] as const;
const terrainPlacementHints: Record<ForestTerrainKind | 'forestScenery' | 'forestAttachment' | 'forestMoss' | 'forestFern' | 'leafBrush' | 'substrateBrush' | 'stockWoodlice' | 'stockShrimp' | 'pondRock' | 'pondWood' | 'pondPlant' | 'pondScenery' | 'waterPlant' | 'pondShape' | 'floatingPlant' | 'leafDeposit', string> = {
  rock: '在地面选择落点。',
  log: '在场景中部选择一整段空地，避开石木和动物。',
  stonework: '在空地放置，避开动物和已有物件。',
  barkShelter: '选地面中部的空地，给前后入口留出通道。',
  leafPatch: '选择土格，避开已有物件和树皮洞口。',
  leafBrush: '按住在土格上拖动，松开一次铺设。琥珀色格会跳过。',
  substrateBrush: '拖动选土格，松开提交整笔。红格已有水量超过目标容量，整笔会取消。',
  stockWoodlice: '点选可通行地面，或用方向键移动标记后回车。',
  stockShrimp: '点选无遮挡的水下位置，或用方向键移动标记后回车。',
  forestMoss: '选一个可通行、没有活体苔藓的土格。',
  forestFern: '选一个没有蕨丛的土格，避开石木和树皮通道。',
  forestScenery: '在画面上选定摆件的位置。',
  forestAttachment: '在内容面板选择承载物。',
  pondRock: '沿池底放置，避开已有石材。',
  pondWood: '沿池底放置，给整段沉木留出空位。',
  pondPlant: '选择承载物和位置后确认附着。',
  waterPlant: '在池底选一处空位。琥珀色标记表示落点被占用或超出范围。',
  pondShape: '在俯视图里编辑池岸。',
  floatingPlant: '在俯视图里摆放或移动浮叶。',
  leafDeposit: '点击池底，或用方向键选点后回车确认。',
  pondScenery: '在画面上选定摆件的位置。',
};

const lights: { id: LightPreview; label: string }[] = [
  { id: 'scene', label: '世界光照' },
  { id: 'day', label: '白天' },
  { id: 'dusk', label: '黄昏' },
  { id: 'night', label: '夜间' },
];
const legacyLights = lights.filter(({ id }) => id !== 'dusk');
const lightPhases = { day: .5, dusk: .725, night: 0 } as const;
const coordinateFormat = new Intl.NumberFormat('zh-CN', { style: 'percent', maximumFractionDigits: 1 });
const beijingSolarTime = new Intl.DateTimeFormat('zh-CN', {
  month: 'long', day: 'numeric',
  timeZone: 'Asia/Shanghai', hour: '2-digit', minute: '2-digit', hourCycle: 'h23',
});

function sameAuthority(a: Authority | null | undefined, b: Authority | null | undefined): boolean {
  return Boolean(a && b && a.worldId === b.worldId && a.runGeneration === b.runGeneration);
}

const staticPondEditTypes = new Set<PondEdit['type']>([
  'addStone', 'moveStone', 'removeStone', 'addWood', 'moveWood', 'removeWood', 'addScenery', 'moveScenery', 'removeScenery',
  'addLeafDeposit', 'moveLeafDeposit', 'removeLeafDeposit',
  'addWaterPlant', 'placeRibbonPlant', 'moveWaterPlant', 'trimWaterPlant', 'removeWaterPlant',
  'stockShrimp', 'removeShrimp', 'enableStoneRest', 'enableStoneBiofilm', 'enableWoodBiofilm', 'addMoss', 'moveMoss', 'trimMoss', 'removeMoss',
] satisfies PondStaticEdit['type'][]);

function isPondStaticEdit(edit: PondEdit): edit is PondStaticEdit {
  return staticPondEditTypes.has(edit.type);
}

function projectPondStaticSection(basin: PondBasinSnapshot, section: PondStaticSection): PondBasinSnapshot {
  const { row, stones, woods, scenery, leafDeposits, waterPlantPatches = [] } = section;
  const filmStock = (patches?: readonly { stockMgC: number }[]) => (patches ?? []).reduce((sum, patch) => sum + patch.stockMgC, 0);
  const otherSectionFilm = (basin.staticSections ?? []).reduce((sum, item) =>
    item.row === row ? sum : sum + filmStock(item.stoneBiofilm) + filmStock(item.woodBiofilm), 0);
  return {
    ...basin, stones, woods, scenery,
    shape: basin.shape && { ...basin.shape, protectedRow: row },
    shrimp: section.shrimp ?? [], shrimpLife: section.shrimpLife, shrimpAttachments: section.shrimpAttachments,
    woodContactFeeding: section.woodContactFeeding, stoneRestEnabled: section.stoneRestEnabled,
    stoneAttachments: section.stoneAttachments, stoneContactFeeding: section.stoneContactFeeding,
    stoneBiofilmEnabled: section.stoneBiofilmEnabled, stoneBiofilm: section.stoneBiofilm,
    woodBiofilmEnabled: section.woodBiofilmEnabled, woodBiofilm: section.woodBiofilm,
    biofilmDistribution: section.biofilmDistribution,
    biofilmMgC: Math.max(0, basin.biofilmMgC - filmStock(basin.stoneBiofilm) - filmStock(basin.woodBiofilm) - otherSectionFilm),
    mossPatches: section.mossPatches ?? [], mossPresets: section.mossPresets ?? basin.mossPresets,
    waterPlantPatches, unassignedPlantMgCnp: [0, 0, 0],
    plantMgC: waterPlantPatches.reduce((total, patch) => total + patch.stockMgCnp[0], 0),
    leafDeposits, detritusMgC: 0,
  };
}

function newestSnapshot(subscription: WorldSnapshot | null, receipt: WorldSnapshot | null) {
  if (!subscription) return receipt;
  if (!receipt) return subscription;
  // The host sequence is shared across active-world switches.
  return BigInt(subscription.sequence) >= BigInt(receipt.sequence) ? subscription : receipt;
}

function usableHistory(history: EditHistory | null, snapshot: WorldSnapshot | null) {
  if (!history || !snapshot?.editPaused || !sameAuthority(history.stamp, snapshot)) return null;
  return history.stamp.editRevision === snapshot.editRevision
    && history.stamp.simulationTick === snapshot.simulationTick ? history : null;
}

function stamp(snapshot: WorldSnapshot): EditStamp {
  const { worldId, runGeneration, simulationTick, editRevision } = snapshot;
  return { worldId, runGeneration, simulationTick, editRevision };
}

function messageFor(error: unknown) {
  const code = error && typeof error === 'object' && 'code' in error ? error.code : error;
  if (code === 'stale_revision' || code === 'stale_generation') return '世界状态已更新。请查看最新读数后重试。';
  if (code === 'edit_requires_pause') return '世界正在运行。请先进入编辑。';
  if (code === 'checkpoint_missing') return '这个世界还没有已保存的检查点。';
  if (code === 'trial_missing') return '试运行副本已结束。请回到当前世界再试。';
  if (code === 'trial_conflict') return '正式世界在启动试运行后已发生变化。可以将结果另存为世界。';
  return errorText(error);
}

function WorldState({ snapshot, editing, trial }: { snapshot: WorldSnapshot | null; editing: boolean; trial: boolean }) {
  const waiting = !snapshot || snapshot.suspended || snapshot.paused;
  const mode = trial ? 'trial' : editing ? 'editing' : waiting ? 'waiting' : 'live';
  const text = !snapshot ? '连接中' : snapshot.suspended ? '系统暂停'
    : trial ? '试运行副本' : editing ? '编辑中 · 已暂停' : snapshot.paused ? '已暂停' : '生活中';

  return <span className="editor-state" data-waiting={waiting} data-mode={mode}>{native ? text : '浏览器预览'}</span>;
}

function ForestDiagnostics({ data, view, showSpringtails = true }: {
  data: ForestReadings; view: 'environment' | 'biology'; showSpringtails?: boolean;
}) {
  const isBiology = view === 'biology';
  const rows = isBiology ? [
    { label: '鼠妇', unit: '只', values: data.woodliceBySide },
    { label: '其中躲藏', unit: '只', values: data.shelteredWoodliceBySide },
    { label: '其中摄食', unit: '只', values: data.feedingWoodliceBySide },
  ] : [
    { label: '地表水量', unit: 'mL', values: data.waterMl.map((value) => value.toFixed(2)) },
    { label: '遮阴', unit: '', values: data.shadeFraction.map((value) => coordinateFormat.format(value)) },
    { label: '掩蔽', unit: '', values: data.shelterFraction.map((value) => coordinateFormat.format(value)) },
    { label: '蒸发速率', unit: 'mL/h', values: data.evaporationMlPerHour.map((value) => value.toFixed(2)) },
  ];
  const life = data.life;
  const juveniles = life?.woodlice.filter((animal) => animal.stage === 'juvenile').length;
  return (
    <section className="editor-context__section">
      <h2 id="editor-forest-title">{isBiology ? '林下生物' : '林下环境'}</h2>
      <table className="editor-forest" aria-labelledby="editor-forest-title">
        <thead><tr><th scope="col">指标</th><th scope="col">左侧</th><th scope="col">右侧</th></tr></thead>
        <tbody>
          {rows.map(({ label, unit, values }) => (
            <tr key={label}>
              <th scope="row">{label}{unit && <small>{unit}</small>}</th>
              <td>{values[0]}</td>
              <td>{values[1]}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {!isBiology && <details className="editor-details editor-details--small">
        <summary>枯落物与土壤养分</summary>
        <dl className="editor-stock">
          <div><dt>枯落物</dt><dd>{data.litterMgC.toFixed(2)} mg C</dd></div>
          <div><dt>可利用氮</dt><dd>{data.soilNitrogenMg.toFixed(2)} mg N</dd></div>
          <div><dt>可利用磷</dt><dd>{data.soilPhosphorusMg.toFixed(2)} mg P</dd></div>
        </dl>
      </details>}
      {isBiology && life && <details className="editor-details editor-details--small">
        <summary>生长与种群</summary>
        <dl className="editor-stock">
          <div><dt>鼠妇幼体</dt><dd>{juveniles} 只</dd></div>
          <div><dt>鼠妇累计出生／死亡</dt><dd>{life.woodliceBorn}／{life.woodliceDied} 只</dd></div>
          {showSpringtails && <div><dt>弹尾虫群</dt><dd>{(life.springtailMgC[0] + life.springtailMgC[1]).toFixed(2)} mg C</dd></div>}
          <div><dt>蕨类斑块</dt><dd>{(life.fernMgC[0] + life.fernMgC[1]).toFixed(2)} mg C</dd></div>
          <div><dt>苔藓斑块</dt><dd>{(life.mossMgC[0] + life.mossMgC[1]).toFixed(2)} mg C</dd></div>
        </dl>
      </details>}
    </section>
  );
}

function locationDraft(location: WorldLocation): LocationDraft {
  return { name: location.name, latitude: String(location.latitudeDegrees), longitude: String(location.longitudeDegrees) };
}

function SolarStatus({ data, phase, paused }: { data: SolarReadings; phase: number; paused: boolean }) {
  const offset = Math.abs((data.targetPhase - phase + 1.5) % 1 - .5);
  const rise = data.sunriseUtcSeconds === null ? '—' : beijingSolarTime.format(data.sunriseUtcSeconds * 1000);
  const set = data.sunsetUtcSeconds === null ? '—' : beijingSolarTime.format(data.sunsetUtcSeconds * 1000);
  const day = data.dayKind === 'continuousDay' ? '该地点今天持续白昼'
    : data.dayKind === 'continuousNight' ? '该地点今天无日出' : '日出 ' + rise + ' · 日落 ' + set + '（北京时间）';
  return (
    <div className="editor-solar">
      <p className="editor-context__hint">{day}</p>
      {offset > .002 && <p className="editor-context__hint">{paused ? '继续运行后将逐渐对齐' : '昼夜正在逐渐对齐'}</p>}
    </div>
  );
}

function TrialSummary({ trial, basinId }: { trial: Trial; basinId?: string }) {
  const original = trial.source;
  const result = trial.snapshot;
  const elapsed = Math.max(0, result.simulatedSeconds - original.simulatedSeconds);
  const beforeBasin = original.pond?.basins.find((entry) => entry.id === basinId) ?? original.pond?.basins[0];
  const afterBasin = result.pond?.basins.find((entry) => entry.id === basinId) ?? result.pond?.basins[0];
  const rows = beforeBasin && afterBasin ? [
    ['水量', `${beforeBasin.waterLiters.toFixed(2)} → ${afterBasin.waterLiters.toFixed(2)} L`],
    ['全池溶氧', `${beforeBasin.oxygenMgPerLiter?.toFixed(2) ?? '—'} → ${afterBasin.oxygenMgPerLiter?.toFixed(2) ?? '—'} mg/L`],
    ['水蚤', `${beforeBasin.daphniaCount} → ${afterBasin.daphniaCount} 只`],
  ] : [
    ['鼠妇', `${original.animals.length} → ${result.animals.length} 只`],
    ...(original.forest && result.forest ? [['枯落物', `${original.forest.litterMgC.toFixed(2)} → ${result.forest.litterMgC.toFixed(2)} mg C`]] : []),
  ];
  return (
    <section className="editor-context__section">
      <h2>试运行变化</h2>
      <p className="editor-context__hint">起点与当前副本对照</p>
      <dl className="editor-stock">
        <div><dt>已推进</dt><dd>{elapsed.toFixed(1)} 秒</dd></div>
        {rows.map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}
      </dl>
    </section>
  );
}

export function WorldEditor({ snapshot, connectionError, viewActive }: WorldEditorProps) {
  const [receipt, setReceipt] = useState<WorldSnapshot | null>(null);
  const [history, setHistory] = useState<EditHistory | null>(null);
  const [activeTrial, setActiveTrial] = useState<Trial | null>(null);
  const [trialAction, setTrialAction] = useState<TrialAction>(null);
  const [trialRate, setTrialRate] = useState<TrialRate>('60x');
  const [trialName, setTrialName] = useState('');
  const [libraryOpen, setLibraryOpen] = useState(false);
  const [selected, setSelected] = useState(false);
  const [pending, setPending] = useState<EditorCommand | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [saveFailures, setSaveFailures] = useState<Record<string, string>>({});
  const [light, setLight] = useState<LightPreview>('scene');
  const [appearance, setAppearance] = useState<'mist' | 'obsidian'>(() => {
    try {
      return window.localStorage.getItem('micro-eco-appearance') === 'obsidian' ? 'obsidian' : 'mist';
    } catch {
      return 'mist';
    }
  });
  const [renderer, setRenderer] = useState('');
  const [confirmRestore, setConfirmRestore] = useState(false);
  const [confirmCreate, setConfirmCreate] = useState<Preset | null>(null);
  const [basinId, setBasinId] = useState('');
  const [sectionSelection, setSectionSelection] = useState<{ authority: Authority; basinId: string; row: number } | null>(null);
  const [animalSelection, setAnimalSelection] = useState<{ authority: Authority; id: string; basinId?: string; row?: number } | null>(null);
  const [location, setLocation] = useState<LocationDraft | null>(null);
  const [inspector, setInspector] = useState<InspectorView>('object');
  const [task, setTask] = useState<EditorTask>('scene');
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [terrainSelection, setTerrainSelection] = useState<{
    authority: Authority; id: number; basinId?: string; sectionRow?: number; pondKind?: PondObjectKind; forestScenery?: true; forestPlant?: ForestPlantKind;
  } | null>(null);
  const [placement, setPlacement] = useState<ScenePlacement | null>(null);
  const [mossMoveOffset, setMossMoveOffset] = useState<number | null>(null);
  const [zoom, setZoom] = useState<R5SampleZoom>(1);
  const [focus, setFocus] = useState<R5SampleFocus>('center');
  const [showScale, setShowScale] = useState(false);
  const [springtailView, setSpringtailView] = useState<ForestSpringtailView>();
  const commandInFlight = useRef(false);
  const imageHost = useRef<HTMLDivElement>(null);
  const sceneControls = useRef<HTMLDivElement>(null);

  const world = newestSnapshot(snapshot, receipt);
  const trial = activeTrial;
  const editing = Boolean(world?.editPaused && !trial);
  const available = Boolean(native && world && viewActive && !world.suspended && !connectionError);
  const canAct = available && pending === null;
  const canMove = canAct && editing;
  const editableHistory = usableHistory(history, world);
  const observedWorld = trial?.snapshot ?? world;
  const forestRocks = observedWorld?.forestRocks;
  const forestScenery = Array.isArray(forestRocks?.scenery) ? forestRocks.scenery : undefined;
  const springtailSideFocus = Boolean(zoom > 1 && springtailView && springtailView !== 'both');
  const hasEditableLogs = Array.isArray(forestRocks?.logs);
  const hasStonework = Array.isArray(forestRocks?.stonework);
  const hasBarkShelters = Array.isArray(forestRocks?.barkShelters);
  const hasLeafPatches = Array.isArray(forestRocks?.leafPatches);
  const terrainObjects: ForestTerrainObject[] = [
    ...(forestRocks?.rocks ?? []).map((rock) => ({ ...rock, kind: 'rock' as const })),
    ...(forestRocks?.logs ?? []).map((log) => ({ ...log, kind: 'log' as const })),
    ...(forestRocks?.stonework ?? []).map((object) => ({ ...object, kind: 'stonework' as const })),
    ...(forestRocks?.barkShelters ?? []).map((object) => ({ ...object, kind: 'barkShelter' as const })),
    ...(forestRocks?.leafPatches ?? []).map((object) => ({ ...object, kind: 'leafPatch' as const })),
  ];
  const canTerrainEdit = Boolean(forestRocks && canAct && (editing || (trial && trial.snapshot.paused && !trialAction)));
  const forestMossPatches = forestRocks?.mossPatches;
  const forestMossPreset = forestRocks?.mossPreset;
  const forestFernPatches = forestRocks?.fernPatches;
  const forestFernPreset = forestRocks?.fernPreset;
  const selectedForestMoss = terrainSelection?.forestPlant === 'moss' && sameAuthority(observedWorld, terrainSelection.authority)
    ? forestMossPatches?.find(({ id }) => id === terrainSelection.id) : undefined;
  const selectedForestFern = terrainSelection?.forestPlant === 'fern' && sameAuthority(observedWorld, terrainSelection.authority)
    ? forestFernPatches?.find(({ id }) => id === terrainSelection.id) : undefined;
  const selectedTerrain = !terrainSelection?.forestScenery && !terrainSelection?.forestPlant && sameAuthority(observedWorld, terrainSelection?.authority)
    ? terrainObjects.find((object) => object.id === terrainSelection?.id) : undefined;
  const selectedForestScenery = terrainSelection?.forestScenery && sameAuthority(observedWorld, terrainSelection.authority)
    ? forestScenery?.find(({ id }) => id === terrainSelection.id) : undefined;
  const activePlacement = placement && sameAuthority(observedWorld, placement.authority) ? placement : null;
  const substrateCells = forestRocks?.substrateCells;
  const substratePresets = forestRocks?.substratePresets;
  const substrateBrush = activePlacement && 'substrateBrush' in activePlacement
    ? substratePresets?.find(({ substrate }) => substrate === activePlacement.substrateBrush) : undefined;
  const placementChoice = activePlacement && 'choice' in activePlacement ? activePlacement.choice : null;
  const forestPlantPlacement = activePlacement && 'forestPlant' in activePlacement ? activePlacement.forestPlant : null;
  const movingForestPlant = (forestPlantPlacement?.kind === 'fern' ? forestFernPatches : forestMossPatches)
    ?.find(({ id }) => id === forestPlantPlacement?.id);
  const leafBrush = activePlacement && 'leafBrush' in activePlacement ? activePlacement.leafBrush : null;
  const fragmentDose = activePlacement && 'leafBrush' in activePlacement ? activePlacement.totalMgC : undefined;
  const leafBrushCapacity = Math.max(0, 16 - (forestRocks?.leafPatches?.length ?? 0));
  const forestPlacement = forestScenery && activePlacement && 'forestScenery' in activePlacement ? activePlacement.forestScenery : null;
  const forestAttachment = forestPlacement?.attachment ? forestPlacement : null;
  const forestCarriers = forestScenery && forestAttachment ? forestSceneryCarriers(forestAttachment.style, terrainObjects, forestScenery) : [];
  const leafPreset = leafBrush ?? (placementChoice?.kind === 'leafPatch' ? placementChoice.preset : null);
  const placementLeafDose = forestRocks?.leafPresets?.find((definition) => definition.preset === leafPreset)?.initialMgCnp;
  const basins = observedWorld?.pond?.basins;
  const basinAuthority = basins?.find((entry) => entry.id === basinId) ?? basins?.[0];
  const staticSection = basinAuthority && sectionSelection?.basinId === basinAuthority.id && sameAuthority(observedWorld, sectionSelection.authority)
    ? basinAuthority.staticSections?.find(({ row }) => row === sectionSelection.row) : undefined;
  const basin = basinAuthority && staticSection ? projectPondStaticSection(basinAuthority, staticSection) : basinAuthority;
  const selectedAnimalId = animalSelection && sameAuthority(observedWorld, animalSelection.authority)
    && basin?.id === animalSelection.basinId && staticSection?.row === animalSelection.row
    ? (basin?.shrimp ?? observedWorld?.animals)?.find(({ id }) => id === animalSelection.id)?.id : undefined;
  const pondStones = basin && Array.isArray(basin.stones) ? basin.stones : undefined;
  const pondWoods = basin && Array.isArray(basin.woods) ? basin.woods : undefined;
  const pondMoss = basin && Array.isArray(basin.mossPatches) ? basin.mossPatches : undefined;
  const pondWaterPlants = basin && Array.isArray(basin.waterPlantPatches) ? basin.waterPlantPatches : undefined;
  const pondLeafDeposits = basin?.leafDeposits ?? [];
  const pondScenery = basin && Array.isArray(basin.scenery) ? basin.scenery : undefined;
  const stocking = activePlacement && 'stocking' in activePlacement && (
    activePlacement.stocking.species === 'woodlice' || activePlacement.stocking.basinId === basin?.id
  ) ? activePlacement.stocking : null;
  const pondBedObjects: PondBedObject[] = [
    ...(pondStones ?? []).map((stone) => ({ ...stone, kind: 'stone' as const })),
    ...(pondWoods ?? []).map((wood) => ({ ...wood, kind: 'wood' as const })),
  ];
  const pondTrialEditing = Boolean(trial?.snapshot.paused && pondStones && !trialAction);
  const canPondEdit = Boolean(basin && canAct && (editing || pondTrialEditing));
  const canPrimaryPondEdit = canPondEdit && !staticSection;
  const mossCarriers = pondBedObjects.filter((object) => !pondStructureProfile(object.preset) && (object.kind === 'wood' || object.preset === 'main')
    && !pondMoss?.some(({ parent }) => pondMossParentKey(object) === pondMossParentKey(parent)));
  const pondPlacement = activePlacement && 'pond' in activePlacement && activePlacement.pond.basinId === basin?.id
    ? activePlacement.pond : null;
  const mossPlacement = activePlacement && 'moss' in activePlacement && activePlacement.moss.basinId === basin?.id
    ? activePlacement.moss : null;
  const waterPlantPlacement = activePlacement && 'waterPlant' in activePlacement && activePlacement.waterPlant.basinId === basin?.id
    ? activePlacement.waterPlant : null;
  const pondShape = basin?.shape;
  const pondShapePlacement = pondShape && activePlacement && 'pondShape' in activePlacement
    && activePlacement.pondShape.basinId === basin?.id;
  const pondFloatingPlacement = pondShape && activePlacement && 'floatingPlants' in activePlacement
    && activePlacement.floatingPlants.basinId === basin?.id;
  const pondSedgePlacement = pondShape && activePlacement && 'shoreSedges' in activePlacement
    && activePlacement.shoreSedges.basinId === basin?.id;
  const pondMapOpen = pondShapePlacement || pondFloatingPlacement || pondSedgePlacement;
  const pendingLeaf = activePlacement && 'leafDeposit' in activePlacement && activePlacement.leafDeposit.basinId === basin?.id
    ? activePlacement.leafDeposit : null;
  const movingLeaf = pendingLeaf && 'id' in pendingLeaf ? pondLeafDeposits.find(({ id }) => id === pendingLeaf.id) : undefined;
  const newLeaf = pendingLeaf && 'preset' in pendingLeaf ? pendingLeaf : undefined;
  const leafPlacement = newLeaf ?? movingLeaf;
  const sceneryPlacement = activePlacement && 'scenery' in activePlacement && activePlacement.scenery.basinId === basin?.id
    ? activePlacement.scenery : null;
  const inSelectedBasin = sameAuthority(observedWorld, terrainSelection?.authority) && terrainSelection?.basinId === basin?.id
    && terrainSelection?.sectionRow === staticSection?.row;
  const selectedPondObject = inSelectedBasin
    ? pondBedObjects.find((object) => object.id === terrainSelection?.id && object.kind === terrainSelection?.pondKind) : undefined;
  const selectedMoss = inSelectedBasin && terrainSelection?.pondKind === 'plant'
    ? pondMoss?.find((patch) => patch.id === terrainSelection.id) : undefined;
  const selectedWaterPlant = inSelectedBasin && terrainSelection?.pondKind === 'waterPlant'
    ? pondWaterPlants?.find(({ id }) => id === terrainSelection.id) : undefined;
  const selectedFloatingPlant = inSelectedBasin && terrainSelection?.pondKind === 'floatingPlant'
    ? basin?.floatingPlants?.find(({ id }) => id === terrainSelection.id) : undefined;
  const selectedSedge = inSelectedBasin && terrainSelection?.pondKind === 'sedge'
    ? basin?.shoreSedges?.find(({ id }) => id === terrainSelection.id) : undefined;
  const selectedLeaf = inSelectedBasin && terrainSelection?.pondKind === 'leafDeposit'
    ? pondLeafDeposits.find(({ id }) => id === terrainSelection.id) : undefined;
  const selectedScenery = inSelectedBasin && terrainSelection?.pondKind === 'scenery'
    ? pondScenery?.find((item) => item.id === terrainSelection.id) : undefined;
  const mossDraft = mossPlacement?.parent
    ? { preset: mossPlacement.preset, parent: mossPlacement.parent, offset: mossPlacement.offset }
    : selectedMoss && mossMoveOffset !== null
      ? { id: selectedMoss.id, preset: selectedMoss.preset, parent: selectedMoss.parent, offset: mossMoveOffset } : undefined;
  const placementKind = forestPlantPlacement ? forestPlantPlacement.kind === 'fern' ? 'forestFern' : 'forestMoss'
    : stocking ? stocking.species === 'woodlice' ? 'stockWoodlice' : 'stockShrimp'
    : substrateBrush ? 'substrateBrush' : leafBrush ? 'leafBrush' : forestPlacement ? forestAttachment ? 'forestAttachment' : 'forestScenery'
    : pondShapePlacement ? 'pondShape' : pondFloatingPlacement ? 'floatingPlant' : leafPlacement ? 'leafDeposit'
    : sceneryPlacement ? 'pondScenery' : mossPlacement ? 'pondPlant' : waterPlantPlacement ? 'waterPlant'
    : pondPlacement ? pondPlacement.kind === 'wood' ? 'pondWood' : 'pondRock' : placementChoice?.kind;
  const placementPreset = substrateBrush?.substrate ?? leafBrush ?? leafPlacement?.preset ?? forestPlacement?.style ?? sceneryPlacement?.style ?? mossPlacement?.preset ?? pondPlacement?.preset
    ?? (placementChoice && 'preset' in placementChoice ? placementChoice.preset : undefined);
  const hasObservationControls = Boolean(forestRocks || basin);
  const season = observedWorld?.season;
  const hasDuskPreview = Boolean(pondScenery || forestScenery);
  const lightPreview = light === 'dusk' && !hasDuskPreview ? 'scene' : light;
  const lightOptions = hasDuskPreview ? lights : legacyLights;
  const litWorld = observedWorld && lightPreview !== 'scene' && !trial
    ? { ...observedWorld, daylightPhase: lightPhases[lightPreview] } : observedWorld;
  const shownWorld = litWorld && basins && basin && staticSection
    ? { ...litWorld, pond: { basins: basins.map((entry) => entry.id === basin.id ? basin : entry) } } : litWorld;

  useEffect(() => {
    if ((!placementKind && !pondSedgePlacement) || !viewActive || libraryOpen) return;
    function cancelPlacement(event: globalThis.KeyboardEvent) {
      if (event.key !== 'Escape' || event.defaultPrevented) return;
      setPlacement(null);
      event.preventDefault();
    }
    window.addEventListener('keydown', cancelPlacement);
    return () => window.removeEventListener('keydown', cancelPlacement);
  }, [placementKind, pondSedgePlacement, viewActive, libraryOpen]);

  async function execute<T>(
    command: EditorCommand,
    payload: Record<string, unknown> | undefined,
    applied: (result: T) => void,
  ) {
    if (!canAct || commandInFlight.current) return false;
    commandInFlight.current = true;
    setPending(command);
    setNotice(null);

    try {
      const result = await invoke<T>(command, payload);
      applied(result);
      return true;
    } catch (error) {
      const text = messageFor(error);
      if (command === 'save_world' && world) {
        setSaveFailures((previous) => ({ ...previous, [world.worldId]: text }));
      } else {
        setNotice({ text, error: true });
      }
      return false;
    } finally {
      commandInFlight.current = false;
      setPending(null);
    }
  }

  function dispatch(command: WorldCommand, payload: Record<string, unknown> | undefined, applied: (result: WorldSnapshot) => void) {
    return execute<WorldSnapshot>(command, payload, (result) => {
      if (command !== 'start_forest_trial' && command !== 'advance_forest_trial' && command !== 'advance_trial_scaled') setReceipt(result);
      applied(result);
    });
  }

  function acceptEditReceipt(target: WorldSnapshot, result: WorldSnapshot) {
    if (!sameAuthority(result, target)) return false;
    if (trial) {
      setActiveTrial((current) => current && sameAuthority(current.snapshot, result)
        && BigInt(result.editRevision) > BigInt(current.snapshot.editRevision) ? { ...current, snapshot: result } : current);
    } else {
      setReceipt(result);
    }
    return true;
  }

  function resetWorldTools() {
    setHistory(null);
    setSelected(false);
    setBasinId('');
    setSectionSelection(null);
    setLocation(null);
    setLight('scene');
    setConfirmRestore(false);
    setConfirmCreate(null);
    setTerrainSelection(null);
    setPlacement(null);
    setMossMoveOffset(null);
    setDrawerOpen(false);
    setInspector('object');
    setTask('scene');
    setZoom(1);
    setFocus('center');
    setSpringtailView(undefined);
  }

  function observeSpringtails(view?: ForestSpringtailView) {
    setSpringtailView(view);
    setPlacement(null);
    setDrawerOpen(false);
    setTask('scene');
    setZoom(view && view !== 'both' ? 4 : 1);
    setFocus('center');
    requestAnimationFrame(() => sceneControls.current?.querySelector<HTMLButtonElement>('[aria-pressed="true"]')?.focus({ preventScroll: true }));
  }

  function libraryAction(action: LibraryAction) {
    if (trial) return Promise.resolve(false);
    if (action.command === 'save_world_preset') {
      return execute<PresetEntry>(action.command, action.payload, (result) => {
        setNotice({ text: `已保存个人预设「${result.name}」` });
      });
    }
    return dispatch(action.command, action.payload, (result) => {
      if (action.command === 'rename_world') {
        setNotice({ text: `已重命名为「${action.payload.name.trim()}」` });
      } else if (action.command === 'archive_world') {
        setNotice({ text: action.payload.archived ? '世界已归档' : '世界已恢复到世界库' });
      } else {
        resetWorldTools();
        setLibraryOpen(false);
        setNotice({ text: `已打开「${worldName(result)}」` });
      }
    });
  }

  async function closeEditor() {
    if (commandInFlight.current) return;
    if (!native) {
      window.location.assign('?view=panel');
      return;
    }
    commandInFlight.current = true;
    setPending('hide_window');
    try {
      await invoke('hide_window', { viewId: 'editor' });
    } catch (error) {
      setNotice({ text: '无法关闭编辑器。' + messageFor(error), error: true });
    } finally {
      commandInFlight.current = false;
      setPending(null);
    }
  }

  function editWorld() {
    if (!world || !canAct || trial) return;
    void dispatch('set_edit_paused', { paused: true }, () => {
      setHistory(null);
    });
  }

  function finishEditing() {
    void dispatch('set_edit_paused', { paused: false }, (result) => {
      setHistory(null);
      setSelected(false);
      setPlacement(null);
      if (result.manualPaused) setNotice({ text: '已结束编辑，保留原先的手动暂停。' });
    });
  }

  function toggleManualPause() {
    if (!world || trial) return;
    void dispatch('set_paused', { paused: !world.paused }, () => {
      setHistory(null);
    });
  }

  function moveRock(destination: ScenePoint, action: EditAction = 'move') {
    if (!world || !canMove || world.pond) return;
    if (destination.x === world.rock.x && destination.y === world.rock.y) return;
    const { undo = [], redo = [] } = editableHistory ?? {};
    const origin = world.rock;

    void dispatch('edit_rock', {
      commandId: crypto.randomUUID(),
      expectedGeneration: world.runGeneration,
      expectedRevision: world.editRevision,
      x: destination.x,
      y: destination.y,
    }, (result) => {
      if (!sameAuthority(result, world)) {
        setHistory(null);
        return;
      }
      setSelected(true);
      const next = stamp(result);
      if (action === 'undo') {
        setHistory({ stamp: next, undo: undo.slice(0, -1), redo: [...redo, origin] });
        setNotice({ text: '已撤销石块移动' });
      } else if (action === 'redo') {
        setHistory({ stamp: next, undo: [...undo, origin], redo: redo.slice(0, -1) });
        setNotice({ text: '已重做石块移动' });
      } else {
        setHistory({ stamp: next, undo: [...undo, origin], redo: [] });
        setNotice({ text: '石块位置已更新' });
      }
    });
  }

  function selectTerrainObject(id: number | null) {
    setTerrainSelection(id === null || !observedWorld ? null : { authority: observedWorld, id });
    if (id !== null) {
      setPlacement(null);
      setInspector('object');
    }
  }

  function beginScenePlacement(next: ScenePlacement, destination: 'scene' | 'content' = 'scene') {
    setPlacement(next);
    setTerrainSelection(null);
    setInspector('object');
    setDrawerOpen(destination === 'content');
    setTask(destination);
    setZoom(1);
    setFocus('center');
    setNotice(null);
    if (destination === 'scene') requestAnimationFrame(() => imageHost.current?.querySelector('canvas')?.focus({ preventScroll: true }));
  }

  function beginTerrainPlacement(choice: ForestTerrainChoice) {
    if (!canTerrainEdit || !observedWorld || (choice.kind === 'log' && !hasEditableLogs)
      || (choice.kind === 'stonework' && !hasStonework) || (choice.kind === 'barkShelter' && !hasBarkShelters)
      || (choice.kind === 'leafPatch' && !hasLeafPatches)) return;
    beginScenePlacement({ authority: observedWorld, choice });
  }

  function beginLeafBrush(preset: LeafPatchPreset, totalMgC?: number) {
    if (!canTerrainEdit || !observedWorld || !forestScenery || !hasLeafPatches || !leafBrushCapacity) return;
    beginScenePlacement({ authority: observedWorld, leafBrush: preset, totalMgC });
  }

  function beginSubstrateBrush(substrate: ForestSubstrate) {
    if (!canTerrainEdit || !observedWorld || !substrateCells || !substratePresets?.some((preset) => preset.substrate === substrate)) return;
    beginScenePlacement({ authority: observedWorld, substrateBrush: substrate });
  }

  function beginStocking(species: StockSpecies, count: number) {
    if (!observedWorld) return;
    if (species === 'woodlice') {
      if (canTerrainEdit && forestScenery) beginScenePlacement({ authority: observedWorld, stocking: { species, count } });
      return;
    }
    if (!canPondEdit || !pondScenery || !basin?.wet) return;
    if (species === 'daphnia') {
      if (canPrimaryPondEdit) editPond({ type: 'stockDaphnia', basinId: basin.id, count });
    }
    else beginScenePlacement({ authority: observedWorld, stocking: { species, basinId: basin.id, count } });
  }

  function selectForestPlant(kind: ForestPlantKind, id: number) {
    if (!observedWorld) return;
    setTerrainSelection({ authority: observedWorld, id, forestPlant: kind });
    setPlacement(null);
    setInspector('object');
  }

  function beginForestPlant(kind: ForestPlantKind, id?: number) {
    const preset = kind === 'fern' ? forestFernPreset : forestMossPreset;
    if (!canTerrainEdit || !observedWorld || !preset) return;
    beginScenePlacement({ authority: observedWorld, forestPlant: { kind, id } });
  }

  function finishForestPlant(position: ScenePoint) {
    if (!forestPlantPlacement || (forestPlantPlacement.id !== undefined && !movingForestPlant)) return;
    const fern = forestPlantPlacement.kind === 'fern';
    if (fern && movingForestPlant?.position.x === position.x && movingForestPlant.position.y === position.y) return;
    editForestRocks({ action: 'change', edit: movingForestPlant
      ? { kind: fern ? 'moveFernPatch' : 'moveMossPatch', id: movingForestPlant.id, position }
      : { kind: fern ? 'addFernPatch' : 'addMossPatch', position } });
  }

  function finishStocking(position: ScenePoint) {
    if (!stocking) return;
    if (stocking.species === 'woodlice') {
      editForestRocks({ action: 'change', edit: { kind: 'stockWoodlice', count: stocking.count, position } });
    } else {
      editPond({ type: 'stockShrimp', basinId: stocking.basinId, count: stocking.count, position });
    }
  }

  function finishLeafBrush({ positions, skipped, outside }: ForestBrushResult) {
    if (!leafBrush) return;
    const skippedParts = [...(skipped ? [`${skipped} 格受占用或数量上限限制`] : []), ...(outside ? ['超出可铺设土面的部分'] : [])];
    const detail = skippedParts.length ? `，已跳过${skippedParts.join('及')}` : '';
    if (!positions.length) {
      setNotice({ text: `本笔没有铺设落叶${detail}。`, error: true });
      return;
    }
    const preset = leafBrush;
    const edit: ForestRockEdit = fragmentDose === undefined ? { kind: 'paintLeafPatches', preset, positions }
      : { kind: 'spreadLeafFragments', positions, mgC: fragmentDose };
    editForestRocks({ action: 'change', edit }, (result) => {
      const count = (result.forestRocks?.leafPatches?.length ?? 0) - (forestRocks?.leafPatches?.length ?? 0);
      const dose = fragmentDose === undefined ? '' : `，总投放 ${displayMass(fragmentDose)} mg C`;
      setNotice({ text: `已在 ${count} 格铺设${forestTerrainName({ kind: 'leafPatch', preset })}${dose}${detail}。` });
      requestAnimationFrame(() => imageHost.current?.querySelector('canvas')?.focus({ preventScroll: true }));
    });
  }

  function finishSubstrateBrush({ changedCells, skipped, outside }: ForestBrushResult) {
    if (!substrateBrush) return;
    if (skipped) {
      setNotice({ text: `本笔有 ${skipped} 格水量超过目标容量 ${substrateBrush.waterCapacityMl} mL，未更换任何土格。`, error: true });
      return;
    }
    if (!changedCells.length) {
      setNotice({ text: outside ? '本笔未覆盖需要更换的土格。' : '选中的土格已经是该基质。' });
      return;
    }
    const { substrate, waterCapacityMl } = substrateBrush;
    editForestRocks({ action: 'change', edit: { kind: 'setSubstrate', substrate, cells: changedCells } }, (result) => {
      const count = result.forestRocks?.substrateCells?.filter((cell) => cell.substrate === substrate && changedCells.includes(cell.cellIndex)).length ?? 0;
      setNotice({ text: `已将 ${count} 格改为${forestSubstrateName(substrate)}，每格容量 ${waterCapacityMl} mL，原有水量保留。` });
      requestAnimationFrame(() => imageHost.current?.querySelector('canvas')?.focus({ preventScroll: true }));
    });
  }

  function selectForestScenery(id: number | null) {
    setTerrainSelection(id === null || !observedWorld ? null : { authority: observedWorld, id, forestScenery: true });
    if (id !== null) {
      setPlacement(null);
      setInspector('object');
    }
  }

  function beginForestScenery(style: string) {
    if (!canTerrainEdit || !observedWorld || !forestScenery) return;
    if (!forestSceneryIsAttachment(style)) {
      beginScenePlacement({ authority: observedWorld, forestScenery: { style, attachment: false } });
      return;
    }
    const options = forestSceneryCarriers(style, terrainObjects, forestScenery);
    const preferred = options.find((option) => option.kind === 'terrain' ? selectedTerrain?.id === option.id : selectedForestScenery?.id === option.id)
      ?? (options.length === 1 ? options[0] : undefined);
    const parent = preferred ? { kind: preferred.kind, id: preferred.id } : null;
    beginScenePlacement({ authority: observedWorld, forestScenery: { style, attachment: true, parent } }, 'content');
  }

  function chooseForestCarrier(parent: ForestSceneryParent | null) {
    setPlacement((current) => current && 'forestScenery' in current && current.forestScenery.attachment
      ? { ...current, forestScenery: { ...current.forestScenery, parent } } : current);
  }

  function confirmForestScenery(x: number, y: number) {
    if (forestPlacement && !forestPlacement.attachment) editForestRocks({
      action: 'change', edit: { kind: 'addScenery', style: forestPlacement.style, x, y },
    });
  }

  function confirmForestAttachment() {
    const parent = forestAttachment?.parent;
    if (!forestAttachment || !parent || !forestCarriers.some((candidate) => candidate.kind === parent.kind && candidate.id === parent.id)) return;
    const { style } = forestAttachment;
    editForestRocks({ action: 'change', edit: { kind: 'addScenery', style, parent, ...forestSceneryOffset(style) } });
  }

  function selectPondBasin(id: string) {
    setBasinId(id);
    setSectionSelection(null);
    setTerrainSelection(null);
    setPlacement(null);
    setMossMoveOffset(null);
  }

  function selectPondSection(row: string) {
    if (!basinAuthority || !observedWorld) return;
    const next = basinAuthority.staticSections?.find((section) => String(section.row) === row);
    setSectionSelection(next ? { authority: observedWorld, basinId: basinAuthority.id, row: next.row } : null);
    setTerrainSelection(null);
    setPlacement(null);
    setMossMoveOffset(null);
    setInspector('object');
  }

  function beginPondShape() {
    if (!canPrimaryPondEdit || !observedWorld || !basin?.shape) return;
    beginScenePlacement({ authority: observedWorld, pondShape: { basinId: basin.id } });
    setInspector('environment');
  }

  function beginFloatingPlants() {
    if (!canPrimaryPondEdit || !observedWorld || !basin?.shape || !pondScenery || (!basin.wet && !basin.floatingPlants?.length)) return;
    beginScenePlacement({ authority: observedWorld, floatingPlants: { basinId: basin.id } });
    if (selectedFloatingPlant) selectFloatingPlant(selectedFloatingPlant.id);
  }

  function selectFloatingPlant(id: number) {
    if (observedWorld && basin) setTerrainSelection({ authority: observedWorld, id, basinId: basin.id, pondKind: 'floatingPlant' });
  }

  function beginShoreSedges() {
    if (!canPrimaryPondEdit || !observedWorld || !basin?.shape || !pondScenery) return;
    beginScenePlacement({ authority: observedWorld, shoreSedges: { basinId: basin.id } });
  }

  function selectSedge(id: number) {
    if (observedWorld && basin) setTerrainSelection({ authority: observedWorld, id, basinId: basin.id, pondKind: 'sedge' });
  }

  function closePondMap() {
    setPlacement(null);
    requestAnimationFrame(() => imageHost.current?.querySelector('canvas')?.focus({ preventScroll: true }));
  }

  function selectPondObject(id: number | null, kind: PondObjectKind = 'stone') {
    if (mossPlacement) {
      const carrier = mossCarriers.find((object) => object.id === id && object.kind === kind);
      if (carrier) changeMossPlacement({ parent: { kind: carrier.kind, id: carrier.id }, offset: 0 });
      return;
    }
    setTerrainSelection(id === null || !observedWorld || !basin ? null : {
      authority: observedWorld, id, basinId: basin.id, sectionRow: staticSection?.row, pondKind: kind,
    });
    setMossMoveOffset(null);
    if (id !== null) {
      setPlacement(null);
      setInspector('object');
    }
  }

  function beginPondPlacement(choice: PondBedChoice) {
    if (!canPondEdit || !observedWorld || !basin || !pondStones || (choice.kind === 'wood' && !pondWoods)) return;
    if ((choice.preset === 'slope' || pondStructureProfile(choice.preset)) && !pondScenery) return;
    beginScenePlacement({ authority: observedWorld, pond: { basinId: basin.id, ...choice } });
  }

  function confirmPondPlacement(x: number) {
    if (!pondPlacement) return;
    const { kind, basinId, preset } = pondPlacement;
    editPond(kind === 'wood' ? { type: 'addWood', basinId, x, preset } : { type: 'addStone', basinId, x, preset });
  }

  function beginWaterPlant(ribbonPreset?: PondRibbonPlantPreset) {
    if (!canPondEdit || !observedWorld || !basin?.wet || !pondWaterPlants || pondWaterPlants.length >= waterPlantLimit) return;
    beginScenePlacement({ authority: observedWorld, waterPlant: { basinId: basin.id, ribbonPreset } });
  }

  function confirmWaterPlant(x: number) {
    if (!waterPlantPlacement) return;
    const { basinId, ribbonPreset } = waterPlantPlacement;
    editPond(ribbonPreset ? { type: 'placeRibbonPlant', basinId, x, preset: ribbonPreset } : { type: 'addWaterPlant', basinId, x });
  }

  function beginLeafDeposit(selection: { preset: PondLeafDepositPreset; mgC: number } | { id: number }) {
    if (!canPondEdit || !observedWorld || !basin || !pondScenery) return;
    if ('id' in selection ? !pondLeafDeposits.some(({ id }) => id === selection.id) : pondLeafDeposits.length >= leafDepositLimit) return;
    beginScenePlacement({ authority: observedWorld, leafDeposit: { basinId: basin.id, ...selection } });
  }

  function confirmLeafDeposit(x: number) {
    if (!basin) return;
    if (newLeaf) editPond({ type: 'addLeafDeposit', basinId: basin.id, preset: newLeaf.preset, mgC: newLeaf.mgC, x });
    else if (movingLeaf && movingLeaf.x !== x) editPond({ type: 'moveLeafDeposit', basinId: basin.id, id: movingLeaf.id, x });
    else setPlacement(null);
  }

  function adjustWaterPlant() {
    setDrawerOpen(false);
    setTask('scene');
    requestAnimationFrame(() => imageHost.current?.querySelector('canvas')?.focus({ preventScroll: true }));
  }

  function beginPondScenery(style: string) {
    if (!canPondEdit || !observedWorld || !basin || !pondScenery) return;
    beginScenePlacement({ authority: observedWorld, scenery: { basinId: basin.id, style } });
  }

  function confirmPondScenery(x: number, y: number) {
    if (sceneryPlacement) editPond({ type: 'addScenery', ...sceneryPlacement, x, y });
  }

  function beginMossPlacement(preset: PondMossPreset) {
    if (!canPondEdit || !observedWorld || !basin?.wet || !pondMoss) return;
    const singleCarrier = mossCarriers.length === 1 ? mossCarriers[0] : undefined;
    const parent = singleCarrier ? { kind: singleCarrier.kind, id: singleCarrier.id } : null;
    beginScenePlacement({ authority: observedWorld, moss: { basinId: basin.id, preset, parent, offset: 0 } }, 'content');
  }

  function changeMossPlacement(change: Partial<Pick<PondMossPlacement, 'offset' | 'parent'>>) {
    setPlacement((current) => current && 'moss' in current ? { ...current, moss: { ...current.moss, ...change } } : current);
  }

  function confirmMossPlacement() {
    if (!mossPlacement?.parent) return;
    const { basinId, preset, parent, offset } = mossPlacement;
    editPond({ type: 'addMoss', basinId, preset, parent, offset });
  }

  function confirmTerrainPlacement(position: ScenePoint) {
    if (!placementChoice) return;
    let edit: ForestRockEdit;
    switch (placementChoice.kind) {
      case 'leafPatch': edit = { kind: 'addLeafPatch', position, preset: placementChoice.preset }; break;
      case 'barkShelter': edit = { kind: 'addBarkShelter', position, preset: placementChoice.preset }; break;
      case 'stonework': edit = { kind: 'addStonework', position, preset: placementChoice.preset }; break;
      case 'log': edit = { kind: 'addLog', position }; break;
      default: edit = { kind: 'add', position }; break;
    }
    editForestRocks({ action: 'change', edit });
  }

  function editForestRocks(action: ForestRockAction, onPainted?: (result: WorldSnapshot) => void) {
    if (!canTerrainEdit || !observedWorld || !forestRocks) return;
    const target = observedWorld;
    void execute<WorldSnapshot>('edit_forest_rocks', {
      worldId: target.worldId,
      commandId: crypto.randomUUID(),
      expectedGeneration: target.runGeneration,
      expectedRevision: target.editRevision,
      action,
    }, (result) => {
      if (!acceptEditReceipt(target, result)) return;
      const edit = action.action === 'change' ? action.edit : null;
      if (!edit) {
        const step = action.action === 'undo' ? '撤销' : '重做';
        setNotice({ text: hasStonework || hasBarkShelters || hasLeafPatches ? `已${step}场景布置` : `已${step}${hasEditableLogs ? '石木' : '石块'}修改` });
        return;
      }
      if (edit.kind === 'paintLeafPatches' || edit.kind === 'spreadLeafFragments' || edit.kind === 'setSubstrate') {
        onPainted?.(result);
        return;
      }
      if (edit.kind === 'stockWoodlice') {
        setPlacement(null);
        setInspector('biology');
        setTask('inspector');
        setNotice({ text: `已投放 ${edit.count} 只成体鼠妇。` });
        return;
      }
      if (edit.kind === 'removeWoodlouse') {
        setAnimalSelection(null);
        setNotice({ text: '已取出鼠妇，碳氮磷按外部输出记账。' });
        return;
      }
      if (edit.kind === 'addMossPatch' || edit.kind === 'moveMossPatch' || edit.kind === 'trimMossPatch' || edit.kind === 'removeMossPatch') {
        const id = edit.kind === 'addMossPatch' ? result.forestRocks?.mossPatches?.find((item) =>
          !forestMossPatches?.some((prior) => prior.id === item.id))?.id : edit.id;
        if (id !== undefined && edit.kind !== 'removeMossPatch') setTerrainSelection({ authority: result, id, forestPlant: 'moss' });
        else setTerrainSelection(null);
        setPlacement(null);
        setInspector('object');
        setTask('inspector');
        const text = { addMossPatch: '已种植地表苔藓。', moveMossPatch: '已移植同一斑块，库存随它移动。',
          trimMossPatch: '修剪量已转入所在土格的枯落物。', removeMossPatch: '已把地表苔藓及其剩余库存取出世界。' };
        setNotice({ text: text[edit.kind] });
        return;
      }
      if (edit.kind === 'addFernPatch' || edit.kind === 'moveFernPatch' || edit.kind === 'trimFernPatch' || edit.kind === 'removeFernPatch') {
        const id = edit.kind === 'addFernPatch' ? result.forestRocks?.fernPatches?.find((item) =>
          !forestFernPatches?.some((prior) => prior.id === item.id))?.id : edit.id;
        setTerrainSelection(id !== undefined && edit.kind !== 'removeFernPatch' ? { authority: result, id, forestPlant: 'fern' } : null);
        setPlacement(null);
        setInspector('object');
        setTask('inspector');
        const text = { addFernPatch: '已种植小型蕨丛。', moveFernPatch: '已移植同一蕨丛，库存随它移动。',
          trimFernPatch: '修剪量已转入所在土格的枯落物。', removeFernPatch: '已把蕨丛及其剩余库存取出世界。' };
        setNotice({ text: text[edit.kind] });
        return;
      }
      if (edit.kind === 'addScenery') {
        const added = result.forestRocks?.scenery?.find(({ id }) => !forestScenery?.some((item) => item.id === id));
        if (added) setTerrainSelection({ authority: result, id: added.id, forestScenery: true });
        setPlacement(null);
        setDrawerOpen(false);
        setTask('scene');
        const name = forestSceneryName(edit.style);
        setNotice({ text: edit.parent ? `${name}已附着到承载物` : `已放置${name}` });
        return;
      }
      if (edit.kind === 'moveScenery' || edit.kind === 'removeScenery') {
        const item = forestScenery?.find(({ id }) => id === edit.id);
        const name = item ? forestSceneryName(item.style) : '林下摆件';
        if (edit.kind === 'removeScenery') setTerrainSelection(null);
        setNotice({ text: edit.kind === 'removeScenery' ? `已移除${name}` : `${name}位置已更新` });
        return;
      }
      if (!('id' in edit)) {
        const layout = result.forestRocks;
        const addedItems = {
          add: layout?.rocks, addLog: layout?.logs, addStonework: layout?.stonework,
          addBarkShelter: layout?.barkShelters, addLeafPatch: layout?.leafPatches,
        };
        const added = addedItems[edit.kind]?.find((item) => !terrainObjects.some((existing) => existing.id === item.id));
        if (added) setTerrainSelection({ authority: result, id: added.id });
        setPlacement(null);
        const choice: ForestTerrainChoice = edit.kind === 'addLeafPatch' ? { kind: 'leafPatch', preset: edit.preset }
          : edit.kind === 'addBarkShelter' ? { kind: 'barkShelter', preset: edit.preset }
          : edit.kind === 'addStonework' ? { kind: 'stonework', preset: edit.preset }
          : { kind: edit.kind === 'addLog' ? 'log' : 'rock' };
        setNotice({ text: `已放置${forestTerrainName(choice)}` });
        return;
      }
      if (edit.kind === 'remove') setTerrainSelection(null);
      const edited = terrainObjects.find((object) => object.id === edit.id);
      const name = forestTerrainName(edited ?? { kind: 'rock' });
      const messages = { move: `${name}位置已更新`, mirror: `已翻转${name}`, remove: `已移除${name}` };
      setNotice({ text: messages[edit.kind] });
    });
  }

  function replay(action: 'undo' | 'redo') {
    if (forestRocks) {
      if (action === 'undo' ? forestRocks.undoAvailable : forestRocks.redoAvailable) editForestRocks({ action });
      return;
    }
    if (observedWorld?.pond) {
      const redo = action === 'redo';
      if (!(redo ? observedWorld.pondRedoAvailable : observedWorld.pondUndoAvailable)) return;
      editPondCommand('edit_pond_history', { redo }, (result) => {
        if (staticSection && !result.pond?.basins.some((entry) => entry.id === basin?.id
          && entry.staticSections?.some(({ row }) => row === staticSection.row))) {
          setSectionSelection(null);
          setTerrainSelection(null);
          setPlacement(null);
        }
        setNotice({ text: redo ? '已重做池塘修改' : '已撤销池塘修改' });
      });
      return;
    }
    const destination = editableHistory?.[action].at(-1);
    if (destination) moveRock(destination, action);
  }

  async function editPondCommand(
    command: 'edit_pond' | 'edit_pond_history',
    payload: Record<string, unknown>,
    applied: (result: WorldSnapshot) => void,
  ) {
    const target = observedWorld;
    if (!target?.pond || !canPondEdit) return false;
    let accepted = false;
    const completed = await execute<WorldSnapshot>(command, {
      worldId: target.worldId,
      commandId: crypto.randomUUID(),
      expectedGeneration: target.runGeneration,
      expectedRevision: target.editRevision,
      ...payload,
    }, (result) => {
      accepted = acceptEditReceipt(target, result);
      if (accepted) applied(result);
    });
    return completed && accepted;
  }

  function editPond(edit: PondEdit) {
    let commandEdit = edit;
    if (staticSection) {
      if (edit.basinId !== basin?.id || !isPondStaticEdit(edit)) return Promise.resolve(false);
      commandEdit = { type: 'editStaticSection', basinId: edit.basinId, row: staticSection.row, edit };
    }
    return editPondCommand('edit_pond', { edit: commandEdit }, (result) => {
      const resultBasin = result.pond?.basins.find(({ id }) => id === edit.basinId);
      const staticResult = staticSection ? resultBasin?.staticSections?.find(({ row }) => row === staticSection.row) : resultBasin;
      if (edit.type === 'paintShape') {
        setNotice({ text: `已${edit.mode === 'add' ? '扩展' : '收回'} ${edit.cells.length} 格水域，水量保持不变。` });
      } else if (edit.type === 'commitTopology') {
        setTerrainSelection(null);
        setNotice({ text: `已${edit.mode === 'add' ? '合并' : '拆分'}水域，可撤销本次修改。` });
      } else if (edit.type === 'addLeafDeposit' || edit.type === 'moveLeafDeposit') {
        const id = edit.type === 'moveLeafDeposit' ? edit.id : staticResult?.leafDeposits?.find((leaf) =>
          !pondLeafDeposits.some((prior) => prior.id === leaf.id))?.id;
        if (id !== undefined) setTerrainSelection({ authority: result, id, basinId: edit.basinId, sectionRow: staticSection?.row, pondKind: 'leafDeposit' });
        setPlacement(null);
        setInspector('object');
        setTask('inspector');
        setNotice({ text: edit.type === 'addLeafDeposit' ? `已投放 ${edit.mgC} mg C 沉叶，计入整池碎屑。` : '已更新沉叶显示位置，投放剂量和库存保持。' });
      } else if (edit.type === 'removeLeafDeposit') {
        setTerrainSelection(null);
        setPlacement(null);
        setNotice({ text: '已取出该处沉叶的剩余份额，按外部输出记账。' });
      } else if (edit.type === 'stockShrimp' || edit.type === 'stockDaphnia') {
        setPlacement(null);
        setInspector('biology');
        setTask('inspector');
        setNotice({ text: edit.type === 'stockShrimp' ? `已投放 ${edit.count} 只成体米虾。` : `已向当前水域投放 ${edit.count} 只水蚤。` });
      } else if (edit.type === 'removeShrimp' || edit.type === 'removeDaphnia') {
        if (edit.type === 'removeShrimp') setAnimalSelection(null);
        const animal = edit.type === 'removeShrimp' ? '米虾' : `${edit.count} 只水蚤`;
        setNotice({ text: `已取出${animal}，碳氮磷按外部输出记账。` });
      } else if (edit.type === 'enableStoneRest') {
        setNotice({ text: '当前剖面已启用主石停靠与摄食。' });
      } else if (edit.type === 'enableStoneBiofilm') {
        setNotice({ text: '当前剖面已启用石面生物膜。' });
      } else if (edit.type === 'enableWoodBiofilm') {
        setNotice({ text: '当前剖面已启用沉木生物膜库存。' });
      } else if (edit.type === 'addWaterPlant' || edit.type === 'placeRibbonPlant') {
        const patches = staticResult?.waterPlantPatches;
        const added = patches?.find(({ id }) => !pondWaterPlants?.some((patch) => patch.id === id));
        if (added) setTerrainSelection({ authority: result, id: added.id, basinId: edit.basinId, sectionRow: staticSection?.row, pondKind: 'waterPlant' });
        setPlacement(null);
        if (edit.type === 'placeRibbonPlant') {
          setInspector('object');
          setTask('inspector');
        }
        setNotice({ text: edit.type === 'placeRibbonPlant' ? '已种下带状叶水草。' : '已种下轮生细叶水草。' });
      } else if (edit.type === 'moveWaterPlant' || edit.type === 'trimWaterPlant' || edit.type === 'removeWaterPlant') {
        if (edit.type === 'removeWaterPlant') setTerrainSelection(null);
        const messages = { moveWaterPlant: '水草位置已更新。', trimWaterPlant: '修剪量已转入池底碎屑。', removeWaterPlant: '水草剩余库存已转入池底碎屑。' };
        setNotice({ text: messages[edit.type] });
      } else if (edit.type === 'addFloatingPlant' || edit.type === 'moveFloatingPlant' || edit.type === 'removeFloatingPlant') {
        const before = basin?.floatingPlants;
        const after = result.pond?.basins.find(({ id }) => id === edit.basinId)?.floatingPlants;
        const id = edit.type === 'addFloatingPlant' ? after?.find((plant) => !before?.some(({ id }) => id === plant.id))?.id
          : edit.type === 'moveFloatingPlant' ? edit.id : undefined;
        setTerrainSelection(id === undefined ? null : { authority: result, id, basinId: edit.basinId, pondKind: 'floatingPlant' });
        const messages = { addFloatingPlant: '已放置活体浮叶。', moveFloatingPlant: '浮叶位置和遮光已更新。', removeFloatingPlant: '浮叶库存已转入池底碎屑。' };
        setNotice({ text: messages[edit.type] });
      } else if (edit.type === 'addSedge' || edit.type === 'moveSedge' || edit.type === 'removeSedge') {
        const id = edit.type === 'addSedge' ? resultBasin?.shoreSedges?.find(({ anchor }) => anchor.shoreCell === edit.anchor.shoreCell)?.id
          : edit.type === 'moveSedge' ? edit.id : undefined;
        setTerrainSelection(id === undefined ? null : { authority: result, id, basinId: edit.basinId, pondKind: 'sedge' });
        const messages = { addSedge: '已沿岸种植莎草。', moveSedge: '已移植莎草，保留原库存。', removeSedge: '已移出莎草，按外部输出记账。' };
        setNotice({ text: messages[edit.type] });
      } else if (edit.type === 'addMoss') {
        const after = staticResult?.mossPatches;
        const added = after?.find(({ id }) => !pondMoss?.some((patch) => patch.id === id));
        if (added) setTerrainSelection({ authority: result, id: added.id, basinId: edit.basinId, sectionRow: staticSection?.row, pondKind: 'plant' });
        setPlacement(null);
        setDrawerOpen(false);
        setTask('scene');
        setNotice({ text: `已附着${pondMossName(edit.preset)}` });
      } else if (edit.type === 'moveMoss' || edit.type === 'trimMoss' || edit.type === 'removeMoss') {
        if (edit.type === 'removeMoss') setTerrainSelection(null);
        const messages = { moveMoss: '斑块位置已更新', trimMoss: '已修剪斑块，修剪量转入水体碎屑', removeMoss: '已移除斑块，剩余库存转入水体碎屑' };
        setNotice({ text: messages[edit.type] });
      } else if (edit.type === 'addScenery') {
        const after = staticResult?.scenery;
        const added = after?.find(({ id }) => !pondScenery?.some((item) => item.id === id));
        if (added) setTerrainSelection({ authority: result, id: added.id, basinId: edit.basinId, sectionRow: staticSection?.row, pondKind: 'scenery' });
        setPlacement(null);
        setNotice({ text: `已放置${pondSceneryName(edit.style)}` });
      } else if (edit.type === 'moveScenery' || edit.type === 'removeScenery') {
        const item = pondScenery?.find(({ id }) => id === edit.id);
        const name = item ? pondSceneryName(item.style) : '场景摆件';
        if (edit.type === 'removeScenery') setTerrainSelection(null);
        setNotice({ text: edit.type === 'removeScenery' ? `已移除${name}` : `${name}位置已更新` });
      } else if (edit.type === 'addStone' || edit.type === 'addWood') {
        const choice: PondBedChoice = edit.type === 'addWood' ? { kind: 'wood', preset: edit.preset } : { kind: 'stone', preset: edit.preset };
        const before = basin?.id === edit.basinId ? basin : basins?.find(({ id }) => id === edit.basinId);
        const collection = choice.kind === 'wood' ? 'woods' : 'stones';
        const added = staticResult?.[collection]?.find((object) => !before?.[collection]?.some(({ id }) => id === object.id));
        if (added) setTerrainSelection({ authority: result, id: added.id, basinId: edit.basinId, sectionRow: staticSection?.row, pondKind: choice.kind });
        setPlacement(null);
        setNotice({ text: `已放置${pondBedName(choice)}` });
      } else if ('id' in edit) {
        const kind = edit.type === 'moveWood' || edit.type === 'removeWood' ? 'wood' : 'stone';
        const object = pondBedObjects.find(({ id, kind: objectKind }) => id === edit.id && kind === objectKind);
        const name = object ? pondBedName(object) : kind === 'wood' ? '沉木' : '池底石';
        const removed = edit.type === 'removeWood' || edit.type === 'removeStone';
        if (removed) setTerrainSelection(null);
        setNotice({ text: removed ? `已移除${name}` : `${name}池底位置已更新` });
      } else {
        setNotice({ text: '池塘环境已更新' });
      }
    });
  }

  function addStarterLife() {
    if (!world?.starterCommunityAvailable || !canMove || staticSection || (basin && !basin.wet)) return;
    void dispatch('add_starter_life', {
      commandId: crypto.randomUUID(),
      expectedGeneration: world.runGeneration,
      expectedRevision: world.editRevision,
    }, () => {
      setHistory(null);
      setNotice({ text: '已放入初始生物' });
    });
  }

  function saveWorld() {
    if (trial) return;
    void dispatch('save_world', undefined, (result) => {
      setSaveFailures((previous) => ({ ...previous, [result.worldId]: '' }));
    });
  }

  function restoreWorld() {
    if (trial) return;
    void dispatch('restore_world', undefined, (result) => {
      setConfirmRestore(false);
      setHistory(null);
      setSelected(false);
      setPlacement(null);
      setSaveFailures((previous) => ({ ...previous, [result.worldId]: '' }));
      setNotice({ text: result.lastSaveMessage || '已读取保存的世界' });
    });
  }

  function createWorld() {
    if (trial || !confirmCreate) return;
    const preset = confirmCreate;
    void dispatch(preset === 'pond' ? 'create_pond_from_preset' : 'create_forest_from_preset', undefined, () => {
      resetWorldTools();
      setNotice({ text: `已从${presetLabels[preset]}预设创建并打开一个新世界。` });
    });
  }

  function startTrial() {
    if (!world || !editing) return;
    void dispatch('start_forest_trial', undefined, (result) => {
      setActiveTrial({ source: world, snapshot: result });
      setTrialAction(null);
      setTrialRate('60x');
      setConfirmCreate(null);
      setConfirmRestore(false);
      setLocation(null);
      setSelected(false);
      setLight('scene');
      setPlacement(null);
      setTask('scene');
    });
  }

  function advanceTrial(scaled = false) {
    if (!trial) return;
    const command = scaled ? 'advance_trial_scaled' : 'advance_forest_trial';
    const options = scaled ? { elapsedTicks: 20, rate: trialRate } : { ticks: 600 };
    void dispatch(command, options, (result) => {
      setActiveTrial({ ...trial, snapshot: result });
    });
  }

  function discardTrial() {
    if (!trial) return;
    void dispatch('discard_forest_trial', undefined, () => {
      setActiveTrial(null);
      setTrialAction(null);
      setPlacement(null);
      setNotice({ text: '已丢弃试运行，原世界保持不变。' });
    });
  }

  function adoptTrial() {
    if (!trial) return;
    void dispatch('adopt_trial', undefined, () => {
      resetWorldTools();
      setActiveTrial(null);
      setTrialAction(null);
      setNotice({ text: '已采用并保存试运行结果，保留正式世界原有的运行设置。' });
    });
  }

  function saveTrial(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = trialName.trim();
    if (!trial || !name) return;
    void dispatch('save_trial_as_world', { name }, (result) => {
      resetWorldTools();
      setActiveTrial(null);
      setTrialAction(null);
      setNotice({ text: `已另存并打开「${worldName(result)}」，新世界已暂停。` });
    });
  }

  function saveLocation(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!location || trial) return;
    const name = location.name.trim();
    const latitudeDegrees = Number(location.latitude);
    const longitudeDegrees = Number(location.longitude);
    if (!name || !location.latitude.trim() || !location.longitude.trim()
      || !Number.isFinite(latitudeDegrees) || Math.abs(latitudeDegrees) > 90
      || !Number.isFinite(longitudeDegrees) || Math.abs(longitudeDegrees) > 180) {
      setNotice({ text: '请填写地点名称，以及合法的纬度和经度。', error: true });
      return;
    }
    void dispatch('set_world_location', { location: { name, latitudeDegrees, longitudeDegrees } }, (result) => {
      setLocation(null);
      setNotice({ text: '已设为' + (result.location?.name ?? name) + '。世界在运行时会逐渐对齐该地点的昼夜。' });
    });
  }

  function handleKeys(event: KeyboardEvent<HTMLElement>) {
    if (libraryOpen) return;
    if (event.key === 'Escape') {
      if (placementKind) return;
      setTrialAction(null);
      setConfirmRestore(false);
      setConfirmCreate(null);
      setSelected(false);
      setTerrainSelection(null);
      return;
    }
    if (event.target instanceof HTMLElement && event.target.matches('input, textarea, select, [contenteditable]')) return;
    if (!event.ctrlKey && !event.metaKey) return;
    switch (event.key.toLowerCase()) {
      case 's':
        event.preventDefault();
        saveWorld();
        break;
      case 'z':
        if (!editing && !canTerrainEdit && !canPondEdit) return;
        event.preventDefault();
        replay(event.shiftKey ? 'redo' : 'undo');
        break;
      case 'y':
        if (!editing && !canTerrainEdit && !canPondEdit) return;
        event.preventDefault();
        replay('redo');
        break;
    }
  }

  const feedback = connectionError || notice?.text;
  const saveFailure = world && saveFailures[world.worldId];
  const saveFeedback = saveFailure || (!trial && world?.lastSaveMessage);
  const trialElapsed = trial ? Math.max(0, trial.snapshot.simulatedSeconds - trial.source.simulatedSeconds) : 0;

  return (
    <main className="world-editor" data-world-id={world?.worldId} data-trial={trial ? 'true' : undefined}
      data-r5-forest={Boolean(forestRocks)} data-r5-pond={Boolean(pondStones)} data-r5-scenery={Boolean(pondScenery) || undefined}
      data-r5-forest-scenery={Boolean(forestScenery) || undefined}
      data-appearance={appearance}
      data-task={task} onKeyDown={handleKeys}>
      <header className="editor-heading" data-tauri-drag-region>
        <div className="editor-heading__titles" data-tauri-drag-region>
          <span data-tauri-drag-region>{trial ? '试运行' : '生态环境编辑器'}</span>
          <h1 data-tauri-drag-region>{world ? worldName(world) : '生态世界'}</h1>
        </div>
        <div className="editor-heading__actions">
          {!trial && <>
            {native && <button type="button" className="editor-button" disabled={!canAct} aria-haspopup="dialog"
              onClick={() => { setNotice(null); setConfirmCreate(null); setLibraryOpen(true); }}>世界库</button>}
            <button type="button" className="editor-button" onClick={saveWorld} disabled={!canAct}>
              {pending === 'save_world' ? '正在保存' : '保存世界'}
            </button>
            <button
              type="button"
              className="editor-button editor-button--primary"
              onClick={editing ? finishEditing : editWorld}
              disabled={!canAct}
            >
              {editing ? world?.manualPaused ? '完成编辑' : '完成编辑，开始生活' : '编辑环境'}
            </button>
          </>}
          <button
            type="button"
            className="editor-button editor-button--close"
            onClick={() => void closeEditor()}
            disabled={pending !== null}
            title="关闭窗口，保留当前编辑和运行状态"
          >
            关闭
          </button>
        </div>
      </header>

      <div className="editor-tools" aria-label="世界操作">
        <div className="editor-tools__row">
          <div className="editor-tools__actions">
            {trial ? (
              <>
                <button type="button" className="editor-tool" disabled={!canAct || Boolean(trialAction)} onClick={() => advanceTrial()}>
                  {pending === 'advance_forest_trial' ? '正在推进' : '推进 60 模拟秒'}
                </button>
                <label className="editor-trial-rate">倍率
                  <select aria-label="试运行倍率" value={trialRate} disabled={!canAct || Boolean(trialAction)} onChange={(event) => setTrialRate(event.target.value as TrialRate)}>
                    <option value="1x">1×</option><option value="10x">10×</option><option value="60x">60×</option>
                  </select>
                </label>
                <button type="button" className="editor-tool" disabled={!canAct || Boolean(trialAction)} onClick={() => advanceTrial(true)}>
                  {pending === 'advance_trial_scaled' ? '正在推进' : `推进 ${trialBatchSeconds[trialRate]} 模拟秒`}
                </button>
                <span className="editor-tools__split" aria-hidden="true" />
                <button type="button" className="editor-tool" disabled={!canAct} aria-expanded={trialAction === 'adopt'}
                  onClick={() => setTrialAction(trialAction === 'adopt' ? null : 'adopt')}>采用结果</button>
                <button type="button" className="editor-tool" disabled={!canAct} aria-expanded={trialAction === 'save'} onClick={() => {
                  setTrialName(`${worldName(trial.source).slice(0, 72)} 试运行`);
                  setTrialAction(trialAction === 'save' ? null : 'save');
                }}>另存为世界</button>
                <button type="button" className="editor-tool" disabled={!canAct || Boolean(trialAction)} onClick={discardTrial}>丢弃副本</button>
                {forestRocks && <>
                  <button type="button" className="editor-tool" disabled={!canTerrainEdit || !forestRocks.undoAvailable} onClick={() => replay('undo')}>撤销</button>
                  <button type="button" className="editor-tool" disabled={!canTerrainEdit || !forestRocks.redoAvailable} onClick={() => replay('redo')}>重做</button>
                </>}
                {pondStones && <>
                  <button type="button" className="editor-tool" disabled={!canPondEdit || !observedWorld?.pondUndoAvailable} onClick={() => replay('undo')}>撤销</button>
                  <button type="button" className="editor-tool" disabled={!canPondEdit || !observedWorld?.pondRedoAvailable} onClick={() => replay('redo')}>重做</button>
                </>}
                <span className="editor-trial-time">+{trialElapsed.toFixed(0)} 秒</span>
              </>
            ) : (
              <>
                {editing && (
                  <>
                    {!basin && !forestRocks && <><button type="button" className="editor-tool" aria-pressed={selected} disabled={!canMove} onClick={() => {
                      setSelected(true);
                      imageHost.current?.querySelector('canvas')?.focus();
                    }}>选择石块</button>
                    <span className="editor-tools__split" aria-hidden="true" /></>}
                    <button type="button" className="editor-tool" disabled={!canMove || !(forestRocks?.undoAvailable ?? (basin ? world?.pondUndoAvailable : editableHistory?.undo.length))} onClick={() => replay('undo')} title="Ctrl+Z">撤销</button>
                    <button type="button" className="editor-tool" disabled={!canMove || !(forestRocks?.redoAvailable ?? (basin ? world?.pondRedoAvailable : editableHistory?.redo.length))} onClick={() => replay('redo')} title="Ctrl+Y">重做</button>
                    {world?.starterCommunityAvailable && !staticSection && <button type="button" className="editor-tool" disabled={!canMove || Boolean(basin && !basin.wet)}
                      title={basin && !basin.wet ? '先补水，再放入初始生物' : '放入当前主题的一组初始生物'} onClick={addStarterLife}>放入初始生物</button>}
                    {(world?.forest || basin) && <button type="button" className="editor-tool" disabled={!canAct} onClick={startTrial}>试运行</button>}
                  </>
                )}
                {!editing && native && world && <button type="button" className="editor-tool" disabled={!canAct} onClick={toggleManualPause}>
                  {world.paused ? '继续运行' : '暂停世界'}
                </button>}
                {native && world && (['forest', 'pond'] as const).map((preset) => (
                  <button key={preset} type="button" className="editor-tool" aria-expanded={confirmCreate === preset} disabled={!canAct}
                    onClick={() => setConfirmCreate(confirmCreate === preset ? null : preset)}>从{presetLabels[preset]}预设新建</button>
                ))}
              </>
            )}
          </div>
          <WorldState snapshot={world} editing={editing} trial={Boolean(trial)} />
        </div>
        {confirmCreate && !trial && (
          <div className="editor-inline-actions">
            <p>先保存当前世界，再创建并打开{presetLabels[confirmCreate]}预设。</p>
            <button type="button" className="editor-tool" disabled={!canAct} onClick={createWorld}>新建并打开</button>
            <button type="button" className="editor-tool" disabled={!canAct} onClick={() => setConfirmCreate(null)}>取消</button>
          </div>
        )}
        {trial && trialAction === 'adopt' && <div className="editor-inline-actions">
          <p>将试运行的生态状态与模拟进度写入「{worldName(trial.source)}」，保留正式世界的暂停设置。</p>
          <button className="editor-button editor-button--primary" type="button" disabled={!canAct} onClick={adoptTrial}>采用并保存</button>
          <button className="editor-button" type="button" disabled={pending !== null} onClick={() => setTrialAction(null)}>取消</button>
        </div>}
        {trial && trialAction === 'save' && <form className="editor-inline-actions editor-trial-save" onSubmit={saveTrial}>
          <label>新世界名称<input autoFocus required maxLength={80} value={trialName} onChange={(event) => setTrialName(event.target.value)} /></label>
          <button className="editor-button editor-button--primary" type="submit" disabled={!canAct || !trialName.trim()}>另存并打开</button>
          <button className="editor-button" type="button" disabled={pending !== null} onClick={() => setTrialAction(null)}>取消</button>
        </form>}
      </div>

      <div className="editor-body">
        <nav className="editor-inspector-rail" aria-label="检查器">
          <button type="button" className="editor-tool editor-return-scene" aria-pressed={task === 'scene'}
            onClick={() => setTask('scene')}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" aria-hidden="true">
              <rect x="3.5" y="5" width="17" height="14" rx="2.5" />
              <path d="m7.5 15 3.5-4 2.8 3 2.2-2.2 2.5 3.2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            场景
          </button>
          {inspectors.map(({ id, label, icon }) => (
            <button key={id} type="button" className="editor-tool" aria-pressed={inspector === id} aria-controls="editor-inspector"
              onClick={() => { if (pondMapOpen) setPlacement(null); setInspector(id); setTask('inspector'); }}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" aria-hidden="true">
                <path d={icon} strokeLinecap="round" strokeLinejoin="round" />
              </svg>
              {label}
            </button>
          ))}
          {(forestRocks || pondStones) && (
            <button type="button" className="editor-tool editor-open-drawer" aria-expanded={drawerOpen}
              onClick={() => {
                if (pondMapOpen) setPlacement(null);
                const compact = window.matchMedia('(max-width: 780px), (max-height: 515px)').matches;
                const open = compact ? task !== 'content' : !drawerOpen;
                setDrawerOpen(open);
                setTask(open ? 'content' : 'scene');
                if (!open && (mossPlacement || forestAttachment)) setPlacement(null);
              }}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" aria-hidden="true">
                <path d="M4 7.5 12 3.5l8 4-8 4-8-4Z M4 7.5v9l8 4 8-4v-9 M12 11.5v9" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
              内容
            </button>
          )}
        </nav>
        <section className="editor-scene" aria-label="生态场景" data-pond-static-section-row={staticSection?.row}>
          <div className="editor-scene__position">
            <div className="editor-scene__mount">
              <div className={`editor-scene__image${pondMapOpen ? ' editor-scene__image--shape' : ''}`} ref={imageHost}>
                {pondShapePlacement && pondShape && basin && basins && observedWorld ? <PondShapeEditor
                    key={`${observedWorld.worldId}/${observedWorld.runGeneration}/${basin.id}`}
                    basin={basin} shape={pondShape} basins={basins} revision={observedWorld.editRevision} disabled={!canPondEdit}
                    onPaint={(mode, cells) => { void editPond({ type: 'paintShape', basinId: basin.id, mode, cells }); }}
                    onApplyTopology={(mode, cells) => editPond({ type: 'commitTopology', basinId: basin.id, mode, cells })}
                    onClose={closePondMap}
                  /> : pondFloatingPlacement && pondShape && basin && basins && observedWorld ? <PondFloatingEditor
                    key={`${observedWorld.worldId}/${observedWorld.runGeneration}/${basin.id}`}
                    basin={basin} shape={pondShape} basins={basins} disabled={!canPondEdit}
                    selectedId={selectedFloatingPlant?.id} onSelect={selectFloatingPlant} onEdit={editPond} onClose={closePondMap}
                  /> : pondSedgePlacement && pondShape && basin && basins && observedWorld ? <PondSedgeEditor
                    key={`${observedWorld.worldId}/${observedWorld.runGeneration}/${basin.id}`}
                    basin={basin} shape={pondShape} basins={basins} disabled={!canPondEdit || pending !== null}
                    selectedId={selectedSedge?.id} onSelect={selectSedge} onEdit={editPond} onClose={closePondMap}
                  /> : shownWorld ? <WorldScene
                    key={staticSection ? `${basin?.id}/${staticSection.row}` : undefined}
                    snapshot={shownWorld}
                    basinId={basin?.id}
                    active={viewActive}
                    pondFloatingSelectionId={selectedFloatingPlant?.id}
                    pondSedgeSelectionId={selectedSedge?.id}
                    rockSelected={selected}
                    onRockSelect={() => setSelected(true)}
                    onRockMove={canMove ? (x, y) => moveRock({ x, y }) : undefined}
                    forestView={hasObservationControls ? { zoom, focus, showScale, springtails: forestScenery ? springtailView : undefined,
                      animalId: inspector === 'biology' && !activePlacement ? selectedAnimalId : undefined } : undefined}
                    forestMossEdit={canTerrainEdit && forestMossPatches ? {
                      selectedId: selectedForestMoss?.id, onSelect: (id) => selectForestPlant('moss', id),
                      placement: forestPlantPlacement?.kind === 'moss' ? {
                        moving: movingForestPlant, onPlace: finishForestPlant, onCancel: () => setPlacement(null),
                      } : undefined,
                    } : undefined}
                    forestFernEdit={canTerrainEdit && forestFernPatches ? {
                      selectedId: selectedForestFern?.id, onSelect: (id) => selectForestPlant('fern', id),
                      placement: forestPlantPlacement?.kind === 'fern' ? {
                        moving: movingForestPlant, onPlace: finishForestPlant, onCancel: () => setPlacement(null),
                      } : undefined,
                    } : undefined}
                    stocking={stocking && (stocking.species === 'woodlice' ? canTerrainEdit : canPondEdit) ? {
                      onPlace: finishStocking, onCancel: () => { setPlacement(null); setInspector('biology'); setTask('inspector'); },
                    } : undefined}
                    forestBrush={canTerrainEdit && substrateBrush && substrateCells ? {
                      kind: 'substrate', preset: substrateBrush, soil: substrateCells, onFinish: finishSubstrateBrush, onCancel: () => setPlacement(null),
                    } : canTerrainEdit && leafBrush ? {
                      kind: 'leaves', preset: leafBrush, capacity: leafBrushCapacity, onFinish: finishLeafBrush, onCancel: () => setPlacement(null),
                    } : undefined}
                    forestEdit={canTerrainEdit && !forestPlacement && !leafBrush && !substrateBrush && !stocking && !forestPlantPlacement ? {
                      mode: placementChoice ? 'add' : 'move',
                      addKind: placementChoice?.kind,
                      stoneworkPreset: placementChoice?.kind === 'stonework' ? placementChoice.preset : undefined,
                      barkPreset: placementChoice?.kind === 'barkShelter' ? placementChoice.preset : undefined,
                      leafPreset: placementChoice?.kind === 'leafPatch' ? placementChoice.preset : undefined,
                      selectedId: selectedTerrain?.id,
                      onSelect: selectTerrainObject,
                      onAdd: confirmTerrainPlacement,
                      onMove: (id, position) => editForestRocks({ action: 'change', edit: { kind: 'move', id, position } }),
                      onCancel: () => setPlacement(null),
                    } : undefined}
                    forestSceneryEdit={canTerrainEdit && forestScenery && !placementChoice && !forestAttachment && !leafBrush && !substrateBrush && !stocking && !forestPlantPlacement ? {
                      selectedId: selectedForestScenery?.id,
                      placement: forestPlacement?.style,
                      onSelect: selectForestScenery,
                      onMove: (id: number, x: number, y: number) => editForestRocks({ action: 'change', edit: { kind: 'moveScenery', id, x, y } }),
                      onPlace: confirmForestScenery,
                      onCancel: () => setPlacement(null),
                    } : undefined}
                    pondRockEdit={canPondEdit && pondStones && basin && !sceneryPlacement && !stocking && !waterPlantPlacement && !leafPlacement ? {
                      mode: pondPlacement ? 'add' : 'move',
                      addKind: pondPlacement?.kind,
                      addPreset: pondPlacement?.kind === 'stone' ? pondPlacement.preset : undefined,
                      woodPreset: pondPlacement?.kind === 'wood' ? pondPlacement.preset : undefined,
                      selectedId: selectedMoss?.id ?? selectedPondObject?.id,
                      selectedKind: selectedMoss ? 'plant' : selectedPondObject?.kind,
                      mossDraft,
                      onSelect: selectPondObject,
                      onAdd: confirmPondPlacement,
                      onMove: (id: number, x: number, kind: PondBedKind) => {
                        if (!mossPlacement && mossMoveOffset === null) editPond({ type: kind === 'wood' ? 'moveWood' : 'moveStone', basinId: basin.id, id, x });
                      },
                      onCancel: () => {
                        setPlacement(null);
                        setMossMoveOffset(null);
                      },
                    } : undefined}
                    pondSceneryEdit={canPondEdit && pondScenery && basin && !pondPlacement && !mossPlacement && mossMoveOffset === null && !stocking && !waterPlantPlacement && !leafPlacement ? {
                      selectedId: selectedScenery?.id,
                      placement: sceneryPlacement?.style,
                      onSelect: (id: number) => selectPondObject(id, 'scenery'),
                      onMove: (id: number, x: number, y: number) => editPond({ type: 'moveScenery', basinId: basin.id, id, x, y }),
                      onPlace: confirmPondScenery,
                    } : undefined}
                    pondWaterPlantEdit={canPondEdit && pondWaterPlants && basin && !pondPlacement && !mossPlacement && mossMoveOffset === null && !stocking && !sceneryPlacement && !leafPlacement ? {
                      selectedId: selectedWaterPlant?.id,
                      placing: Boolean(waterPlantPlacement),
                      ribbonPreset: waterPlantPlacement?.ribbonPreset,
                      onSelect: (id) => selectPondObject(id, 'waterPlant'),
                      onAdd: confirmWaterPlant,
                      onMove: (id, x) => editPond({ type: 'moveWaterPlant', basinId: basin.id, id, x }),
                      onCancel: () => setPlacement(null),
                    } : undefined}
                    pondLeafEdit={canPondEdit && pondScenery && !pondPlacement && !mossPlacement && mossMoveOffset === null
                      && !stocking && !sceneryPlacement && !waterPlantPlacement ? {
                        selectedId: selectedLeaf?.id, onSelect: (id) => selectPondObject(id, 'leafDeposit'),
                        placement: leafPlacement ? {
                          id: movingLeaf?.id ?? null, preset: leafPlacement.preset, x: movingLeaf?.x ?? .5,
                          onPlace: ({ x }) => confirmLeafDeposit(x), onCancel: () => setPlacement(null),
                        } : undefined,
                      } : undefined}
                    onReady={setRenderer}
                  /> : <p className="editor-scene__waiting">正在打开世界</p>}
                {!pondMapOpen && (trial ? <span className="editor-scene__light">试运行副本</span>
                  : lightPreview !== 'scene' && <span className="editor-scene__light">光照预览</span>)}
              </div>
            </div>
            {!pondMapOpen && basinAuthority?.shape && Boolean(basinAuthority.staticSections?.length) && <div className="editor-pond-section">
              <label className="editor-pond-field">观景剖面
                <select data-pond-static-section-select value={staticSection?.row ?? ''} disabled={pending !== null}
                  onChange={(event) => selectPondSection(event.target.value)}>
                  <option value="">主剖面 · 第 {basinAuthority.shape.protectedRow + 1} 行</option>
                  {basinAuthority.staticSections?.map(({ row }) => <option key={row} value={row}>副剖面 · 第 {row + 1} 行</option>)}
                </select>
              </label>
              {staticSection && <p className="editor-scene__hint">本剖面的生物和布置独立编辑，水质由整片水域统一计算。</p>}
            </div>}
            {hasObservationControls && !pondMapOpen && <div className="editor-scene__controls" ref={sceneControls}>
              {placementKind ? <div className="editor-placement-status" role="status"
                data-placement-kind={placementKind} data-placement-preset={placementPreset}>
                <span>{terrainPlacementHints[placementKind]}按 Esc 可取消。</span>
                {stocking && <span className="editor-leaf-placement-dose">待投放 {stocking.count} 只{stockSpeciesName(stocking.species)}</span>}
                {newLeaf && <span className="editor-leaf-placement-dose">投放 {newLeaf.mgC} mg C</span>}
                {forestPlantPlacement && !movingForestPlant && <span className="editor-leaf-placement-dose">
                  {forestPlantPlacement.kind === 'fern' && forestFernPreset ? `投放 ${matterSummary(forestFernPreset.initialMgCnp)} · 株高约 ${forestFernPreset.heightMm} mm`
                    : forestMossPreset ? `投放 ${matterSummary(forestMossPreset.initialMgCnp)}` : null}
                </span>}
                {placementLeafDose && <span className="editor-leaf-placement-dose">
                  {leafBrush ? `每格 ${matterSummary(placementLeafDose)} · 剩余 ${leafBrushCapacity} 格` : `投放 ${matterSummary(placementLeafDose)}`}
                </span>}
                {fragmentDose !== undefined && <span className="editor-leaf-placement-dose" data-forest-fragment-total={fragmentDose}>
                  每笔共 {displayMass(fragmentDose)} mg C · 平分至有效土格 · 剩余 {leafBrushCapacity} 格
                </span>}
                {substrateBrush && <span className="editor-leaf-placement-dose">
                  {forestSubstrateName(substrateBrush.substrate)} · 每格容量 {substrateBrush.waterCapacityMl} mL · 蒸发 ×{substrateBrush.evaporationMultiplier}
                </span>}
                {sceneryPlacement && pondSceneryIsBed(sceneryPlacement.style) && <span className="editor-leaf-placement-dose">
                  在底床选点，松开鼠标或按回车放置。红框表示超出范围。
                </span>}
                {mossPlacement || forestAttachment
                  ? <button type="button" className="editor-tool editor-moss-resume" onClick={() => { setDrawerOpen(true); setTask('content'); }}>继续附着</button>
                  : <button type="button" className="editor-tool" onClick={() => {
                    setPlacement(null);
                    if (stocking) { setInspector('biology'); setTask('inspector'); }
                  }}>{stocking ? '取消投放' : substrateBrush ? '结束笔刷' : '取消放置'}</button>}
              </div> : <R5ObservationControls zoom={zoom} focus={springtailSideFocus ? undefined : focus} showScale={showScale}
                onZoomChange={setZoom} onFocusChange={(next) => {
                  setFocus(next);
                  if (springtailView && springtailView !== 'both') setSpringtailView('both');
                }} onScaleChange={setShowScale} />}
            </div>}
            {canTerrainEdit && !placementKind && <p className="editor-scene__hint">
              {forestScenery ? '拖动地形或造景。附着的装饰会随承载物移动。'
                : `点选或拖动${hasStonework || hasBarkShelters || hasLeafPatches ? '地面物件' : hasEditableLogs ? '石木' : '石块'}。方向键预览，按 Enter 确认。`}
            </p>}
            {canPondEdit && pondStones && !placementKind && <p className="editor-scene__hint">
              {pondScenery ? '拖动摆件调整构图，石材与沉木沿池底移动。' : `沿池底拖动${pondWoods ? '石材或沉木' : '石材'}。`}
              方向键预览，按 Enter 确认。
            </p>}
            {editing && !basin && !forestRocks && <p className="editor-scene__hint">拖动石块修改位置。画布获焦后，也可用方向键微调。</p>}
            {editing && basin && !staticSection && !basin.wet && world?.starterCommunityAvailable && <p className="editor-scene__hint">补水后可放入初始生物。</p>}
            {trial && <p className="editor-scene__hint">每次推进只作用于副本，正式世界仍暂停。</p>}
          </div>
        </section>

        {forestRocks && drawerOpen && <ForestTerrainDrawer hasEditableLogs={hasEditableLogs} hasStonework={hasStonework}
          hasBarkShelters={hasBarkShelters} hasLeafPatches={hasLeafPatches} leafDefinitions={forestRocks.leafPresets}
          editable={canTerrainEdit} onAdd={beginTerrainPlacement}
          leafBrush={forestScenery && hasLeafPatches ? { capacity: leafBrushCapacity, onStart: beginLeafBrush } : undefined}
          substrate={substrateCells && substratePresets?.length ? <ForestSubstrateCards presets={substratePresets}
            editable={canTerrainEdit} onStart={beginSubstrateBrush} /> : undefined}
          scenery={forestScenery ? <>
            {(forestMossPreset || forestFernPreset) && <ForestPlantCards plants={[
              ...(forestMossPreset ? [{ kind: 'moss' as const, dose: forestMossPreset.initialMgCnp,
                editable: canTerrainEdit && (forestMossPatches?.length ?? 0) < 48, onPlant: () => beginForestPlant('moss') }] : []),
              ...(forestFernPreset ? [{ kind: 'fern' as const, dose: forestFernPreset.initialMgCnp,
                editable: canTerrainEdit && (forestFernPatches?.length ?? 0) < 48, onPlant: () => beginForestPlant('fern') }] : []),
            ]} />}
            <ForestSceneryCards editable={canTerrainEdit} onAdd={beginForestScenery} />
          </> : undefined}
          attachment={forestAttachment ? <ForestSceneryAttachment style={forestAttachment.style} parent={forestAttachment.parent}
            carriers={forestCarriers} editable={canTerrainEdit} onSelect={chooseForestCarrier}
            onPlace={confirmForestAttachment} onCancel={() => setPlacement(null)} /> : undefined}
          onClose={() => { setDrawerOpen(false); setTask('scene'); if (forestAttachment) setPlacement(null); }} />}
        {pondStones && drawerOpen && (mossPlacement
          ? <PondMossPlacementForm preset={mossPlacement.preset} carriers={mossCarriers} parent={mossPlacement.parent}
              offset={mossPlacement.offset} enabled={canPondEdit && Boolean(basin?.wet)} definitions={basin?.mossPresets}
              chooseParent={(parent) => changeMossPlacement({ parent, offset: 0 })}
              chooseOffset={(offset) => changeMossPlacement({ offset })} confirm={confirmMossPlacement} cancel={() => setPlacement(null)} />
          : <PondBedDrawer hasWoods={Boolean(pondWoods)} hasSlope={Boolean(pondScenery)} editable={canPondEdit} onAdd={beginPondPlacement}
              supplement={pondMoss || pondWaterPlants || pondScenery ? <>
                {pondScenery && <PondLeafDepositCard editable={canPondEdit} count={pondLeafDeposits.length}
                  begin={(preset, mgC) => beginLeafDeposit({ preset, mgC })} />}
                {pondWaterPlants && <PondPlantCards enabled={canPondEdit} wet={Boolean(basin?.wet)}
                  count={pondWaterPlants.length} onAdd={beginWaterPlant} />}
                {!staticSection && pondScenery && pondShape && basin && <PondFloatingCard basin={basin} enabled={canPondEdit} open={beginFloatingPlants} />}
                {!staticSection && pondScenery && pondShape && <PondSedgeCard enabled={canPondEdit} open={beginShoreSedges} />}
                {pondMoss && <PondMossCards enabled={canPondEdit && Boolean(basin?.wet)}
                  definitions={basin?.mossPresets} onAdd={beginMossPlacement} />}
              </> : undefined}
              scenery={pondScenery && basin ? <PondSceneryCards editable={canPondEdit} wet={basin.wet} onAdd={beginPondScenery} /> : undefined}
              onClose={() => { setDrawerOpen(false); setTask('scene'); }} />)}
        <aside id="editor-inspector" className="editor-context" aria-label="场景对象和观察数据">
          {inspector === 'object' && pondLeafDeposits.length > 0 && <PondLeafDepositProperties
            deposits={pondLeafDeposits} selected={selectedLeaf} editable={canPondEdit}
            select={(id) => selectPondObject(id, 'leafDeposit')} move={(id) => beginLeafDeposit({ id })}
            remove={(id) => basin ? editPond({ type: 'removeLeafDeposit', basinId: basin.id, id }) : Promise.resolve(false)} />}
          {trial && inspector === 'environment' && <TrialSummary trial={trial} basinId={basin?.id} />}
          {inspector === 'object' && forestRocks && <ForestTerrainProperties
            key={observedWorld ? `${observedWorld.worldId}:${observedWorld.runGeneration}` : undefined}
            items={terrainObjects} selected={selectedTerrain} hasEditableLogs={hasEditableLogs} hasStonework={hasStonework}
            hasBarkShelters={hasBarkShelters} hasLeafPatches={hasLeafPatches} editable={canTerrainEdit} hasScenery={Boolean(forestScenery)}
            substrateCells={substrateCells}
            carriesScenery={forestScenery?.some(({ parent }) => parent?.kind === 'terrain' && parent.id === selectedTerrain?.id)}
            onSelect={selectTerrainObject}
            onMirror={(id) => editForestRocks({ action: 'change', edit: { kind: 'mirror', id } })}
            onRemove={(id) => editForestRocks({ action: 'change', edit: { kind: 'remove', id } })}
          />}
          {inspector === 'object' && forestScenery && <ForestSceneryProperties items={forestScenery} terrain={terrainObjects}
            selected={selectedForestScenery} editable={canTerrainEdit} onSelect={selectForestScenery}
            onRemove={(id) => editForestRocks({ action: 'change', edit: { kind: 'removeScenery', id } })} />}
          {inspector === 'object' && !basin && !forestRocks && <><section className="editor-context__section">
            <h2>场景对象</h2>
            {observedWorld && (
              <button
                type="button"
                className="editor-object"
                onClick={() => setSelected(!selected)}
                aria-pressed={selected}
              >
                <img src={rockThumbnail} alt="" width="44" height="32" />
                <span><strong>石块</strong><small>遮蔽物</small></span>
              </button>
            )}
          </section>
          {selected && observedWorld && (
            <section className="editor-context__section editor-context__properties">
              <h2>石块位置</h2>
              <dl>
                <div><dt>距左侧</dt><dd>{coordinateFormat.format(observedWorld.rock.x)}</dd></div>
                <div><dt>距上侧</dt><dd>{coordinateFormat.format(observedWorld.rock.y)}</dd></div>
              </dl>
            </section>
          )}
          </>}
          {inspector === 'object' && basin && <section className="editor-context__section">
            <h2>浅塘水域</h2>
            <p className="editor-context__hint">{basin.label} · {basin.wet ? '有水' : '已干涸'}</p>
            <button type="button" className="editor-tool" onClick={() => setInspector('environment')}>查看水域参数</button>
          </section>}
          {inspector === 'object' && pondStones && basin && <PondBedProperties
            key={`pond-bed:${observedWorld?.worldId}:${observedWorld?.runGeneration}:${basin.id}:${staticSection?.row ?? 'main'}`}
            items={pondBedObjects} selected={selectedPondObject} water={basin} hasWoods={Boolean(pondWoods)}
            attachments={basin.shrimpAttachments} mossPatches={pondMoss} scenery={pondScenery} editable={canPondEdit}
            staticSection={Boolean(staticSection)} onSelect={selectPondObject}
            onRemove={(id, kind) => editPond({
              type: kind === 'wood' ? 'removeWood' : 'removeStone', basinId: basin.id, id,
            })}
          />}
          {inspector === 'object' && pondWaterPlants && basin && <PondPlantProperties patches={pondWaterPlants}
            selected={selectedWaterPlant} editable={canPondEdit} onSelect={(id) => selectPondObject(id, 'waterPlant')}
            onPosition={adjustWaterPlant} onTrim={(id, mgC) => editPond({ type: 'trimWaterPlant', basinId: basin.id, id, mgC })}
            onRemove={(id) => editPond({ type: 'removeWaterPlant', basinId: basin.id, id })} />}
          {inspector === 'object' && !pondFloatingPlacement && basin && Boolean(basin.floatingPlants?.length) && <section className="editor-context__section">
            <h2>活体浮叶</h2>
            <PondFloatingReadout basin={basin} />
            {canPrimaryPondEdit && <button type="button" className="editor-tool" onClick={beginFloatingPlants}>编辑水面浮叶</button>}
          </section>}
          {inspector === 'object' && !pondSedgePlacement && !staticSection && Boolean(basin?.shoreSedges?.length) && <section className="editor-context__section">
            <h2>岸边植被</h2>
            <label className="editor-pond-field">选择莎草
              <select value={selectedSedge?.id ?? ''} onChange={event => selectSedge(Number(event.currentTarget.value))}>
                <option value="" disabled>请选择</option>
                {basin?.shoreSedges?.map(({ id }) => <option key={id} value={id}>岸边莎草 {id}</option>)}
              </select>
            </label>
            {selectedSedge && <SedgeReadout patch={selectedSedge} />}
            {canPrimaryPondEdit && <button type="button" className="editor-tool" onClick={beginShoreSedges}>编辑岸边莎草</button>}
          </section>}
          {inspector === 'object' && pondScenery && basin && <PondSceneryProperties
            items={pondScenery} selected={selectedScenery} editable={canPondEdit}
            onSelect={(id) => selectPondObject(id, 'scenery')}
            onRemove={(id) => editPond({ type: 'removeScenery', basinId: basin.id, id })}
          />}
          {inspector === 'object' && pondMoss && basin && <PondMossProperties
            key={`pond-moss:${observedWorld?.worldId}:${observedWorld?.runGeneration}:${basin.id}`}
            patches={pondMoss} selected={selectedMoss} carriers={pondBedObjects} editable={canPondEdit}
            select={(id) => selectPondObject(id, 'plant')}
            preview={setMossMoveOffset}
            move={(id, offset) => editPond({ type: 'moveMoss', basinId: basin.id, id, offset })}
            trim={(id, mgC) => editPond({ type: 'trimMoss', basinId: basin.id, id, mgC })}
            remove={(id) => editPond({ type: 'removeMoss', basinId: basin.id, id })}
          />}
          {inspector === 'environment' && basinAuthority && basins && <PondInspector
            key={`${observedWorld?.worldId}:${basinAuthority.id}`} basin={basinAuthority} basins={basins} selectBasin={selectPondBasin}
            editing={!staticSection && (editing || pondTrialEditing)} canEdit={canPrimaryPondEdit} onEdit={editPond}
            onShape={basinAuthority.shape && !staticSection ? beginPondShape : undefined} />}
          {inspector === 'biology' && basinAuthority && basin && <section className="editor-context__section">
            <h2>浅塘生物</h2>
            <dl className="editor-stock">
              <div><dt>{basinAuthority.staticSections?.length ? '当前剖面米虾' : '小虾'}</dt><dd data-pond-section-shrimp-count={basin.shrimp.length}>{basin.shrimp.length} 只</dd></div>
              {basin.shrimpLife && <div><dt>成体／幼体</dt><dd>
                {basin.shrimpLife.filter(({ stage }) => stage === 'adult').length}／{basin.shrimpLife.filter(({ stage }) => stage === 'juvenile').length} 只
              </dd></div>}
              <div><dt>整池水草</dt><dd>{basinAuthority.plantMgC.toFixed(2)} mg C</dd></div>
              <div><dt>整池生物膜</dt><dd>{basinAuthority.biofilmMgC.toFixed(2)} mg C</dd></div>
            </dl>
            {pondScenery && <>
              <PondBiofilmReadout basin={basin} editable={canPondEdit} localSection={Boolean(staticSection)}
                enable={() => { void editPond({ type: 'enableStoneBiofilm', basinId: basinAuthority.id }); }}
                enableWood={() => { void editPond({ type: 'enableWoodBiofilm', basinId: basinAuthority.id }); }} />
              <PondWoodFeedingReadout basin={basin} />
              <PondStoneFeedingReadout basin={basin} editable={canPondEdit}
                enable={() => { void editPond({ type: 'enableStoneRest', basinId: basinAuthority.id }); }} />
            </>}
          </section>}
          {inspector === 'biology' && basinAuthority && <PondDaphniaObservation basin={basinAuthority} />}
          {inspector !== 'object' && observedWorld?.forest &&
            <ForestDiagnostics data={observedWorld.forest} view={inspector} showSpringtails={!forestScenery} />}
          {inspector === 'biology' && observedWorld && (forestScenery || (pondScenery && basin)) && <AnimalIndividuals
            key={`${observedWorld.worldId}/${observedWorld.runGeneration}/${basin?.id ?? 'forest'}/${staticSection?.row ?? 'main'}`}
            kind={basin ? 'shrimp' : 'woodlouse'} animals={basin?.shrimp ?? observedWorld.animals}
            life={basin ? basin.shrimpLife : observedWorld.forest?.life?.woodlice} selectedId={selectedAnimalId}
            enabled={!activePlacement && (basin ? canPondEdit : canTerrainEdit)}
            onSelect={(id) => setAnimalSelection(id ? { authority: observedWorld, id, basinId: basin?.id, row: staticSection?.row } : null)}
            onLocate={() => { setPlacement(null); setSpringtailView(undefined); setZoom(2); setFocus('animal'); setTask('scene'); }}
            onRemove={(id) => {
              if (basin) void editPond({ type: 'removeShrimp', basinId: basin.id, id });
              else editForestRocks({ action: 'change', edit: { kind: 'removeWoodlouse', id } });
            }} />}
          {inspector === 'biology' && basinAuthority && pondScenery && !staticSection && <DaphniaRemoval
            key={`${observedWorld?.worldId}/${observedWorld?.runGeneration}/${basinAuthority.id}`}
            count={basinAuthority.daphniaCount} enabled={canPrimaryPondEdit && !activePlacement}
            onRemove={(count) => { void editPond({ type: 'removeDaphnia', basinId: basinAuthority.id, count }); }} />}
          {inspector === 'biology' && forestScenery && <ForestSpringtailObservation stock={observedWorld?.forest?.life?.springtailMgC}
            mode={springtailView} zoom={zoom} observe={observeSpringtails} />}
          {inspector === 'environment' && substrateCells && <ForestSubstrateReadings key={observedWorld?.worldId} cells={substrateCells} />}
          {inspector === 'object' && Boolean(forestMossPatches?.length) && <ForestPlantProperties kind="moss" patches={forestMossPatches ?? []}
            selected={selectedForestMoss} editable={canTerrainEdit} select={(id) => selectForestPlant('moss', id)} move={(id) => beginForestPlant('moss', id)}
            trim={(id, mgC) => editForestRocks({ action: 'change', edit: { kind: 'trimMossPatch', id, mgC } })}
            remove={(id) => editForestRocks({ action: 'change', edit: { kind: 'removeMossPatch', id } })} />}
          {inspector === 'object' && Boolean(forestFernPatches?.length) && <ForestPlantProperties kind="fern" patches={forestFernPatches ?? []}
            selected={selectedForestFern} initialMgC={forestFernPreset?.initialMgCnp[0]} editable={canTerrainEdit}
            select={(id) => selectForestPlant('fern', id)} move={(id) => beginForestPlant('fern', id)}
            trim={(id, mgC) => editForestRocks({ action: 'change', edit: { kind: 'trimFernPatch', id, mgC } })}
            remove={(id) => editForestRocks({ action: 'change', edit: { kind: 'removeFernPatch', id } })} />}
          {inspector === 'biology' && observedWorld && !basin && !observedWorld.forest && <section className="editor-context__section">
            <h2>林下生物</h2>
            <p className="editor-context__hint">鼠妇 {observedWorld.animals.length} 只</p>
          </section>}
          {inspector === 'biology' && observedWorld && (forestScenery || pondScenery) && <StockingControls
            key={`${observedWorld.worldId}/${basin?.id ?? 'land'}/${staticSection?.row ?? 'main'}`}
            options={basin ? [{ species: 'shrimp', capacity: Math.max(0, 12 - basin.shrimp.length) },
              ...(staticSection ? [] : [{ species: 'daphnia' as const, capacity: Math.max(0, 2048 - basin.daphniaCount) }])]
              : [{ species: 'woodlice', capacity: Math.max(0, 32 - observedWorld.animals.length) }]}
            disabled={basin ? !canPondEdit : !canTerrainEdit} needsWater={basin?.wet === false} onStart={beginStocking} />}
          {inspector === 'biology' && (forestScenery || pondScenery) && <SnailCandidateObservation
            key={`${observedWorld?.worldId}:${pondScenery ? 'pond' : 'forest'}`} theme={pondScenery ? 'pond' : 'forest'} />}
          {inspector === 'environment' && !basin && !observedWorld?.forest && observedWorld?.moisture && (
            <section className="editor-context__section">
              <h2>地表水分指标</h2>
              <div className="editor-moisture">
                {observedWorld.moisture.map((value, index) => (
                  <div className="editor-moisture__reading" key={index}>
                    <span>{index === 0 ? '左侧' : '右侧'}</span>
                    <meter min={0} max={1} value={value} aria-label={index === 0 ? '左侧水分指标' : '右侧水分指标'} />
                    <span>{value.toFixed(2)}</span>
                  </div>
                ))}
              </div>
              <p className="editor-context__hint">原型相对指标 0–1，随运行更新。</p>
            </section>
          )}
          {inspector === 'environment' && season && <section className="editor-context__section">
            <h2>世界气候</h2>
            <dl className="editor-stock">
              <div><dt>季节</dt><dd>{seasonLabels[season.season]} · 第 {season.year} 年第 {season.dayOfYear} 天</dd></div>
              <div><dt>气温</dt><dd>{season.airTemperatureCelsius.toFixed(1)} °C</dd></div>
              <div><dt>相对湿度</dt><dd>{Math.round(season.relativeHumidity * 100)}%</dd></div>
            </dl>
          </section>}
          {inspector === 'environment' && observedWorld && (observedWorld.forest || basinAuthority) && <ObservationTrends
            key={`${observedWorld.worldId}:${observedWorld.runGeneration}`} worldId={observedWorld.worldId}
            generation={observedWorld.runGeneration} basinId={basinAuthority?.id} basinLabel={basinAuthority?.label}
            springtails={Boolean(observedWorld.forest?.life)} trial={Boolean(trial)} active={viewActive} />}
          {inspector === 'environment' && native && !trial && (world?.forest || world?.pond) && (
            <section className="editor-context__section">
              <h2>日照地点</h2>
              {location ? (
                <form className="editor-location" onSubmit={saveLocation}>
                  <label>地点名称<input required maxLength={80} value={location.name} onChange={(event) => setLocation({ ...location, name: event.target.value })} /></label>
                  <label>纬度（°）<input required type="number" min={-90} max={90} step="any" value={location.latitude} onChange={(event) => setLocation({ ...location, latitude: event.target.value })} /></label>
                  <label>经度（°）<input required type="number" min={-180} max={180} step="any" value={location.longitude} onChange={(event) => setLocation({ ...location, longitude: event.target.value })} /></label>
                  <div className="editor-details__options">
                    <button type="button" className="editor-tool" onClick={() => setLocation(locationDraft(beijing))}>北京示例</button>
                    <button type="submit" className="editor-tool" disabled={!canAct}>应用</button>
                    <button type="button" className="editor-tool" onClick={() => setLocation(null)}>取消</button>
                  </div>
                </form>
              ) : (
                <div className="editor-location__summary">
                  <span>{world.location?.name || beijing.name}</span>
                  <button type="button" className="editor-tool" disabled={!canAct} onClick={() => setLocation(locationDraft(world.location ?? beijing))}>
                    更改地点
                  </button>
                </div>
              )}
              {world.solar && <SolarStatus data={world.solar} phase={world.daylightPhase} paused={world.paused || world.suspended} />}
            </section>
          )}
          {!trial && inspector === 'object' && <details className="editor-details">
            <summary>画面与存档</summary>
            <div className="editor-details__body">
              {(forestRocks || pondStones) && <fieldset>
                <legend>界面材质</legend>
                <div className="editor-details__options">
                  {([['mist', '朝雾浅玻'], ['obsidian', '夜林深玻']] as const).map(([id, label]) => (
                    <button type="button" key={id} className="editor-tool" data-appearance-choice={id}
                      aria-pressed={appearance === id} onClick={() => {
                        setAppearance(id);
                        try { window.localStorage.setItem('micro-eco-appearance', id); } catch { /* ignore */ }
                      }}>{label}</button>
                  ))}
                </div>
              </fieldset>}
              <fieldset>
                <legend>光照预览</legend>
                <div className="editor-details__options">
                  {lightOptions.map(({ id, label }) => (
                    <button type="button" key={id} className="editor-tool" data-light-preview={id}
                      aria-pressed={lightPreview === id} onClick={() => setLight(id)}>{label}</button>
                  ))}
                </div>
              </fieldset>
              {native && (
                <div>
                  {confirmRestore ? (
                    <>
                      <p className="editor-context__hint">读取会用已保存状态替换当前世界，并结束本轮撤销。</p>
                      <div className="editor-details__options">
                        <button type="button" className="editor-tool" onClick={restoreWorld} disabled={!canAct}>确认读取</button>
                        <button type="button" className="editor-tool" onClick={() => setConfirmRestore(false)}>取消</button>
                      </div>
                    </>
                  ) : <button type="button" className="editor-tool" onClick={() => setConfirmRestore(true)} disabled={!canAct}>读取检查点</button>}
                </div>
              )}
              {renderer === 'webgl' && <p className="editor-context__hint">WebGL</p>}
            </div>
          </details>}
        </aside>
      </div>

      <footer className="editor-footer">
        <span>{native ? '从 Windows 系统托盘打开其他窗口' : '在 Windows 应用中可编辑世界'}</span>
        <div className="editor-feedbacks">
          {feedback && feedback !== saveFeedback && <span role="status" className="editor-feedback"
            data-error={Boolean(connectionError || notice?.error)}>{feedback}</span>}
          {saveFeedback && <span role="status" className="editor-feedback" data-error={Boolean(saveFailure || saveFeedback.includes('失败'))}>
            {saveFeedback}
          </span>}
        </div>
      </footer>
      {libraryOpen && <WorldLibrary busy={pending !== null} canAct={canAct} dismiss={() => setLibraryOpen(false)}
        onAction={libraryAction} operationError={notice?.error ? notice.text : undefined} />}
    </main>
  );
}
