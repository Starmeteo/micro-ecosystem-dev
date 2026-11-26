import type { PondBasinSnapshot, PondLeafDeposit, PondLeafDepositPreset, PondRibbonPlantPreset, PondScenery, PondShrimpStage, PondWaterPlantPatch } from '../../world';
import type { R5ForestView } from './ForestLiveScene';
import { clamp, type Point, type Rect } from './geometry';
import {
  composedWidth, composedHeight, composedPixelsPerMm, composedPhysicalStyles, sceneryStyles, spritePose, spritePoint,
  drawScenerySprite, drawShadow, drawFloatRings, spriteBounds, hitsSprite,
  type ComposedArt, type SceneryStyle, type SpritePose,
} from './pondComposedArt';
import { paintPondBase, paintPondLight, paintPondSurface } from './pondComposedOptics';
import { bedY, bedPoint, pondRockShrimpPoint, pondTerrainObjects, stoneWaterHeight,
  type PondStoneDraft, type PondStoneHint, type PondTerrainObject } from './pondStoneGeometry';
import { mossVisual, type PondMossPreview } from './pondMoss';
import { type PondWaterPlantDraft, ribbonPlantDimensions, waterPlantFrame, waterPlantGrowth } from './pondWaterPlant';
import { pondBiofilmDisplay } from './pondBiofilm';
import { drawPondSurfaceFilm } from './pondBiofilmArt';
import { currentPondWoodFeeding } from './pondWoodFeeding';
import { currentPondStoneContacts, currentPondStoneFeeding } from './pondStoneFeeding';
import { pondStructureProfile } from './pondStructureProfiles';
import { markFloatingPlant, paintFloatingBlades, paintFloatingRoots, paintFloatingShade, pondFloatingVisuals } from './pondFloatingArt';
import { markPondSedge, paintPondSedges, pondSedgeVisuals } from './pondSedgeArt';

const scale = 2.028;
const left = 99;
const top = bedY - 265 / scale;
export const nominalSurface = (bedY - 130 - top) * scale;
const widget = { x: 0, y: 0, width: composedWidth, height: composedHeight };
const fullPond = { x: -72, y: -40.5, width: 720, height: 405 };
export function stageView(view: Rect): Rect {
  return { x: left + view.x / scale, y: top + view.y / scale, width: view.width / scale, height: view.height / scale };
}

export function atStage(point: Point): Point {
  return { x: (point.x - left) * scale, y: (point.y - top) * scale };
}

export function cameraAtStage(view: Rect): Rect {
  return { ...atStage(view), width: view.width * scale, height: view.height * scale };
}

export function pondCompositionCamera(
  pond: PondBasinSnapshot, underwater: boolean, view?: R5ForestView, contact?: Point, placing = false,
): Rect {
  const surface = compositionSurface(pond);
  if (underwater) {
    // Keep the bed in view as shallower water brings the surface down.
    return { ...widget, y: clamp((surface - nominalSurface) * .19, -6, 34) };
  }
  // Full framing includes the actual waterline even at the maximum fill level.
  const y = Math.min(fullPond.y, surface - 22);
  const height = fullPond.y + fullPond.height - y;
  const width = height * 16 / 9;
  const whole = { x: (composedWidth - width) / 2, y, width, height };
  const zoom = placing ? 1 : view?.zoom ?? 1;
  if (zoom === 1) return whole;
  const animal = pond.shrimp.find(({ id }) => id === view?.animalId) ?? pond.shrimp[0];
  const focus = view?.focus === 'animal' && animal ? atStage(pondRockShrimpPoint(animal, pond))
    : view?.focus === 'contact' && contact ? contact : { x: composedWidth * .50, y: composedHeight * .57 };
  const w = whole.width / zoom;
  const h = whole.height / zoom;
  return {
    width: w, height: h, x: clamp(focus.x - w / 2, whole.x, whole.x + whole.width - w),
    y: clamp(focus.y - h / 2, whole.y, whole.y + whole.height - h),
  };
}

export function compositionSurface(pond: PondBasinSnapshot) {
  return atStage({ x: 0, y: bedY - stoneWaterHeight(pond) }).y;
}

export function compositionSedges(pond: PondBasinSnapshot) {
  return pondSedgeVisuals(pond, atStage({ x: 0, y: bedY }).y,
    compositionSurface({ ...pond, wet: true, depthCm: pond.maxDepthCm }));
}

type ScenePick = PondTerrainObject | { kind: 'scenery'; id: number } | { kind: 'plant'; id: number }
  | { kind: 'waterPlant'; id: number } | { kind: 'leafDeposit'; id: number };

export interface SceneryDraft extends Point {
  id: number | null;
  style: string;
  bedSite?: 'ground' | 'outside';
}

export interface PondLeafDraft {
  id: number | null;
  preset: PondLeafDepositPreset;
  x: number;
}

