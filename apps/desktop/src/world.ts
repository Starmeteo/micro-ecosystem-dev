import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';

export type ViewId = 'editor' | 'observe' | 'panel' | 'layout';

export interface ScenePoint {
  x: number;
  y: number;
}

export interface ForestRock {
  id: number;
  position: ScenePoint;
  mirrored: boolean;
}

export type ForestLog = ForestRock;
export type StoneworkPreset = 'slab' | 'pair' | 'cluster';
export type ForestStonework = ForestRock & { preset: StoneworkPreset };
export type BarkShelterPreset = 'arch' | 'split' | 'rootArch' | 'branchBridge';
export type ForestBarkShelter = ForestRock & {
  preset: BarkShelterPreset;
  bridgeClearanceMm?: readonly [number, number];
  footCells: [left: number, right: number];
  passageCells: [back: number, center: number, front: number];
  occupantIds: string[];
};
export type LeafPatchPreset = 'leaves' | 'heap' | 'fragments';
export type MatterMgCnp = [carbonMg: number, nitrogenMg: number, phosphorusMg: number];
export type LeafPatchMatter = MatterMgCnp;
export interface ForestLeafPreset {
  preset: LeafPatchPreset;
  initialMgCnp: LeafPatchMatter;
}
export type ForestLeafPatch = ForestRock & {
  preset: LeafPatchPreset;
  initialMgCnp: LeafPatchMatter;
  remainingMgCnp: LeafPatchMatter;
  cellIndex: number;
};
export interface ForestMossPatch {
  id: number;
  position: ScenePoint;
  cellIndex: number;
  stockMgCnp: MatterMgCnp;
  stressFraction: number;
}
export interface ForestFernPatch extends ForestMossPatch {
  canopyShadeFraction: number;
}
export type ForestSubstrate = 'native' | 'fineSoil' | 'coarseHumus';
export interface ForestSubstratePreset {
  substrate: ForestSubstrate;
  waterCapacityMl: number;
  evaporationMultiplier: number;
}
export interface ForestSubstrateCell extends ForestSubstratePreset {
  cellIndex: number;
  position: ScenePoint;
  waterMl: number;
  decomposerMgC?: number;
}
export type ForestTerrainKind = 'rock' | 'log' | 'stonework' | 'barkShelter' | 'leafPatch';

export interface ForestSceneryParent {
  kind: 'terrain' | 'scenery';
  id: number;
}

export interface ForestScenery extends ScenePoint {
  id: number;
  style: string;
  parent?: ForestSceneryParent | null;
}

export type ForestRockEdit =
  | { kind: 'add' | 'addLog'; position: ScenePoint }
  | { kind: 'addStonework'; position: ScenePoint; preset: StoneworkPreset }
  | { kind: 'addBarkShelter'; position: ScenePoint; preset: BarkShelterPreset }
  | { kind: 'addLeafPatch'; position: ScenePoint; preset: LeafPatchPreset }
  | { kind: 'paintLeafPatches'; positions: ScenePoint[]; preset: LeafPatchPreset }
  | { kind: 'spreadLeafFragments'; positions: ScenePoint[]; mgC: number }
  | { kind: 'setSubstrate'; cells: number[]; substrate: ForestSubstrate }
  | { kind: 'stockWoodlice'; position: ScenePoint; count: number }
  | { kind: 'removeWoodlouse'; id: string }
  | { kind: 'addMossPatch'; position: ScenePoint }
  | { kind: 'moveMossPatch'; id: number; position: ScenePoint }
  | { kind: 'trimMossPatch'; id: number; mgC: number }
  | { kind: 'removeMossPatch'; id: number }
  | { kind: 'addFernPatch'; position: ScenePoint }
  | { kind: 'moveFernPatch'; id: number; position: ScenePoint }
  | { kind: 'trimFernPatch'; id: number; mgC: number }
  | { kind: 'removeFernPatch'; id: number }
  | { kind: 'move'; id: number; position: ScenePoint }
  | { kind: 'mirror' | 'remove'; id: number }
  | ({ kind: 'addScenery'; style: string; parent?: ForestSceneryParent } & ScenePoint)
  | ({ kind: 'moveScenery'; id: number } & ScenePoint)
  | { kind: 'removeScenery'; id: number };

export type ForestRockAction =
  | { action: 'change'; edit: ForestRockEdit }
  | { action: 'undo' | 'redo' };

export interface FixtureAnimal extends ScenePoint {
  id: string;
  heading: number;
  activity: string;
}

