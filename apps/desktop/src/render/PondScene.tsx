import { useEffect, useRef, useState } from 'react';
import type { PondBasinSnapshot } from '../world';
import shrimpRest from './assets/r5/pond/shrimp-rest.png';
import shrimpSwim from './assets/r5/pond/shrimp-swim.png';
import r5Art from './assets/r5/manifest.json';
import { type Point, type Rect, sampleProjection, sampleViewport } from './r5/geometry';
import { drawRuler, presentScene } from './r5/drawSample';
import type { R5ForestView } from './r5/ForestLiveScene';
import {
  type PondStoneArt, type PondStoneDraft, type PondStoneHint, type PondTerrainObject,
  bedLeft, bedPoint, bedWidth, bedY, drawPondTerrain, loadPondStoneArt, markPondStone, pickPondTerrain,
  pondRockShrimpPoint, pondTerrainObjects, stoneWaterHeight,
} from './r5/pondStoneGeometry';
import { type R5PondRockEdit, usePondRockInput } from './r5/pondStoneInput';
import {
  type PondMossArt, type PondMossPreview, drawPondMoss, loadPondMossArt, markPondMoss,
  mossFrame, mossVisual, pickPondMoss, previewPondMoss, sameMossParent,
} from './r5/pondMoss';
import { hitSprite } from './r5/sceneArt';
import { pondUnderwaterViewport } from './r5/pondFraming';
import {
  type PondLandscapeArt, drawPondLandscapeSurface, drawPondLandscapeWater,
  loadPondLandscapeArt, pondLandscapeLight,
} from './r5/pondLandscape';
import { seasonColor } from './seasonColor';
import './pond.css';

interface PondSceneProps {
  pond: PondBasinSnapshot;
  daylightPhase: number;
  seasonDay?: number;
  active: boolean;
  view?: R5ForestView;
  framing?: 'underwater';
  rockEdit?: R5PondRockEdit;
  editRevision?: string;
  onReady?: (backend: string) => void;
}

type SceneState = Pick<PondSceneProps, 'pond' | 'daylightPhase' | 'seasonDay' | 'view'> & {
  viewport: Rect;
  preview: PondStoneDraft | null;
  hint?: PondStoneHint;
  selected?: PondTerrainObject;
  selectedPlant?: number;
  mossPreview?: PondMossPreview;
};
const width = 480;
const height = 270;
const floor = 242;
const highWater = 34;
const assets = ['far', 'ground', 'gravel', 'wood', 'shore', 'reeds', 'biofilm', 'light',
  'hornwort-0', 'hornwort-1', 'shrimp-rest', 'shrimp-swim'] as const;
type PondArt = Record<typeof assets[number], HTMLImageElement>;

const { lengthMm: shrimpLengthMm, body: shrimpAxis, anchor: shrimpAnchor } = r5Art.pond.objects.shrimp;
const shrimpScale = sampleProjection(shrimpLengthMm, r5Art.pond.referenceWidthMm, width)
  / Math.hypot(shrimpAxis[1][0] - shrimpAxis[0][0], shrimpAxis[1][1] - shrimpAxis[0][1]);

const assetUrls = import.meta.glob<string>('./assets/pond/v1/*.png', {
  eager: true,
  query: '?url',
  import: 'default',
});
let decodedArt: Promise<PondArt> | undefined;

function loadArt() {
  decodedArt ??= Promise.all(assets.map(async (name) => {
    const url = name === 'shrimp-rest' ? shrimpRest : name === 'shrimp-swim' ? shrimpSwim
      : assetUrls['./assets/pond/v1/' + name + '.png'];
    if (!url) throw new Error('Missing pond art: ' + name);
    const image = new Image();
    image.src = url;
    await image.decode();
    return [name, image] as const;
  })).then((images) => Object.fromEntries(images) as PondArt).catch((error) => {
    decodedArt = undefined;
    throw error;
  });
  return decodedArt;
}

const margin = [
  { y: highWater, left: 23, right: 463 },
  { y: 85, left: 25, right: 457 },
  { y: 148, left: 38, right: 448 },
  { y: 205, left: 58, right: 432 },
  { y: floor, left: 93, right: 402 },
] as const;