export interface ComposedLayer {
  style: SceneryStyle;
  pose: SpritePose;
  pick: ScenePick;
  parentPose?: SpritePose;
  draft?: boolean;
  bedSite?: SceneryDraft['bedSite'];
  structureRear?: SpritePose;
}

function stagePosition(item: Pick<PondScenery, 'x' | 'y'>, style: SceneryStyle, surface: number) {
  return { x: item.x * composedWidth, y: item.y * composedHeight + (style.rings ? surface - nominalSurface : 0) };
}

function materialStyle(item: PondTerrainObject): SceneryStyle {
  if (item.kind === 'stone') {
    if (item.preset === 'main') return { ...composedPhysicalStyles.main, mm: 80, flat: .88, shadow: [150, 25, .57] };
    return { type: 'pebble-16', band: 3, mm: 25, shade: .82, shadow: [48, 9, .44] };
  }
  if (item.preset === 'slope') return composedPhysicalStyles.slope;
  const long = item.preset === 'branch';
  return { type: 'wood-ridged', band: 3, mm: long ? 300 : 150, flat: .31, angle: .084, shade: .91,
    shadow: long ? [345, 15, .49] : [173, 11, .51] };
}

function physicalPose(art: ComposedArt, style: SceneryStyle, item: PondTerrainObject) {
  const pose = spritePose(art, style, atStage(bedPoint(item.x)));
  if (item.kind === 'wood' && item.preset !== 'slope') {
    return { ...pose, art: { ...pose.art, anchor: [.5, .88] } };
  }
  return pose;
}

function visualMoss(art: ComposedArt, visual: ReturnType<typeof mossVisual>, draft = false): ComposedLayer {
  const { frame, id, point, preset } = visual;
  const style = {
    type: preset === 'mat' ? 'moss-ledge' : 'moss-fringe', band: 3,
    mm: preset === 'mat' ? 44 : 33, shade: frame === 'exposed' ? .71 : 1.04,
    flat: frame === 'sparse' ? .72 : 1, opacity: frame === 'sparse' ? .62 : 1,
  };
  return { style, pose: spritePose(art, style, atStage(point)), pick: { kind: 'plant', id: id ?? -1 }, draft };
}

function visualWaterPlant(art: ComposedArt, x: number, patch?: PondWaterPlantPatch, draft = false): ComposedLayer {
  const growth = patch ? Math.sqrt(waterPlantGrowth(patch)) : 1;
  const frame = patch ? waterPlantFrame(patch) : 'full';
  const exposed = frame === 'exposed' || frame === 'depleted';
  const style: SceneryStyle = {
    type: 'hornwort-100', band: 2, mm: (24 + 76 * growth) * (exposed ? .83 : 1),
    shade: exposed ? .66 : .91, saturation: exposed ? .48 : .91,
    opacity: (draft ? .72 : 1) * (frame === 'depleted' ? .28 : frame === 'sparse' ? .76 : 1),
    flip: (patch?.id ?? 1) % 2 === 0, shadow: [25, 8, .32],
  };
  return { style, pose: spritePose(art, style, atStage(bedPoint(x))), pick: { kind: 'waterPlant', id: patch?.id ?? -1 }, draft };
}

// Non-translucent silhouette of ribbon-180.png. Its calibrated anchor is the common leaf base.
const ribbonCanopyTop = .137;
const ribbonCanopyWidth = .773;

function visualRibbonPlant(art: ComposedArt, pond: PondBasinSnapshot, x: number, preset: PondRibbonPlantPreset,
  patch?: PondWaterPlantPatch, draft = false): ComposedLayer {
  const { leafArcMm, spreadMm } = ribbonPlantDimensions[preset];
  const stock = patch ? waterPlantGrowth(patch) : 1;
  const growth = .42 + .58 * Math.sqrt(Math.min(stock, 1.44));
  const at = atStage(bedPoint(x));
  const style: SceneryStyle = {
    type: 'ribbon-180', band: 2, mm: leafArcMm * growth, rootCut: true, flip: (patch?.id ?? 1) % 2 === 0,
    shade: pond.wet ? .83 + .10 * Math.min(1, stock) : .68,
    saturation: pond.wet ? .96 : .60,
    opacity: (draft ? .72 : 1) * (.32 + .68 * Math.min(1, stock)),
    shadow: [spreadMm * .43, 7, .29],
  };
  const natural = spritePose(art, style, at);
  const canopyRows = natural.art.h * (natural.art.anchor[1] - ribbonCanopyTop);
  const available = pond.wet
    ? Math.min(pond.depthCm * 10 * composedPixelsPerMm * .94, (at.y - compositionSurface(pond)) * .8)
    : spreadMm * composedPixelsPerMm * .16;
  // The existing leaves have curved blades. Lower water projects those arcs across the bed
  // instead of drawing their full length upright or cutting the tips off at the surface.
  const scaleX = spreadMm * growth * composedPixelsPerMm / (natural.art.w * ribbonCanopyWidth);
  const height = available > 0 ? available * -Math.expm1(-natural.scale * canopyRows / available) : 0;
  const scaleY = Math.max(.2, height) / canopyRows;
  return { style, pose: { ...natural, scale: scaleX, flat: scaleY / scaleX },
    pick: { kind: 'waterPlant', id: patch?.id ?? -1 }, draft };
}