export interface ForestReadings {
  waterMl: readonly [number, number];
  shadeFraction: readonly [number, number];
  shelterFraction: readonly [number, number];
  evaporationMlPerHour: readonly [number, number];
  litterMgC: number;
  soilNitrogenMg: number;
  soilPhosphorusMg: number;
  woodliceBySide: readonly [number, number];
  shelteredWoodliceBySide: readonly [number, number];
  feedingWoodliceBySide: readonly [number, number];
  life?: {
    woodlice: { id: string; stage: 'adult' | 'juvenile'; ageSeconds: number; biomassMgC: number; broodProgress: number | null }[];
    woodliceBorn: number;
    woodliceDied: number;
    springtailMgC: readonly [number, number];
    fernMgC: readonly [number, number];
    mossMgC: readonly [number, number];
    fernStressFraction: readonly [number, number];
    mossStressFraction: readonly [number, number];
  };
}

export type PondShapeMode = 'add' | 'remove';
export interface PondShapeSnapshot {
  columns: number;
  rows: number;
  cells: number[];
  protectedRow: number;
  cellAreaCm2: number;
}

export interface PondStaticSection {
  row: number;
  stones: PondStone[];
  woods: PondWood[];
  scenery: PondScenery[];
  leafDeposits?: PondLeafDeposit[];
  waterPlantPatches?: PondWaterPlantPatch[];
  mossPatches?: PondMossPatch[];
  mossPresets?: PondMossDefinition[];
  shrimp?: FixtureAnimal[];
  shrimpLife?: readonly PondShrimpLife[];
  shrimpAttachments?: PondShrimpAttachment[];
  woodContactFeeding?: readonly PondWoodContactFeeding[];
  stoneRestEnabled?: boolean;
  stoneAttachments?: readonly PondShrimpStoneAttachment[];
  stoneContactFeeding?: readonly PondStoneContactFeeding[];
  stoneBiofilmEnabled?: boolean;
  stoneBiofilm?: readonly PondStoneBiofilm[];
  biofilmDistribution?: PondBiofilmDistribution;
}

export interface PondBasinSnapshot {
  id: string;
  label: string;
  wet: boolean;
  waterLiters: number;
  shape?: PondShapeSnapshot;
  retainObservationRow?: boolean;
  capacityLiters: number;
  depthCm: number;
  maxDepthCm: number;
  surfaceAreaCm2: number;
  temperatureCelsius: number | null;
  oxygenMgPerLiter: number | null;
  availableNitrogenMgPerLiter: number | null;
  availablePhosphorusMgPerLiter: number | null;
  algaeMgC: number;
  biofilmMgC: number;
  biofilmDistribution?: PondBiofilmDistribution;
  stoneBiofilmEnabled?: boolean;
  stoneBiofilm?: readonly PondStoneBiofilm[];
  plantMgC: number;
  detritusMgC: number;
  leafDeposits?: PondLeafDeposit[];
  daphniaCount: number;
  daphniaStockMgCnp?: MatterMgCnp;
  shrimp: FixtureAnimal[];
  shrimpLife?: readonly PondShrimpLife[];
  turbidityIndex: number;
  stones?: PondStone[] | null;
  woods?: PondWood[] | null;
  shrimpAttachments?: PondShrimpAttachment[] | null;
  woodContactFeeding?: readonly PondWoodContactFeeding[];
  stoneRestEnabled?: boolean;
  stoneAttachments?: readonly PondShrimpStoneAttachment[];
  stoneContactFeeding?: readonly PondStoneContactFeeding[];
  mossPatches?: PondMossPatch[] | null;
  mossPresets?: PondMossDefinition[] | null;
  waterPlantPatches?: PondWaterPlantPatch[] | null;
  unassignedPlantMgCnp?: MatterMgCnp | null;
  floatingPlants?: readonly PondFloatingPlant[];
  floatingCanopy?: PondFloatingCanopy;
  shoreSedges?: readonly PondSedgePatch[];
  scenery?: PondScenery[] | null;
  staticSections?: PondStaticSection[];
}

export type PondShrimpStage = 'juvenile' | 'adult';
export interface PondShrimpLife {
  id: string;
  stage: PondShrimpStage;
  ageSeconds: number;
}

export type PondStonePreset = 'main' | 'pebble' | 'flatStoneCave';
export interface PondStone {
  id: number;
  x: number;
  preset: PondStonePreset;
}