const plantGroups = [
  { x: 105, y: 220, width: 78, threshold: 0, frame: 1 },
  { x: 359, y: 225, width: 109, threshold: 18, frame: 0 },
  { x: 409, y: 209, width: 74, threshold: 56, frame: 1 },
  { x: 127, y: 207, width: 73, threshold: 96, frame: 0 },
  { x: 325, y: 226, width: 79, threshold: 155, frame: 1 },
  { x: 162, y: 232, width: 65, threshold: 245, frame: 0 },
] as const;

const scraps = [
  [185, 218, 10], [231, 234, 12], [286, 222, 11], [259, 219, 6],
  [198, 241, 15], [138, 230, 9], [396, 223, 8], [322, 243, 13],
] as const;

function clamp(value: number) {
  return Math.min(1, Math.max(0, value));
}

function waterSurface(pond: PondBasinSnapshot) {
  if (!pond.wet) return floor;
  if (Array.isArray(pond.stones)) return bedY - stoneWaterHeight(pond);
  const fraction = pond.maxDepthCm > 0 ? clamp(pond.depthCm / pond.maxDepthCm) : 0;
  return floor - Math.max(1, Math.round((floor - highWater) * fraction));
}

function daphniaPoints(pond: PondBasinSnapshot) {
  return pond.wet ? Math.min(32, Math.ceil(pond.daphniaCount / 6)) : 0;
}

function sidesAt(y: number) {
  const after = margin.findIndex((breakpoint) => y <= breakpoint.y);
  if (after === 0) return margin[0];
  if (after < 0) return margin[margin.length - 1];
  const low = margin[after - 1];
  const high = margin[after];
  const t = (y - low.y) / (high.y - low.y);
  return { left: low.left + t * (high.left - low.left), right: low.right + t * (high.right - low.right) };
}

function aquaticPoint(x: number, y: number, surface: number) {
  const depth = floor - surface;
  const padding = Math.min(12, depth * .19);
  const visualY = Math.round(surface + padding + clamp(y) * (depth - padding * 2));
  const edges = sidesAt(visualY);
  return {
    x: Math.round(edges.left + 20 + clamp(x) * (edges.right - edges.left - 40)),
    y: visualY,
  };
}

function shrimpPoint(shrimp: Point, pond: PondBasinSnapshot, surface: number) {
  return Array.isArray(pond.stones) ? pondRockShrimpPoint(shrimp, pond) : aquaticPoint(shrimp.x, shrimp.y, surface);
}

function pondViewport(pond: PondBasinSnapshot, view?: R5ForestView, edit?: R5PondRockEdit) {
  const firstShrimp = pond.shrimp[0];
  const objects = pondTerrainObjects(pond);
  const anchor = objects.find(({ id, kind }) => id === edit?.selectedId && kind === (edit.selectedKind ?? 'stone')) ?? objects[0];
  const patch = edit?.selectedKind === 'plant' ? pond.mossPatches?.find(({ id }) => id === edit.selectedId) : undefined;
  const focus = view?.focus === 'animal' && firstShrimp ? shrimpPoint(firstShrimp, pond, waterSurface(pond))
    : view?.focus === 'contact' ? patch ? mossVisual(patch).point
      : anchor ? { x: bedPoint(anchor.x).x, y: bedY - 12 } : { x: 250, y: 154 }
      : { x: width / 2, y: height / 2 };
  return sampleViewport(edit?.mode === 'add' ? 1 : view?.zoom ?? 1, focus);
}

function waterEdges(surface: number, foreground: number) {
  const { left, right } = sidesAt(surface);
  const points: { y: number; left: number; right: number }[] = [{ y: surface, left, right }];
  for (const point of margin) {
    if (point.y > surface) points.push(point);
  }
  if (foreground > 0) {
    const bottom = margin[margin.length - 1];
    points.push({ y: floor + foreground, left: bottom.left - foreground * .65, right: bottom.right + foreground * .65 });
  }
  return points;
}

function clipWater(context: CanvasRenderingContext2D, surface: number, foreground = 0) {
  const points = waterEdges(surface, foreground);
  context.beginPath();
  context.moveTo(points[0].left, surface);
  for (const point of points) context.lineTo(point.right, point.y);
  for (let index = points.length - 1; index >= 0; index -= 1) {
    context.lineTo(points[index].left, points[index].y);
  }
  context.closePath();
  context.clip();
}