function leafDepositLayers(art: ComposedArt, deposit: Pick<PondLeafDeposit, 'preset' | 'stage' | 'x' | 'id'>,
  wet: boolean, draft: boolean): ComposedLayer[] {
  const center = atStage(bedPoint(deposit.x));
  const large = deposit.preset === 'foreground';
  const parts = deposit.stage === 'whole' ? [[0, 0, large ? 105 : 55, -.12]]
    : deposit.stage === 'fragments' ? [[-9, -3, large ? 30 : 22, -.39], [12, 1, large ? 23 : 15, .24]]
    : [[-8, -2, 8, -.30], [0, 2, 12, .41], [9, 0, 6, -.69]];
  return parts.map(([x, y, mm, angle], index) => {
    const style: SceneryStyle = {
      type: large && deposit.stage === 'whole' ? 'leaf-105' : 'leaf-55', band: 1.9,
      mm, angle, flat: .66, flip: (deposit.id + index) % 2 === 0,
      shade: wet ? .84 : .90, saturation: wet ? .82 : .71, opacity: draft ? .70 : .96,
      shadow: [mm * 1.12, mm * .21, .48],
    };
    return { style, pose: spritePose(art, style, { x: center.x + x, y: center.y + y }), pick: { kind: 'leafDeposit', id: deposit.id }, draft };
  });
}