export type PondWoodPreset = 'branch' | 'root' | 'slope' | 'hangingRoots' | 'rootNetwork';
export interface PondWood {
  id: number;
  x: number;
  preset: PondWoodPreset;
}

export interface PondShrimpAttachment {
  shrimpId: string;
  woodId: number;
  offset: number;
  restTicks: number;
}

export interface PondWoodContactFeeding {
  shrimpId: string;
  woodId: number;
  offset: number;
  biofilmMgC: number;
  woodBiofilmMgC?: number;
  sharedBiofilmMgC?: number;
}

export interface PondShrimpStoneAttachment {
  shrimpId: string;
  stoneId: number;
  offset: number;
  restTicks: number;
}

export interface PondStoneContactFeeding {
  shrimpId: string;
  stoneId: number;
  offset: number;
  biofilmMgC: number;
  stoneBiofilmMgC: number;
  sharedBiofilmMgC: number;
}

export interface PondBiofilmSurface {
  kind: 'stone' | 'wood';
  id: number;
  mgC: number;
}

export interface PondBiofilmDistribution {
  bedMgC: number;
  inactiveMgC: number;
  surfaces: readonly PondBiofilmSurface[];
}

export interface PondStoneBiofilm {
  stoneId: number;
  stockMgC: number;
  availableMgC: number;
}

export type PondMossPreset = 'mat' | 'frond';
export interface PondMossParent {
  kind: 'stone' | 'wood';
  id: number;
}

export interface PondMossDefinition {
  preset: PondMossPreset;
  initialMgCnp: MatterMgCnp;
}

export interface PondMossPatch {
  id: number;
  parent: PondMossParent;
  offset: number;
  preset: PondMossPreset;
  stockMgCnp: MatterMgCnp;
  x: number;
  surfaceHeightPx: number;
  submerged: boolean;
}

export type PondLeafDepositPreset = 'midground' | 'foreground';
export type PondLeafDepositStage = 'whole' | 'fragments' | 'fine';
export interface PondLeafDeposit {
  id: number;
  x: number;
  preset: PondLeafDepositPreset;
  initialMgC: number;
  remainingMgC: number;
  stage: PondLeafDepositStage;
}

export type PondRibbonPlantPreset = 'small' | 'medium' | 'large';

export interface PondWaterPlantPatch {
  id: number;
  x: number;
  initialMgCnp: MatterMgCnp;
  stockMgCnp: MatterMgCnp;
  submerged: boolean;
  ribbonPreset?: PondRibbonPlantPreset | null;
  ribbonLightTransmission?: number | null;
}

export type PondFloatingPlantPreset = 'small' | 'medium' | 'large';

export type PondSedgePreset = 'small' | 'medium' | 'large';

export interface PondShoreAnchor {
  waterCell: number;
  shoreCell: number;
}

export interface PondSedgePatch {
  id: number;
  anchor: PondShoreAnchor;
  preset: PondSedgePreset;
  shootHeightMm: number;
  rootDiameterMm: number;
  rootCollarMm: number;
  rootWetFraction: number;
  initialMgCnp: MatterMgCnp;
  stockMgCnp: MatterMgCnp;
}

export interface PondFloatingPlant {
  id: number;
  position: ScenePoint;
  preset: PondFloatingPlantPreset;
  initialMgCnp: MatterMgCnp;
  stockMgCnp: MatterMgCnp;
  diameterMm: number;
  floating: boolean;
}

export interface PondFloatingCanopy {
  coveredAreaCm2: number;
  coverage: number;
  lightTransmission: number;
}

export interface PondScenery {
  id: number;
  style: string;
  x: number;
  y: number;
  parent?: { kind: 'wood' | 'scenery'; id: number } | null;
}