function blendLandscapeBanks(
  context: CanvasRenderingContext2D, dry: HTMLImageElement, surface: number, foreground: number,
) {
  const points = waterEdges(surface, foreground);
  for (const [span, opacity] of [[14, .26], [6, .55]] as const) {
    context.save();
    context.beginPath();
    for (const [side, direction] of [['left', 1], ['right', -1]] as const) {
      context.moveTo(points[0][side], surface);
      for (const point of points) context.lineTo(point[side], point.y);
      for (let index = points.length - 1; index >= 0; index -= 1) {
        const point = points[index];
        context.lineTo(point[side] + span * direction, point.y);
      }
      context.closePath();
    }
    context.clip();
    context.globalAlpha = opacity;
    context.drawImage(dry, 0, 0, width, height);
    context.restore();
  }
}

function drawScraps(context: CanvasRenderingContext2D, amount: number, leaf?: HTMLImageElement) {
  for (const [index, [x, y, span]] of scraps.entries()) {
    if (amount <= index * 15) break;
    context.globalAlpha = clamp((amount - index * 15) / 20);
    if (leaf) {
      const size = span * 1.65;
      context.drawImage(leaf, x - size / 2, y - size * .26, size, size * .39);
      continue;
    }
    context.fillStyle = index % 3 === 0 ? '#7b5e45' : '#948059';
    context.beginPath();
    context.moveTo(x - span / 2, y + 1);
    context.lineTo(x + 1, y - 4);
    context.lineTo(x + span / 2, y - 2);
    context.lineTo(x + 4, y + 2);
    context.lineTo(x - 2, y + 3);
    context.fill();
    context.fillStyle = '#bca477';
    context.fillRect(x - 2, y - 1, span * .45, 1);
  }
  context.globalAlpha = 1;
}

function drawPlants(context: CanvasRenderingContext2D, art: PondArt, amount: number, landscape?: PondLandscapeArt) {
  for (const group of plantGroups) {
    const growth = clamp((amount - group.threshold) / 55);
    if (!growth) continue;
    const cell = landscape?.hornwort ?? art[group.frame === 0 ? 'hornwort-0' : 'hornwort-1'];
    const w = Math.round(group.width * (.57 + growth * .43));
    const h = Math.round(w * cell.naturalHeight / cell.naturalWidth);
    context.globalAlpha = landscape ? .20 + growth * .46 : .28 + growth * .62;
    context.drawImage(cell, Math.round(group.x - w / 2), group.y - h, w, h);
  }
  context.globalAlpha = 1;
}

function drawDaphnia(context: CanvasRenderingContext2D, pond: PondBasinSnapshot, surface: number, seconds: number) {
  for (let index = 0; index < daphniaPoints(pond); index += 1) {
    // This is a uniformly dispersed visual sample of one population, not
    // per-organism state or a spatial concentration of any water chemistry.
    const x = .08 + ((index * .61803 + .11) % 1) * .84;
    const y = .14 + ((index * .41421 + .04) % 1) * .70;
    const p = aquaticPoint(x + Math.sin(seconds * .9 + index) * .007, y, surface);
    context.fillStyle = '#c3d8a3b9';
    context.fillRect(p.x, p.y, 2, 2);
    context.fillStyle = '#7f9c84';
    context.fillRect(p.x + 1, p.y + 1, 1, 2);
  }
}

function drawShrimp(
  context: CanvasRenderingContext2D, pond: PondBasinSnapshot, art: PondArt, surface: number,
  animals: readonly PondBasinSnapshot['shrimp'][number][] = pond.shrimp, attached = false,
) {
  const bottomToFront = [...animals].sort((left, right) => left.y - right.y);
  for (const shrimp of bottomToFront) {
    const p = shrimpPoint(shrimp, pond, surface);
    const swimming = !attached && (shrimp.activity === 'moving' || shrimp.activity === 'foraging');
    const image = swimming ? art['shrimp-swim'] : art['shrimp-rest'];
    const facing = Math.cos(shrimp.heading) >= 0 ? 1 : -1;
    context.save();
    context.translate(p.x, p.y);
    context.scale(shrimpScale * facing, shrimpScale);
    context.drawImage(image, -shrimpAnchor[0], -shrimpAnchor[1]);
    context.restore();
  }
}