export function composedLayers(art: ComposedArt, pond: PondBasinSnapshot, sceneryDraft?: SceneryDraft | null,
  terrainDraft?: PondStoneDraft | null, mossDraft?: PondMossPreview, waterPlantDraft?: PondWaterPlantDraft | null,
  leafDraft?: PondLeafDraft): ComposedLayer[] {
  const surface = compositionSurface(pond);
  const scenery = pond.scenery ?? [];
  const terrain = pondTerrainObjects(pond);
  const roots: ComposedLayer[] = [];
  const attachments = new Map<string, PondScenery[]>();
  for (const item of scenery) {
    if (item.parent) {
      const key = item.parent.kind + '/' + item.parent.id;
      const children = attachments.get(key) ?? [];
      children.push(item);
      attachments.set(key, children);
    }
  }

  function decor(item: PondScenery, parentPose?: SpritePose): ComposedLayer | undefined {
    const draft = sceneryDraft?.id === item.id ? sceneryDraft : undefined;
    const style = sceneryStyles.get(item.style);
    if (!style || (style.rings && !pond.wet)) return;
    const position = draft ?? item;
    const at = parentPose ? spritePoint(parentPose, [position.x, position.y]) : stagePosition(position, style, surface);
    const pose = spritePose(art, style, at, parentPose?.angle);
    return { style, pose, pick: { kind: 'scenery', id: item.id }, parentPose, draft: Boolean(draft), bedSite: draft?.bedSite };
  }

  function flat(item: PondTerrainObject, draft = false): ComposedLayer {
    const profile = pondStructureProfile(item.preset);
    if (profile) {
      const base = bedPoint(item.x);
      const origin = atStage({ x: base.x, y: base.y - profile.bottom });
      const style: SceneryStyle = { type: `${item.preset}-front`, band: 6, shade: .95,
        opacity: draft ? .68 : 1, shadow: [profile.right - profile.left, profile.bottom > 0 ? 7 : 12, .34] };
      const poseFor = (type: string) => {
        const pose = spritePose(art, { ...style, type }, origin);
        const width = (profile.right - profile.left) * scale;
        const height = (profile.top - profile.bottom) * scale;
        return { ...pose, scale: width / pose.art.w, flat: (height * pose.art.w) / (width * pose.art.h) };
      };
      return { style, pose: poseFor(style.type), structureRear: poseFor(`${item.preset}-rear`), pick: item, draft };
    }
    const style = materialStyle(item);
    return { style, pose: physicalPose(art, style, item), pick: item, draft };
  }

  for (const item of terrain) {
    const draft = terrainDraft?.id === item.id && terrainDraft.kind === item.kind ? terrainDraft : undefined;
    roots.push(flat(draft ? { ...item, x: draft.x } : item, Boolean(draft)));
  }
  if (terrainDraft?.id === null) roots.push(flat({ ...terrainDraft, id: -1 }, true));
  for (const item of scenery) {
    if (item.parent) continue;
    const layer = decor(item);
    if (layer) roots.push(layer);
  }
  if (sceneryDraft?.id === null) {
    const layer = decor({ ...sceneryDraft, id: -1 });
    if (layer) roots.push({ ...layer, draft: true, bedSite: sceneryDraft.bedSite });
  }
  for (const patch of pond.waterPlantPatches ?? []) {
    const draft = waterPlantDraft?.id === patch.id ? waterPlantDraft : undefined;
    const x = draft?.x ?? patch.x;
    roots.push(patch.ribbonPreset ? visualRibbonPlant(art, pond, x, patch.ribbonPreset, patch, Boolean(draft))
      : visualWaterPlant(art, x, patch, Boolean(draft)));
  }
  if (waterPlantDraft?.id === null) {
    const { x, ribbonPreset } = waterPlantDraft;
    roots.push(ribbonPreset ? visualRibbonPlant(art, pond, x, ribbonPreset, undefined, true)
      : visualWaterPlant(art, x, undefined, true));
  }
  for (const deposit of pond.leafDeposits ?? []) {
    const moving = leafDraft?.id === deposit.id;
    roots.push(...leafDepositLayers(art, moving ? { ...deposit, x: leafDraft.x } : deposit, pond.wet, moving));
  }
  if (leafDraft?.id === null) {
    roots.push(...leafDepositLayers(art, { ...leafDraft, id: -1, stage: 'whole' }, pond.wet, true));
  }
  const hasSecondaryPlants = pond.staticSections?.some(({ waterPlantPatches }) => waterPlantPatches?.length);
  const unassigned = pond.unassignedPlantMgCnp?.[0] ?? (hasSecondaryPlants ? 0 : pond.plantMgC);
  if (pond.wet && unassigned > 0) {
    // This stock projection is separate from hand-placed vegetation in the preset.
    const growth = clamp(unassigned / 155, .12, 1);
    for (const [index, x] of [122, 442].entries()) {
      const style = { type: 'hornwort-100', band: 0, mm: 78 * growth + 19, opacity: .49 * growth, shade: .82, flip: index === 0 };
      roots.push({ style, pose: spritePose(art, style, { x, y: 251 }), pick: { kind: 'plant', id: -1 } });
    }
  }

  const result: ComposedLayer[] = [];
  function addWithChildren(layer: ComposedLayer) {
    result.push(layer);
    const { kind, id } = layer.pick;
    const children = attachments.get(kind + '/' + id) ?? [];
    const placed = children.map((item) => decor(item, layer.pose)).filter((item) => item !== undefined);
    placed.sort((a, b) => (a.style.attachBand ?? 0) - (b.style.attachBand ?? 0));
    for (const child of placed) addWithChildren(child);
    if (kind !== 'wood' && kind !== 'stone') return;
    for (const patch of pond.mossPatches ?? []) {
      if (patch.parent.kind !== kind || patch.parent.id !== id || patch.id === mossDraft?.visual?.id) continue;
      const original = terrain.find((item) => item.kind === kind && item.id === id);
      result.push(visualMoss(art, mossVisual(patch, original ? layer.pick.x - original.x : 0)));
    }
  }

  roots.sort((a, b) => a.style.band - b.style.band || a.pose.y - b.pose.y);
  for (const layer of roots) addWithChildren(layer);
  if (mossDraft?.visual) result.push(visualMoss(art, mossDraft.visual, true));
  return result;
}

export function composedPick(layers: readonly ComposedLayer[], point: Point, kind?: ScenePick['kind']) {
  for (let index = layers.length - 1; index >= 0; index--) {
    const layer = layers[index];
    if (!layer.draft && layer.pick.id > 0 && (kind === undefined || layer.pick.kind === kind)
      && (hitsSprite(layer.pose, layer.style, point) || (layer.structureRear && hitsSprite(layer.structureRear, layer.style, point)))) return layer;
  }
}

function smooth(a: number, b: number, x: number) {
  const t = clamp((x - a) / (b - a), 0, 1);
  return t * t * (3 - 2 * t);
}

function lighting(phase: number, seasonal: readonly number[]) {
  const elevation = Math.cos((phase - .5) * Math.PI * 2);
  const sun = smooth(-.14, .61, elevation);
  const ambient = smooth(-.83, .88, elevation);
  const twilight = 1 - smooth(.08, .67, Math.abs(elevation - .12));
  const dusk = phase > .5 ? [246, 175, 146] : [190, 209, 224];
  const night = [87, 111, 151];
  return { sun, tint: seasonal.map((target, i) => {
    const ambientColor = night[i] + (target - night[i]) * ambient;
    return Math.round(ambientColor + (dusk[i] - ambientColor) * (phase > .5 ? .58 : .37) * twilight);
  }) };
}