type PondStaticEditBody =
  | { type: 'addStone'; x: number; preset: PondStonePreset }
  | { type: 'moveStone'; id: number; x: number }
  | { type: 'removeStone'; id: number }
  | { type: 'addWood'; x: number; preset: PondWoodPreset }
  | { type: 'moveWood'; id: number; x: number }
  | { type: 'removeWood'; id: number }
  | { type: 'addScenery'; style: string; x: number; y: number }
  | { type: 'moveScenery'; id: number; x: number; y: number }
  | { type: 'removeScenery'; id: number }
  | { type: 'addLeafDeposit'; x: number; preset: PondLeafDepositPreset; mgC: number }
  | { type: 'moveLeafDeposit'; id: number; x: number }
  | { type: 'removeLeafDeposit'; id: number }
  | { type: 'addWaterPlant'; x: number }
  | { type: 'placeRibbonPlant'; x: number; preset: PondRibbonPlantPreset }
  | { type: 'moveWaterPlant'; id: number; x: number }
  | { type: 'trimWaterPlant'; id: number; mgC: number }
  | { type: 'removeWaterPlant'; id: number }
  | { type: 'stockShrimp'; position: ScenePoint; count: number }
  | { type: 'removeShrimp'; id: string }
  | { type: 'enableStoneRest' }
  | { type: 'enableStoneBiofilm' }
  | { type: 'addMoss'; parent: PondMossParent; offset: number; preset: PondMossPreset }
  | { type: 'moveMoss'; id: number; offset: number }
  | { type: 'trimMoss'; id: number; mgC: number }
  | { type: 'removeMoss'; id: number };

export type PondStaticEdit = { basinId: string } & PondStaticEditBody;

export type PondEdit = { basinId: string } & (
  | PondStaticEditBody
  | { type: 'editStaticSection'; row: number; edit: PondStaticEdit }
  | { type: 'addWater' | 'drainWater'; liters: number }
  | { type: 'reshape'; surfaceAreaCm2: number }
  | { type: 'paintShape'; mode: PondShapeMode; cells: number[] }
  | { type: 'commitTopology'; mode: PondShapeMode; cells: number[] }
  | { type: 'addDetritus'; mgC: number }
  | { type: 'stockDaphnia'; count: number }
  | { type: 'removeDaphnia'; count: number }
  | { type: 'addFloatingPlant'; position: ScenePoint; preset: PondFloatingPlantPreset }
  | { type: 'addSedge'; anchor: PondShoreAnchor; preset: PondSedgePreset }
  | { type: 'moveSedge'; id: number; anchor: PondShoreAnchor }
  | { type: 'removeSedge'; id: number }
  | { type: 'moveFloatingPlant'; id: number; position: ScenePoint }
  | { type: 'removeFloatingPlant'; id: number }
);

export interface SeasonReadings {
  year: number;
  dayOfYear: number;
  season: 'winter' | 'spring' | 'summer' | 'autumn';
  airTemperatureCelsius: number;
  relativeHumidity: number;
}

export interface WorldLocation {
  latitudeDegrees: number;
  longitudeDegrees: number;
  name: string;
}

export interface SolarReadings {
  targetPhase: number;
  sunriseUtcSeconds: number | null;
  sunsetUtcSeconds: number | null;
  dayKind: 'riseSet' | 'continuousDay' | 'continuousNight' | 'singleCrossing';
}

export interface WorldSnapshot {
  worldId: string;
  worldName?: string | null;
  runGeneration: string;
  sequence: string;
  editRevision: string;
  simulationTick: string;
  simulatedSeconds: number;
  paused: boolean;
  manualPaused?: boolean;
  editPaused?: boolean;
  suspended: boolean;
  daylightPhase: number;
  modelVersion?: number;
  definition?: string;
  starterCommunityAvailable?: boolean;
  pondUndoAvailable?: boolean;
  pondRedoAvailable?: boolean;
  rock: ScenePoint;
  forestRocks?: {
    rocks: ForestRock[];
    logs?: ForestLog[] | null;
    stonework?: ForestStonework[] | null;
    barkShelters?: ForestBarkShelter[] | null;
    leafPatches?: ForestLeafPatch[] | null;
    leafPresets?: ForestLeafPreset[] | null;
    mossPatches?: ForestMossPatch[];
    mossPreset?: { initialMgCnp: MatterMgCnp; widthMm: number };
    fernPatches?: ForestFernPatch[];
    fernPreset?: { initialMgCnp: MatterMgCnp; heightMm: number; widthMm: number };
    substrateCells?: ForestSubstrateCell[];
    substratePresets?: ForestSubstratePreset[];
    scenery?: ForestScenery[] | null;
    undoAvailable: boolean;
    redoAvailable: boolean;
  };
  animals: FixtureAnimal[];
  moisture?: readonly [number, number];
  forest?: ForestReadings;
  pond?: { basins: PondBasinSnapshot[] };
  season?: SeasonReadings;
  location?: WorldLocation;
  solar?: SolarReadings;
  lastSaveMessage?: string | null;
}

export function worldName(snapshot: WorldSnapshot | null | undefined) {
  return snapshot?.worldName || (snapshot?.pond ? '浅水池塘' : '林下切片');
}