function drawWaterActors(
  output: CanvasRenderingContext2D, scratch: CanvasRenderingContext2D, art: PondArt, terrain: PondStoneArt,
  pond: PondBasinSnapshot, items: readonly PondTerrainObject[], surface: number, seconds: number,
  mossArt?: PondMossArt, moss: readonly ReturnType<typeof mossVisual>[] = [],
) {
  const woodIds = new Set(items.filter(({ kind }) => kind === 'wood').map(({ id }) => id));
  const attachedTo = new Map(pond.shrimpAttachments?.filter(({ woodId }) => woodIds.has(woodId))
    .map(({ shrimpId, woodId }) => [shrimpId, woodId]));
  const free: PondBasinSnapshot['shrimp'] = [];
  const riders = new Map<number, PondBasinSnapshot['shrimp']>();
  for (const shrimp of pond.shrimp) {
    const woodId = attachedTo.get(shrimp.id);
    if (woodId === undefined) free.push(shrimp);
    else {
      const group = riders.get(woodId) ?? [];
      group.push(shrimp);
      riders.set(woodId, group);
    }
  }

  function composite(animals: PondBasinSnapshot['shrimp'], onWood?: number) {
    scratch.clearRect(0, 0, width, height);
    if (onWood === undefined) drawDaphnia(scratch, pond, surface, seconds);
    drawShrimp(scratch, pond, art, surface, animals, onWood !== undefined);
    scratch.globalCompositeOperation = 'destination-out';
    for (const item of items) {
      drawPondTerrain(scratch, terrain, item, item.kind === 'wood' && item.id === onWood ? 'front' : 'mask');
    }
    if (mossArt) for (const patch of moss) drawPondMoss(scratch, mossArt, patch);
    scratch.globalCompositeOperation = 'source-over';
    output.drawImage(scratch.canvas, 0, 0, width, height);
  }

  composite(free);
  for (const [woodId, group] of riders) composite(group, woodId);
}

function drawRipples(context: CanvasRenderingContext2D, surface: number, daylight: number, seconds: number) {
  const bands = [
    [[61, 88], [107, 142], [173, 222], [244, 308], [331, 369], [397, 428]],
    [[80, 104], [161, 192], [205, 222], [262, 294], [310, 360], [425, 443]],
    [[117, 133], [178, 215], [247, 287], [315, 340], [366, 385]],
    [[98, 116], [219, 243], [274, 301], [319, 350]],
  ];
  context.globalAlpha = .31 + .56 * daylight;
  for (const [row, segments] of bands.entries()) {
    const y = surface + row * row + row * 2;
    context.fillStyle = row % 2 ? '#8cc2b2' : row === 0 ? '#b9e0c8' : '#94bdb0';
    for (const [index, [left, right]] of segments.entries()) {
      const shift = Math.round(Math.sin(seconds * 1.3 + index + row * .9) * (1 + row));
      context.fillRect(left + shift, y, right - left, 1);
      if ((index + row) % 3 === 1) context.fillRect(left + shift + 3, y + 1, Math.round((right - left) * .37), 1);
    }
  }
  context.globalAlpha = 1;
}