const gravel = [
  [69,292,4],[81,278,5],[112,279,9],[142,296,11],[162,298,7],
  [178,286,6],[197,290,4],[221,282,7],[245,287,10],[273,279,5],[287,270,4],
  [296,299,7],[317,259,6],[326,252,4],[327,279,11],[339,233,4],[348,252,7],
  [369,261,8],[386,265,11],[397,242,5],[411,263,7],[432,269,5],
  [180,310,5],[228,314,7],[249,313,5],[260,327,9],[286,309,4],[418,319,6],
  [460,318,9],[483,320,7],[501,275,6],[535,272,6],
] as const;

const residueSites = [
  [198, 257, 8], [282, 288, 9], [334, 238, 7], [457, 274, 10],
  [89, 296, 6], [390, 317, 9], [518, 309, 8], [306, 307, 6],
  [227, 275, 7], [472, 245, 7], [140, 317, 8], [372, 263, 5],
] as const;

function paintPondResidue(context: CanvasRenderingContext2D, art: ComposedArt, pond: PondBasinSnapshot, sun: number) {
  // Positioned leaves account for a named share of the same stock.
  const carbon = (deposits: PondBasinSnapshot['leafDeposits']) => deposits?.reduce((sum, leaf) => sum + leaf.remainingMgC, 0) ?? 0;
  const assigned = (pond.staticSections ?? []).reduce((sum, section) => sum + carbon(section.leafDeposits), carbon(pond.leafDeposits));
  const unassigned = Math.max(0, pond.detritusMgC - assigned);
  for (const [index, [x, y, mm]] of residueSites.entries()) {
    const amount = clamp((unassigned - index * 32) / 56, 0, 1);
    if (amount <= 0) break;
    const style = { type: 'leaf-55', band: 2, mm, flat: .64, angle: (index % 5 - 2) * .32,
      shade: .68 + (pond.wet ? sun * .12 : .18), saturation: .69, opacity: amount * .67, flip: index % 2 === 0 };
    drawScenerySprite(context, style, spritePose(art, style, { x, y }));
  }
}

function paintBedFilm(context: CanvasRenderingContext2D, mgC: number, sun: number) {
  const opacity = clamp(mgC / 65, 0, 1) * (.16 + sun * .12);
  if (opacity <= 0) return;
  // Bed contours show the portion left after the display weights have been assigned to submerged carriers.
  for (const [x, y, w, h, angle] of [[117, 279, 78, 8, -.13], [183, 314, 87, 12, .05],
    [286, 260, 109, 9, -.15], [401, 282, 117, 15, .13], [521, 304, 74, 11, -.16]]) {
    context.save();
    context.translate(x, y);
    context.rotate(angle);
    context.scale(w / 2, h / 2);
    const film = context.createRadialGradient(0, 0, .04, 0, 0, 1);
    film.addColorStop(0, 'rgba(119,137,66,' + opacity + ')');
    film.addColorStop(.46, 'rgba(103,122,61,' + opacity * .82 + ')');
    film.addColorStop(1, '#5a713200');
    context.fillStyle = film;
    context.fillRect(-1, -1, 2, 2);
    context.restore();
  }
}

function waterClip(context: CanvasRenderingContext2D, camera: Rect, surface: number) {
  const top = Math.max(camera.y, surface);
  context.beginPath();
  context.rect(camera.x, top, camera.width, Math.max(0, camera.y + camera.height - top));
  context.clip();
}

function drawWaterQuality(context: CanvasRenderingContext2D, pond: PondBasinSnapshot, camera: Rect, surface: number) {
  const algae = clamp(pond.algaeMgC / 150, 0, 1) * .20;
  const turbidity = clamp(pond.turbidityIndex, 0, 1) * .48;
  if (algae === 0 && turbidity === 0) return;
  context.save();
  waterClip(context, camera, surface);
  context.fillStyle = 'rgba(93,138,76,' + algae + ')';
  context.fillRect(camera.x, camera.y, camera.width, camera.height);
  context.fillStyle = 'rgba(89,107,92,' + turbidity + ')';
  context.fillRect(camera.x, camera.y, camera.width, camera.height);
  context.restore();
}

function drawGravel(context: CanvasRenderingContext2D, art: ComposedArt, sun: number) {
  for (const [index, [x, y, mm]] of gravel.entries()) {
    const style = { type: 'pebble-16', band: 2, mm, shade: .53 + sun * .14 + index % 4 * .09,
      angle: (index % 5 - 2) * .13, opacity: .91, flip: index % 2 !== 0 };
    const pose = spritePose(art, style, { x, y });
    drawShadow(context, pose, [mm * 1.9, mm * .4, .34]);
    drawScenerySprite(context, style, pose);
  }
}

function drawDaphnia(context: CanvasRenderingContext2D, pond: PondBasinSnapshot) {
  for (let index = 0; index < Math.min(32, Math.ceil(pond.daphniaCount / 6)); index++) {
    const p = atStage(pondRockShrimpPoint({ x: .08 + ((index * .61803 + .11) % 1) * .84,
      y: .14 + ((index * .41421 + .04) % 1) * .70 }, pond));
    context.fillStyle = '#b7d7a8a3';
    context.fillRect(Math.round(p.x), Math.round(p.y), 1.4, 1.4);
  }
}