interface SubscriptionBaseline {
  subscriptionToken: string;
  snapshot: WorldSnapshot;
}

export const native = isTauri();
const label = native ? getCurrentWindow().label : new URLSearchParams(window.location.search).get('view') ?? 'panel';
export const viewId: ViewId = label === 'observe' || label === 'panel' || label === 'layout' ? label : 'editor';

const preview: WorldSnapshot = {
  worldId: 'browser-preview',
  runGeneration: 'preview',
  sequence: '0',
  editRevision: '0',
  simulationTick: '0',
  simulatedSeconds: 0,
  paused: true,
  suspended: false,
  daylightPhase: 0.49,
  rock: { x: 0.54, y: 0.72 },
  animals: [
    { id: 'preview-a', x: 0.42, y: 0.78, heading: 0, activity: 'moving' },
    { id: 'preview-b', x: 0.61, y: 0.84, heading: 2.6, activity: 'resting' },
  ],
};

export function errorText(error: unknown): string {
  if (typeof error === 'string') {
    try {
      const parsed: unknown = JSON.parse(error);
      if (parsed !== error) return errorText(parsed);
    } catch {
      return error;
    }
  }
  if (error && typeof error === 'object' && 'message' in error) return String(error.message);
  return String(error);
}

function errorCode(error: unknown, code: string): boolean {
  if (typeof error === 'string') return error.includes(code);
  return Boolean(error && typeof error === 'object' && 'code' in error && error.code === code);
}

export function useWorld(worldId?: string, enabled = true, slot?: string) {
  const [snapshot, setSnapshot] = useState<WorldSnapshot | null>(native ? null : preview);
  const [connectionError, setConnectionError] = useState('');
  const [viewActive, setViewActive] = useState(!native);

  useEffect(() => {
    if (!native || !enabled) return;
    let closed = false;
    let timer: number | undefined;
    let baseline: SubscriptionBaseline | undefined;
    let startupRetries = 0;

    const unsubscribe = (subscriptionToken: string) => {
      void invoke('unsubscribe_world', { viewId, subscriptionToken, slot }).catch(() => undefined);
    };

    async function pull() {
      let delay = document.hidden ? 900 : 120;
      try {
        if (!baseline) {
          const response = await invoke<SubscriptionBaseline>('subscribe_world', { viewId, worldId, slot });
          if (closed) {
            unsubscribe(response.subscriptionToken);
            return;
          }
          baseline = response;
          setSnapshot(response.snapshot);
          setViewActive(true);
        } else {
          const current = baseline;
          const response = await invoke<WorldSnapshot | null>('poll_world', {
            viewId,
            subscriptionToken: current.subscriptionToken,
            slot,
            afterSequence: current.snapshot.sequence,
          });
          if (closed) return;
          if (response) {
            const sameAuthority = response.worldId === current.snapshot.worldId
              && response.runGeneration === current.snapshot.runGeneration;
            if (!sameAuthority) {
              unsubscribe(current.subscriptionToken);
              baseline = undefined;
              delay = 0;
            } else if (BigInt(response.sequence) > BigInt(current.snapshot.sequence)) {
              baseline = { ...current, snapshot: response };
              setSnapshot(response);
            }
          }
        }
        setConnectionError('');
        startupRetries = 0;
      } catch (error) {
        if (closed) return;
        if (errorCode(error, 'stale_subscription')) {
          baseline = undefined;
          delay = 0;
        } else if (errorCode(error, 'view_hidden')) {
          baseline = undefined;
          setViewActive(false);
          delay = 900;
        } else if (errorCode(error, 'bound_world_unavailable')) {
          baseline = undefined;
          setSnapshot(null);
          setViewActive(true);
          setConnectionError(errorText(error));
          delay = 900;
        } else if (!baseline && startupRetries < 12 && errorCode(error, 'state not managed')) {
          // Configured WebViews can start before Tauri has installed the shared host.
          startupRetries += 1;
          delay = 140;
        } else {
          setConnectionError(errorText(error));
          delay = 600;
        }
      } finally {
        if (!closed) timer = window.setTimeout(() => void pull(), delay);
      }
    }

    void pull();
    return () => {
      closed = true;
      window.clearTimeout(timer);
      if (baseline) unsubscribe(baseline.subscriptionToken);
    };
  }, [worldId, enabled, slot]);

  return { snapshot: native && worldId && snapshot?.worldId !== worldId ? null : snapshot, connectionError, viewActive };
}