function drawFrame(
  context: CanvasRenderingContext2D, art: PondArt, state: SceneState, seconds: number,
  terrain?: PondStoneArt, actorLayer?: CanvasRenderingContext2D, mossArt?: PondMossArt, landscape?: PondLandscapeArt,
) {
  const { pond, daylightPhase, seasonDay } = state;
  const daylight = clamp((Math.cos((daylightPhase - .5) * 2 * Math.PI) + .17) / .68);
  const season = seasonColor(seasonDay, 'light');
  const [red, green, blue] = season;
  const tintR = Math.round(113 + (red - 113) * daylight);
  const tintG = Math.round(147 + (green - 147) * daylight);
  const tintB = Math.round(196 + (blue - 196) * daylight);
  const light = landscape ? pondLandscapeLight(daylightPhase, season) : undefined;
  const surface = waterSurface(pond);
  if (landscape && light) {
    context.drawImage(landscape.dry, 0, 0, width, height);
    if (pond.wet) {
      // A deep view can show the submerged foreground in front of the editable bed.
      // Thin standing water stops at the physical bed instead of covering the frame.
      const foreground = Math.min(height - floor, Math.max(0, (floor - surface - 26) * .32));
      context.save();
      clipWater(context, surface, foreground);
      drawPondLandscapeWater(context, landscape, surface, light.direct);
      context.restore();
      blendLandscapeBanks(context, landscape.dry, surface, foreground);
      context.save();
      clipWater(context, surface);
      drawPlants(context, art, pond.plantMgC, landscape);
      context.restore();
    }
  } else {
    context.drawImage(art.far, 0, 0);
    context.drawImage(art.ground, 0, 0);
    context.drawImage(terrain?.bedGrit ?? art.gravel, 0, 0);
    if (!terrain?.wood) context.drawImage(art.wood, 0, 0);
  }
  context.globalAlpha = clamp(pond.biofilmMgC / 28);
  if (context.globalAlpha > 0) context.drawImage(terrain?.wood?.bedBiofilm ?? art.biofilm, 0, 0);
  context.globalAlpha = 1;
  drawScraps(context, pond.detritusMgC, landscape?.leaf);
  const objects = pondTerrainObjects(pond);
  const { preview } = state;
  const originalCarrier = preview && state.hint === 'ground' && preview.id !== null
    ? objects.find(({ kind, id }) => kind === preview.kind && id === preview.id) : undefined;
  const originalWood = originalCarrier?.kind === 'wood' ? originalCarrier : undefined;
  const patches = pond.mossPatches ?? [];
  const pendingPatch = preview && patches.find((patch) => patch.parent.kind === preview.kind && patch.parent.id === preview.id);
  const movedPatch = originalCarrier && pendingPatch ? pendingPatch : undefined;
  const moss = patches.filter((patch) => patch !== movedPatch).map((patch) => mossVisual(patch));
  const echoIds = new Set(originalWood && pond.wet ? pond.shrimpAttachments
    ?.filter(({ woodId }) => woodId === originalWood.id).map(({ shrimpId }) => shrimpId) : undefined);
  const underlyingPond = echoIds.size ? { ...pond, shrimp: pond.shrimp.filter(({ id }) => !echoIds.has(id)) } : pond;
  const keepOriginal = (preview?.kind === 'wood' || Boolean(pendingPatch)) && state.hint !== 'ground';
  const solids = keepOriginal ? objects : objects.filter(({ id, kind }) => kind !== preview?.kind || id !== preview.id);
  if (terrain) {
    for (const item of solids) drawPondTerrain(context, terrain, item, 'shadow');
    for (const item of solids) drawPondTerrain(context, terrain, item);
  }
  if (mossArt) for (const patch of moss) drawPondMoss(context, mossArt, patch);

  if (pond.wet) {
    context.save();
    clipWater(context, surface);
    // Optical layers use the same color across the basin width. Depth and light
    // set artistic contrast, while turbidity is a single basin-wide index.
    const depth = floor - surface;
    context.fillStyle = landscape ? '#337e8f1c' : '#337e8f70';
    context.fillRect(0, surface, width, depth);
    if (!landscape) {
      context.fillStyle = '#5aafa23c';
      context.fillRect(0, surface, width, Math.ceil(depth * .44));
      context.fillStyle = '#24587328';
      context.fillRect(0, surface + Math.floor(depth * .71), width, depth);
      drawPlants(context, art, pond.plantMgC);
      context.globalAlpha = daylight * .94;
      context.drawImage(art.light, 0, 0);
      context.globalAlpha = 1;
    }
    if (terrain && actorLayer && solids.length) {
      drawWaterActors(context, actorLayer, art, terrain, underlyingPond, solids, surface, seconds, mossArt, moss);
    } else {
      drawDaphnia(context, pond, surface, seconds);
      drawShrimp(context, underlyingPond, art, surface);
    }
    const algaeTint = clamp(pond.algaeMgC / 150) * .19;
    const siltTint = clamp(pond.turbidityIndex) * .64;
    context.fillStyle = 'rgba(91, 143, 92, ' + algaeTint + ')';
    context.fillRect(0, surface, width, depth);
    context.fillStyle = 'rgba(65, 98, 79, ' + siltTint + ')';
    context.fillRect(0, surface, width, depth);
    if (light) drawPondLandscapeSurface(context, surface, seconds, light.direct);
    else drawRipples(context, surface, daylight, seconds);
    context.restore();
  }

  if (!landscape) {
    context.drawImage(art.shore, 0, 0);
    context.drawImage(art.reeds, 0, 0);
  }
  context.globalCompositeOperation = 'multiply';
  context.fillStyle = light?.tint ?? 'rgb(' + tintR + ', ' + tintG + ', ' + tintB + ')';
  context.fillRect(0, 0, width, height);
  context.globalCompositeOperation = 'source-over';
  if (terrain) {
    const selected = solids.find(({ id, kind }) => kind === state.selected?.kind && id === state.selected.id);
    if (selected) markPondStone(context, selected);
    if (preview) {
      context.save();
      context.globalAlpha = .68;
      if (originalWood && echoIds.size) {
        // Local drag preview only. The current shrimp coordinates remain authoritative.
        const dx = preview.x - originalWood.x;
        const riders = pond.shrimp.filter(({ id }) => echoIds.has(id)).map((shrimp) => ({ ...shrimp, x: shrimp.x + dx }));
        drawPondTerrain(context, terrain, preview, 'behind');
        context.save();
        const surface = waterSurface(pond);
        clipWater(context, surface);
        drawShrimp(context, pond, art, surface, riders, true);
        context.restore();
        drawPondTerrain(context, terrain, preview, 'front');
      } else drawPondTerrain(context, terrain, preview);
      if (mossArt && originalCarrier && movedPatch) {
        drawPondMoss(context, mossArt, mossVisual(movedPatch, preview.x - originalCarrier.x));
      }
      context.restore();
      markPondStone(context, preview, state.hint);
    }
  }
  if (mossArt) {
    const selected = moss.find(({ id }) => id === state.selectedPlant);
    if (selected) markPondMoss(context, selected);
    const draft = state.mossPreview?.visual;
    if (draft) {
      context.save();
      context.globalAlpha = .68;
      drawPondMoss(context, mossArt, draft);
      context.restore();
      markPondMoss(context, draft, state.mossPreview?.site);
    }
  }
}