type Shrimp = PondBasinSnapshot['shrimp'][number];
// Opaque upper edge of rock-low-98.png across the usable stretch of the main stone, in sprite UV.
const mainStoneRidge = [
  [.11936, .31127], [.21078, .24187], [.30257, .19568], [.39459, .16343], [.48692, .14974],
  [.57970, .16391], [.67271, .19201], [.76610, .24332], [.85987, .31785],
] as const;
const restingShrimpFoot = [.521, .688] as const;

function shrimpOnMainStone(shrimp: SpritePose, stone: SpritePose): SpritePose {
  let left = spritePoint(stone, mainStoneRidge[0]);
  for (let index = 1; index < mainStoneRidge.length; index += 1) {
    const right = spritePoint(stone, mainStoneRidge[index]);
    if (shrimp.x <= right.x || index === mainStoneRidge.length - 1) {
      const progress = clamp((shrimp.x - left.x) / (right.x - left.x), 0, 1);
      const angle = Math.atan2(right.y - left.y, right.x - left.x);
      const foot = spritePoint({ ...shrimp, angle }, restingShrimpFoot);
      return {
        ...shrimp, angle, x: left.x + progress * (right.x - left.x) + shrimp.x - foot.x,
        y: left.y + progress * (right.y - left.y) + shrimp.y - foot.y + .25,
      };
    }
    left = right;
  }
  return shrimp;
}

function drawShrimp(context: CanvasRenderingContext2D, art: ComposedArt, shrimp: Shrimp, at: Point, riding: boolean,
  stage?: PondShrimpStage, feeding?: { camera: Rect; waterline: number; source: 'wood' | 'stone' | 'shared' }, stone?: SpritePose,
) {
  const swim = !riding && (shrimp.activity === 'moving' || shrimp.activity === 'foraging');
  const style = { type: swim ? 'shrimp-swim-20' : 'shrimp-rest-20', band: 3,
    mm: stage === 'juvenile' ? 9 : 20, shade: riding ? .98 : .94, flip: Math.cos(shrimp.heading) < 0 };
  const rest = spritePose(art, style, at);
  const pose = stone ? shrimpOnMainStone(rest, stone) : rest;
  drawScenerySprite(context, style, pose);
  if (!feeding) return;

  const mouth = spritePoint(pose, [.678, .53]);
  const size = style.mm / 20;
  context.save();
  waterClip(context, feeding.camera, feeding.waterline);
  context.translate(mouth.x, mouth.y);
  if (stone) context.rotate(pose.angle);
  context.scale(pose.flip * size, size);
  const feedingColors = { wood: '#d8d59bc9', stone: '#b5e2d0d0', shared: '#e4c19ba8' };
  context.strokeStyle = feedingColors[feeding.source];
  context.lineWidth = 1;
  context.lineCap = 'round';
  context.beginPath();
  context.moveTo(.6, 0);
  context.quadraticCurveTo(3.3, 1.2, 2.4, 3.4);
  context.moveTo(2.4, -.1);
  context.quadraticCurveTo(5, .6, 4.7, 2.3);
  context.stroke();
  context.restore();
}

function drawMark(context: CanvasRenderingContext2D, item: ComposedLayer, hint?: PondStoneHint | PondMossPreview['site']) {
  const { x, y, width, height } = spriteBounds(item.pose, item.style);
  context.save();
  context.strokeStyle = item.bedSite === 'outside' ? '#fc987a' : hint && hint !== 'ground' ? '#ffcc94' : '#c0f0df';
  context.lineWidth = 1.3;
  if (item.draft) context.setLineDash([3, 3]);
  context.strokeRect(x - 2, y - 2, width + 4, height + 4);
  context.restore();
}

function drawBedScenery(context: CanvasRenderingContext2D, layers: readonly ComposedLayer[], submerged: boolean) {
  for (const { style, pose, draft, bedSite } of layers) {
    if (!style.bedAnchor || bedSite === 'outside') continue;
    drawScenerySprite(context, { ...style, shade: submerged ? .91 : 1.02, saturation: submerged ? .80 : .98,
      opacity: draft ? .67 : .94 }, pose);
  }
}