interface PondControls {
  update(state: SceneState, active: boolean): void;
  pick(at: Point): ReturnType<typeof pickPondTerrain> | ReturnType<typeof pickPondMoss>;
  destroy(): void;
}

function mountPond(
  host: HTMLElement, canvas: HTMLCanvasElement, art: PondArt, terrain?: PondStoneArt, moss?: PondMossArt,
  landscape?: PondLandscapeArt,
): PondControls {
  const rasterScale = landscape ? 2 : 1;
  const logical = document.createElement('canvas');
  logical.width = width * rasterScale;
  logical.height = height * rasterScale;
  const buffer = logical.getContext('2d', { alpha: false });
  const screen = canvas.getContext('2d', { alpha: false });
  if (!buffer || !screen) throw new Error('Canvas 2D unavailable');
  buffer.scale(rasterScale, rasterScale);
  buffer.imageSmoothingEnabled = false;
  const actorLayer = terrain ? document.createElement('canvas').getContext('2d') : undefined;
  if (actorLayer) {
    actorLayer.canvas.width = width * rasterScale;
    actorLayer.canvas.height = height * rasterScale;
    actorLayer.scale(rasterScale, rasterScale);
    actorLayer.imageSmoothingEnabled = false;
  }
  let state: SceneState | undefined;
  let active = false;
  let lastFrame = 0;
  let request = 0;
  let backingRatio = 0;
  const motion = matchMedia('(prefers-reduced-motion: reduce)');

  function render(now: number) {
    if (!active || !state || !canvas.width || !canvas.height) return;
    if (backingRatio !== (window.devicePixelRatio || 1)) sizeCanvas();
    lastFrame = now;
    drawFrame(buffer!, art, state, motion.matches ? 0 : now / 1000, terrain, actorLayer ?? undefined, moss, landscape);
    const { viewport } = state;
    presentScene(canvas, logical, viewport);
    if (state.view?.showScale) drawRuler(screen!, viewport, canvas, r5Art.pond.referenceWidthMm, backingRatio);
  }

  function tick(now: number) {
    request = 0;
    if (!active || !state?.pond.wet || motion.matches) return;
    if (now - lastFrame >= 80) render(now);
    request = requestAnimationFrame(tick);
  }

  function syncLoop() {
    if (request) cancelAnimationFrame(request);
    request = 0;
    if (active && state?.pond.wet && !motion.matches) request = requestAnimationFrame(tick);
  }

  function sizeCanvas() {
    const dpr = window.devicePixelRatio || 1;
    const scale = Math.min(host.clientWidth / width, host.clientHeight / height);
    const physicalWidth = Math.round(width * scale * dpr);
    const physicalHeight = Math.round(height * scale * dpr);
    if (physicalWidth <= 0 || physicalHeight <= 0) return;
    backingRatio = dpr;
    if (canvas.width !== physicalWidth) canvas.width = physicalWidth;
    if (canvas.height !== physicalHeight) canvas.height = physicalHeight;
    canvas.style.width = physicalWidth / dpr + 'px';
    canvas.style.height = physicalHeight / dpr + 'px';
  }

  function resize() {
    sizeCanvas();
    render(performance.now());
  }

  const observer = new ResizeObserver(resize);
  observer.observe(host);
  let resolution: MediaQueryList | undefined;
  function watchResolution() {
    resolution?.removeEventListener('change', watchResolution);
    resolution = matchMedia('(resolution: ' + window.devicePixelRatio + 'dppx)');
    resolution.addEventListener('change', watchResolution);
    resize();
  }
  function motionChanged() {
    render(performance.now());
    syncLoop();
  }
  motion.addEventListener('change', motionChanged);
  window.addEventListener('resize', resize);
  watchResolution();

  return {
    update(next, shown) {
      const loopChanged = shown !== active || next.pond.wet !== state?.pond.wet;
      state = next;
      active = shown;
      render(performance.now());
      if (loopChanged) syncLoop();
    },
    pick(at) {
      if (!terrain || !state) return;
      const x = Math.floor(at.x);
      const y = Math.floor(at.y);
      if (!landscape && (hitSprite(art.shore, x, y) || hitSprite(art.reeds, x, y))) return;
      const plant = moss && pickPondMoss(state.pond, at, moss);
      if (plant) return plant;
      return pickPondTerrain(pondTerrainObjects(state.pond), at, terrain);
    },
    destroy() {
      if (request) cancelAnimationFrame(request);
      motion.removeEventListener('change', motionChanged);
      window.removeEventListener('resize', resize);
      resolution?.removeEventListener('change', watchResolution);
      observer.disconnect();
    },
  };
}