function exposeBed(context: CanvasRenderingContext2D, art: ComposedArt, view: Rect, water: number, wet: boolean) {
  const bottom = view.y + view.height;
  const upland = Math.min(wet ? water + 1 : bottom, bottom);
  if (upland <= view.y) return;
  context.save();
  context.beginPath();
  context.rect(view.x, view.y, view.width, upland - view.y);
  context.clip();
  const horizon = context.createLinearGradient(0, view.y, 0, Math.max(view.y + 1, wet ? water : 290));
  horizon.addColorStop(0, '#223a34');
  horizon.addColorStop(.70, '#3b4a3a');
  horizon.addColorStop(1, '#585447');
  context.fillStyle = horizon;
  context.fillRect(view.x, view.y, view.width, view.height);
  const bedTop = wet ? upland - 61 : 146;
  context.globalAlpha = wet ? .27 : .72;
  context.drawImage(art.bed, view.x, bedTop, view.width, Math.max(96, bottom - bedTop));
  const fade = context.createLinearGradient(0, bedTop, 0, bedTop + 46);
  fade.addColorStop(0, '#3b4a3a');
  fade.addColorStop(1, '#3b4a3a00');
  context.globalAlpha = 1;
  context.fillStyle = fade;
  context.fillRect(view.x, bedTop, view.width, 46);
  context.restore();
}