export function PondScene({ pond, daylightPhase, seasonDay, active, view, framing, rockEdit, editRevision = '', onReady }: PondSceneProps) {
  const host = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const controls = useRef<PondControls | null>(null);
  const [status, setStatus] = useState<'loading' | 'ready' | 'error'>('loading');
  const hasStones = Array.isArray(pond.stones);
  const hasWoods = Array.isArray(pond.woods);
  const hasMoss = Array.isArray(pond.mossPatches);
  const edit = hasStones && active && status === 'ready' ? rockEdit : undefined;
  const viewport = framing === 'underwater' && hasMoss && !edit
    ? pondUnderwaterViewport(waterSurface(pond)) : pondViewport(pond, view, edit);
  const input = usePondRockInput({ pond, edit, revision: editRevision, canvas, view: viewport,
    pick: (at) => controls.current?.pick(at) });
  const display = input.latched ?? viewport;
  const mossPreview = hasMoss ? previewPondMoss(pond, edit?.mossDraft) : undefined;
  const selectedPlant = edit?.selectedKind === 'plant' ? edit.selectedId ?? undefined : undefined;
  const sceneState = { pond, daylightPhase, seasonDay, view, viewport: display, preview: input.preview,
    hint: input.hint, selected: input.selected, selectedPlant, mossPreview };
  const latest = useRef({ state: sceneState, active, onReady });

  useEffect(() => {
    latest.current = { state: sceneState, active, onReady };
    controls.current?.update(sceneState, active);
  });

  useEffect(() => {
    const container = host.current;
    const panel = canvas.current;
    if (!container || !panel) return;
    let closed = false;
    let mounted: PondControls | undefined;
    void Promise.all([loadArt(), hasStones ? loadPondStoneArt(hasWoods, hasMoss) : undefined,
      hasMoss ? loadPondMossArt() : undefined, hasMoss ? loadPondLandscapeArt() : undefined])
      .then(([art, terrain, moss, landscape]) => {
        if (closed) return;
        mounted = mountPond(container, panel, art, terrain, moss, landscape);
        controls.current = mounted;
        mounted.update(latest.current.state, latest.current.active);
        latest.current.onReady?.('canvas2d');
        setStatus('ready');
      }).catch(() => {
        if (!closed) {
          latest.current.onReady?.('unavailable');
          setStatus('error');
        }
      });
    return () => {
      closed = true;
      mounted?.destroy();
      if (controls.current === mounted) controls.current = null;
    };
  }, [hasStones, hasWoods, hasMoss]);

  const label = pond.label + (pond.wet ? '，虾' + pond.shrimp.length + '只。水蚤采用抽样显示。' : '，水域已干涸。');
  const actions = ' 可拖动底床物件，左右键预览，回车确认。' + (hasMoss ? '可点选附着斑块查看。' : '');
  return (
    <div ref={host} className="pond-scene" data-pond-id={pond.id} data-wet={pond.wet}
      data-waterline={waterSurface(pond)} data-daphnia-points={daphniaPoints(pond)}
      data-zoom={edit?.mode === 'add' ? 1 : view?.zoom ?? 1}>
      <canvas ref={canvas} role={edit ? undefined : 'img'} aria-label={edit ? label + actions : label}
        tabIndex={edit ? 0 : undefined} className="pond-scene__canvas"
        data-pond-rock-live={hasStones || undefined}
        data-pond-wood-live={hasWoods || undefined}
        data-pond-plant-live={hasMoss || undefined}
        data-pond-landscape={hasMoss || undefined}
        data-ready={hasStones && status === 'ready' || undefined}
        data-viewport={hasStones ? `${display.x},${display.y},${display.width},${display.height}` : undefined}
        data-placement-mode={input.mode}
        data-placement-preset={input.preset}
        data-placement-kind={input.addKind}
        data-dragging={input.dragging || undefined}
        data-preview-site={input.hint}
        data-bed-y={hasStones ? bedY : undefined} data-bed-left={hasStones ? bedLeft : undefined} data-bed-width={hasStones ? bedWidth : undefined}
        data-pond-rock-ids={pond.stones?.map(({ id }) => id).join(',')}
        data-pond-rock-anchors={pond.stones?.map(({ id, x }) => `${id}:${bedPoint(x).x},${bedY}`).join(';')}
        data-selected-pond-rock-id={input.selected?.kind === 'stone' ? input.selected.id : undefined}
        data-pond-wood-ids={pond.woods?.map(({ id }) => id).join(',')}
        data-pond-wood-anchors={pond.woods?.map(({ id, x }) => `${id}:${bedPoint(x).x},${bedY}`).join(';')}
        data-selected-pond-wood-id={input.selected?.kind === 'wood' ? input.selected.id : undefined}
        data-selected-pond-kind={selectedPlant !== undefined ? 'plant' : input.selected?.kind}
        data-pond-attachment-ids={pond.shrimpAttachments?.map(({ shrimpId, woodId }) => `${shrimpId}:${woodId}`).join(',')}
        data-pond-plant-ids={pond.mossPatches?.map(({ id }) => id).join(',')}
        data-pond-plant-anchors={pond.mossPatches?.map((patch) => {
          const { x, y } = mossVisual(patch).point;
          return `${patch.id}:${x},${y}`;
        }).join(';')}
        data-pond-plant-frames={pond.mossPatches?.map((patch) => `${patch.id}:${mossFrame(patch)}`).join(',')}
        data-selected-pond-plant-id={selectedPlant}
        data-pond-plant-draft-site={mossPreview?.site}
        data-pond-plant-draft-anchor={mossPreview?.visual ? `${mossPreview.visual.point.x},${mossPreview.visual.point.y}` : undefined}
        {...input.handlers}
      />
      {status !== 'ready' && <p className="pond-scene__status" role={status === 'error' ? 'alert' : 'status'}>
        {status === 'error' ? '浅塘画面暂时无法显示' : '正在打开浅塘画面'}
      </p>}
    </div>
  );
}