export function drawPondComposition(context: CanvasRenderingContext2D, art: ComposedArt, pond: PondBasinSnapshot,
  camera: Rect, daylightPhase: number, seasonal: readonly number[], layers: ComposedLayer[],
  selection?: ScenePick, hint?: PondStoneHint, mossHint?: PondMossPreview['site'], waterPlantHint?: PondStoneHint,
  floatingSelectionId?: number, sedgeSelectionId?: number) {
  const { sun, tint } = lighting(daylightPhase, seasonal);
  const surface = compositionSurface(pond);
  const floating = pondFloatingVisuals(pond, surface, atStage({ x: 0, y: bedY }).y);
  const sedges = compositionSedges(pond);
  const film = pondBiofilmDisplay(pond);
  const surfaceFilm = new Map(film.surfaces.map((share) => [share.kind + '/' + share.id, share]));
  const x = Math.min(0, camera.x);
  const w = Math.max(composedWidth, camera.x + camera.width) - x;
  context.fillStyle = '#093c3b';
  context.fillRect(camera.x, camera.y, camera.width, camera.height);
  context.imageSmoothingEnabled = true;
  context.imageSmoothingQuality = 'high';
  if (pond.wet) {
    context.save();
    context.translate(x, 0);
    context.scale(w / composedWidth, 1);
    paintPondBase(context, art, sun, camera.y + camera.height);
    context.restore();
    context.save();
    waterClip(context, camera, surface);
    drawBedScenery(context, layers, true);
    for (const item of layers) {
      if (item.style.band === 0) drawScenerySprite(context, item.style, item.pose);
    }
    const lightTransmission = pond.floatingCanopy?.lightTransmission ?? 1;
    paintPondLight(context, art, sun * (1 - .44 * clamp(pond.turbidityIndex, 0, 1)) * lightTransmission);
    paintFloatingShade(context, floating, sun);
    paintBedFilm(context, film.bedMgC, sun);
    drawDaphnia(context, pond);
    context.restore();
  }
  exposeBed(context, art, camera, surface, pond.wet);
  if (!pond.wet || surface > camera.y) {
    context.save();
    if (pond.wet) {
      context.beginPath();
      context.rect(camera.x, camera.y, camera.width, surface - camera.y);
      context.clip();
    }
    drawBedScenery(context, layers, false);
    context.restore();
  }
  if (pond.wet) {
    context.save();
    context.translate(x, surface - nominalSurface);
    context.scale(w / composedWidth, 1);
    paintPondSurface(context, art, sun);
    context.restore();
    paintFloatingRoots(context, art, floating);
  }
  drawGravel(context, art, sun);
  paintPondResidue(context, art, pond, sun);
  paintPondSedges(context, art, camera, sedges, pond.wet, surface, false);
  const woodCarriers = new Map(pond.shrimpAttachments?.map(({ shrimpId, woodId }) => [shrimpId, woodId]));
  const stoneContacts = currentPondStoneContacts(pond).filter(({ stoneId }) =>
    !layers.some(({ draft, pick }) => draft && pick.kind === 'stone' && pick.id === stoneId));
  const stoneCarriers = new Map(stoneContacts.map(({ shrimpId, stoneId }) => [shrimpId, stoneId]));
  const life = new Map(pond.shrimpLife?.map(({ id, stage }) => [id, stage]));
  const woodFeeding = new Map(currentPondWoodFeeding(pond).map(event => [event.shrimpId, event]));
  const stoneFeeding = new Map(currentPondStoneFeeding(pond, stoneContacts).map(event => [event.shrimpId, event]));
  const drawn = new Set<string>();
  const animals = pond.wet ? pond.shrimp.map((shrimp) => ({ shrimp,
    at: atStage(pondRockShrimpPoint(shrimp, pond)),
    woodCarrier: woodCarriers.get(shrimp.id), stoneCarrier: stoneCarriers.get(shrimp.id), stage: life.get(shrimp.id),
  })) : [];
  function swimUntil(y: number) {
    for (const animal of animals) {
      if (animal.woodCarrier !== undefined || animal.stoneCarrier !== undefined || drawn.has(animal.shrimp.id) || animal.at.y > y) continue;
      drawShrimp(context, art, animal.shrimp, animal.at, false, animal.stage);
      drawn.add(animal.shrimp.id);
    }
  }
  function drawStructure(pose: SpritePose, style: SceneryStyle) {
    if (pond.wet) {
      context.save();
      waterClip(context, camera, surface);
      drawScenerySprite(context, style, pose);
      context.restore();
    }
    if (!pond.wet || surface > camera.y) {
      context.save();
      if (pond.wet) {
        context.beginPath();
        context.rect(camera.x, camera.y, camera.width, surface - camera.y);
        context.clip();
      }
      drawScenerySprite(context, { ...style, shade: .81, saturation: .76 }, pose);
      context.restore();
    }
  }
  for (const { structureRear, style, pose } of layers) {
    if (!structureRear) continue;
    if (style.shadow) drawShadow(context, pose, style.shadow);
    drawStructure(structureRear, { ...style, shade: .78, saturation: .84 });
  }
  for (const item of layers) {
    const { pose, style, pick } = item;
    if (style.bedAnchor) continue;
    if (!style.band) {
      if (!pond.wet && pick.kind === 'scenery') drawScenerySprite(context, style, pose);
      continue;
    }
    if (style.band >= 2) swimUntil(style.band < 6 ? Math.min(229, pose.y - 20) : Infinity);
    if (style.shadow && !item.structureRear) drawShadow(context, pose, style.shadow);
    if (style.extraShadow) drawShadow(context, spritePoint(pose, style.extraShadow.local), style.extraShadow.size);
    if (style.rings) drawFloatRings(context, pose, sun);
    if (item.structureRear) {
      drawStructure(pose, style);
    } else if (pick.kind === 'waterPlant' && pond.wet && spriteBounds(pose, style).y < surface) {
      context.save();
      waterClip(context, camera, surface);
      drawScenerySprite(context, style, pose);
      context.restore();
      if (surface > camera.y) {
        context.save();
        context.beginPath();
        context.rect(camera.x, camera.y, camera.width, surface - camera.y);
        context.clip();
        drawScenerySprite(context, { ...style, shade: .70, saturation: .57 }, pose);
        context.restore();
      }
    } else drawScenerySprite(context, style, pose);
    const share = (pick.kind === 'stone' || pick.kind === 'wood') ? surfaceFilm.get(pick.kind + '/' + pick.id) : undefined;
    if (share) {
      context.save();
      waterClip(context, camera, surface);
      drawPondSurfaceFilm(context, style, pose, share, sun);
      context.restore();
    }
    if (pick.kind === 'wood') {
      const original = pond.woods?.find(({ id }) => id === pick.id);
      const dx = original ? (pick.x - original.x) * 288 * scale : 0;
      for (const animal of animals.filter(({ woodCarrier }) => woodCarrier === pick.id)) {
        const event = woodFeeding.get(animal.shrimp.id);
        drawShrimp(context, art, animal.shrimp, { x: animal.at.x + dx, y: animal.at.y }, true, animal.stage,
          event ? { camera, waterline: surface, source: (event.sources?.woodBiofilmMgC ?? 0) > 0 ? 'wood' : 'shared' } : undefined);
        drawn.add(animal.shrimp.id);
      }
    } else if (pick.kind === 'stone') {
      for (const animal of animals.filter(({ stoneCarrier }) => stoneCarrier === pick.id)) {
        const event = stoneFeeding.get(animal.shrimp.id);
        context.save();
        waterClip(context, camera, surface);
        drawShrimp(context, art, animal.shrimp, animal.at, true, animal.stage,
          event ? { camera, waterline: surface, source: event.stoneBiofilmMgC > 0 ? 'stone' : 'shared' } : undefined, pose);
        context.restore();
        drawn.add(animal.shrimp.id);
      }
    }
  }
  swimUntil(Infinity);
  paintPondSedges(context, art, camera, sedges, pond.wet, surface, true);
  if (pond.wet) drawWaterQuality(context, pond, camera, surface);
  paintFloatingBlades(context, art, floating, sun);
  context.save();
  context.globalCompositeOperation = 'multiply';
  context.fillStyle = 'rgb(' + tint.join(', ') + ')';
  context.fillRect(camera.x, camera.y, camera.width, camera.height);
  context.restore();
  for (const item of layers) {
    if (item.draft || (item.pick.kind === selection?.kind && item.pick.id === selection.id)) {
      const draftHint = item.pick.kind === 'plant' ? mossHint : item.pick.kind === 'waterPlant' ? waterPlantHint
        : item.pick.kind === 'leafDeposit' ? 'ground' : hint;
      drawMark(context, item, item.draft ? draftHint : undefined);
    }
  }
  markFloatingPlant(context, floating, floatingSelectionId);
  markPondSedge(context, sedges, sedgeSelectionId);
}
