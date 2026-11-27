import { type PointerEvent, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { PondBasinSnapshot } from '../world';
import { seasonColor } from './seasonColor';
import { fitSample, R5_WIDTH } from './r5/geometry';
import { drawRuler } from './r5/drawRuler';
import type { R5ForestView } from './r5/forestView';
import { loadComposedArt, composedPixelsPerMm, type ComposedArt } from './r5/pondComposedArt';
import {
  atStage, cameraAtStage, composedLayers, composedPick, compositionSedges, compositionSurface, drawPondComposition,
  nominalSurface, pondCompositionCamera, stageView, type ComposedLayer, type PondLeafDraft,
} from './r5/pondComposition';
import { bedLeft, bedPoint, bedWidth, bedY, pondRockShrimpPoint, pondStockSite, pondTerrainObjects, stoneWaterHeight } from './r5/pondStoneGeometry';
import { pondStructureProfile } from './r5/pondStructureProfiles';
import { usePondRockInput, type R5PondRockEdit } from './r5/pondStoneInput';
import { useR5SceneryInput, type R5SceneryEdit } from './r5/pondSceneryInput';
import { mossVisual, previewPondMoss } from './r5/pondMoss';
import { sameComposedFrame } from './r5/composedFrame';
import { usePondWaterPlantInput, type R5PondWaterPlantEdit } from './r5/pondWaterPlantInput';
import { waterPlantFrame } from './r5/pondWaterPlant';
import { drawStockMarker, useStockPosition, type StockPositionTool } from './r5/stockPosition';
import { drawAnimalMarker } from './r5/animalMarker';
import { pointerOnSample } from './r5/geometry';
import { currentPondWoodFeeding } from './r5/pondWoodFeeding';
import { currentPondStoneContacts, currentPondStoneFeeding } from './r5/pondStoneFeeding';
import { pondFloatingVisuals } from './r5/pondFloatingArt';
import './pond.css';

export interface R5PondLeafEdit {
  selectedId?: number;
  onSelect: (id: number) => void;
  placement?: PondLeafDraft & StockPositionTool;
}

export interface PondComposedSceneProps {
  pond: PondBasinSnapshot;
  daylightPhase: number;
  seasonDay?: number;
  active: boolean;
  view?: R5ForestView;
  framing?: 'underwater';
  simulationTick: string;
  editRevision: string;
  rockEdit?: R5PondRockEdit;
  sceneryEdit?: R5SceneryEdit;
  waterPlantEdit?: R5PondWaterPlantEdit;
  leafEdit?: R5PondLeafEdit;
  floatingSelectionId?: number;
  sedgeSelectionId?: number;
  stocking?: StockPositionTool;
  onReady?: (backend: string) => void;
}

export function PondComposedScene({
  pond, daylightPhase, seasonDay, active, view, framing, simulationTick, editRevision, rockEdit, sceneryEdit, waterPlantEdit, leafEdit, floatingSelectionId, sedgeSelectionId, stocking, onReady,
}: PondComposedSceneProps) {
  const host = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const lastLayers = useRef<ComposedLayer[]>([]);
  const paintedFrame = useRef<readonly unknown[] | null>(null);
  const [art, setArt] = useState<ComposedArt | null>(null);
  const [failed, setFailed] = useState(false);
  const ready = useRef(onReady);
  ready.current = onReady;
  const interact = active && Boolean(art);
  const rocks = interact ? rockEdit : undefined;
  const scenery = interact ? sceneryEdit : undefined;
  const plantEdit = interact ? waterPlantEdit : undefined;
  const leaf = interact ? leafEdit : undefined;
  const leafPlacement = leaf?.placement;
  const stockTool = interact && pond.wet ? stocking : undefined;
  const observedAnimal = pond.shrimp.find(({ id }) => id === view?.animalId);
  const shrub = pond.mossPatches?.find(({ id }) => rocks?.selectedKind === 'plant' && id === rocks.selectedId);
  const solid = (rocks?.selectedKind === 'wood' ? pond.woods : pond.stones)?.find(({ id }) => id === rocks?.selectedId);
  const selectedPlant = pond.waterPlantPatches?.find(({ id }) => id === plantEdit?.selectedId);
  const selectedLeaf = pond.leafDeposits?.find(({ id }) => id === leaf?.selectedId);
  const surface = compositionSurface(pond);
  const floating = pondFloatingVisuals(pond, surface, atStage({ x: 0, y: bedY }).y);
  const selectedFloating = floating.find(({ id }) => id === floatingSelectionId);
  const floatingAnchors = floating.map(({ id, x, y }) => `${id}:${x},${y}`).join(';');
  const floatingGeometry = floating.map(({ width, blade, roots }) => `${width},${blade},${roots}`).join(';');
  const sedges = compositionSedges(pond);
  const sedgeAnchors = sedges.map(({ patch, x, collarY }) => `${patch.id}:${x},${collarY}`).join(';');
  const sedgeWetness = sedges.map(({ patch }) => `${patch.id}:${patch.rootWetFraction}`).join(';');
  const selectedSedge = sedges.find(({ patch }) => patch.id === sedgeSelectionId);
  const sedgeContact = selectedSedge && { x: selectedSedge.x, y: selectedSedge.collarY };
  const contact = selectedFloating ?? sedgeContact ?? (selectedLeaf ? atStage(bedPoint(selectedLeaf.x)) : selectedPlant ? atStage(bedPoint(selectedPlant.x))
    : shrub ? atStage(mossVisual(shrub).point) : solid ? atStage(bedPoint(solid.x))
    : lastLayers.current.find(({ pick }) => pick.kind === 'scenery' && pick.id === scenery?.selectedId)?.pose);
  const camera = pondCompositionCamera(pond, framing === 'underwater', view, contact,
    rocks?.mode === 'add' || Boolean(scenery?.placement || stockTool || plantEdit?.placing || leafPlacement));
  const rockInput = usePondRockInput({
    pond, edit: rocks, revision: editRevision, canvas, view: stageView(camera),
    pick: (at) => {
      const target = composedPick(lastLayers.current, atStage(at))?.pick;
      return target?.kind === 'scenery' || target?.kind === 'waterPlant' || target?.kind === 'leafDeposit' ? undefined : target;
    },
  });
  const sceneryInput = useR5SceneryInput({
    edit: scenery, revision: editRevision, canvas, scene: pond.scenery ?? [], viewport: camera,
    waterShift: surface - nominalSurface,
    wet: pond.wet, pick: (at) => composedPick(lastLayers.current, at),
  });
  const plantInput = usePondWaterPlantInput({
    edit: plantEdit, pond, revision: editRevision, canvas, view: stageView(camera),
    pick: (point) => {
      const target = composedPick(lastLayers.current, atStage(point))?.pick;
      return target?.kind === 'waterPlant' ? target.id : undefined;
    },
  });
  const latchedBedView = plantInput.latched ?? rockInput.latched;
  const viewport = latchedBedView ? cameraAtStage(latchedBedView) : sceneryInput.latched ?? camera;
  const locator = useStockPosition(leafPlacement ?? stockTool,
    `${editRevision}/${leafPlacement ? `${leafPlacement.id}/${leafPlacement.preset}` : 'stock'}`, stageView(viewport), ({ x, y }) => ({
      x: (x - bedLeft) / bedWidth,
      y: leafPlacement ? (y - bedY + 37) / 52 : 1 + (y - bedY) / (stoneWaterHeight(pond) || 1),
    }), leafPlacement ? { x: leafPlacement.x, y: 37 / 52 } : { x: .5, y: .36 }, (point) => {
      if (!leafPlacement) return pondStockSite(point, pond);
      return point.x >= .08 && point.x <= .92 && point.y >= 0 && point.y <= 1 ? 'clear' : 'outside';
    });
  const leafDraft = leafPlacement && locator.point && locator.state === 'clear'
    ? { id: leafPlacement.id, preset: leafPlacement.preset, x: locator.point.x } : undefined;
  const selectedId = leaf?.selectedId ?? plantEdit?.selectedId ?? scenery?.selectedId ?? rocks?.selectedId;
  const selectedKind = leaf?.selectedId != null ? 'leafDeposit' : plantEdit?.selectedId != null ? 'waterPlant'
    : scenery?.selectedId != null ? 'scenery' : rocks?.selectedKind ?? 'stone';
  const mossDraft = previewPondMoss(pond, rocks?.mossDraft);
  const woodFeeding = currentPondWoodFeeding(pond);
  const woodFeedingKey = woodFeeding.map(({ shrimpId, woodId, offset, biofilmMgC }) =>
    `${shrimpId}:${woodId}:${offset}:${biofilmMgC}`).join(';');
  const woodFeedingSourceKey = woodFeeding.flatMap(({ shrimpId, sources }) =>
    sources ? [`${shrimpId}:${sources.woodBiofilmMgC}:${sources.sharedBiofilmMgC}`] : []).join(';');
  const stoneContacts = currentPondStoneContacts(pond);
  const stoneFeeding = currentPondStoneFeeding(pond, stoneContacts);
  const stoneContactsKey = stoneContacts.map(({ shrimpId, stoneId, offset }) => `${shrimpId}:${stoneId}:${offset}`).join(';');
  const stoneFeedingKey = stoneFeeding.map(({ shrimpId, stoneId, offset, biofilmMgC }) =>
    `${shrimpId}:${stoneId}:${offset}:${biofilmMgC}`).join(';');
  const stoneFeedingSourceKey = stoneFeeding.map(({ shrimpId, stoneBiofilmMgC, sharedBiofilmMgC }) =>
    `${shrimpId}:${stoneBiofilmMgC}:${sharedBiofilmMgC}`).join(';');
  const biofilmDistributionKey = JSON.stringify(pond.biofilmDistribution);
  const stoneBiofilmKey = JSON.stringify(pond.stoneBiofilm);
  const woodBiofilmKey = JSON.stringify(pond.woodBiofilm);

  const render = useRef<() => void>(() => {});
  useLayoutEffect(() => {
    render.current = () => {
      const target = canvas.current;
      const box = host.current;
      if (!active || !art || !target || !box || !box.clientWidth || !box.clientHeight) {
        paintedFrame.current = null;
        return;
      }
      const dpr = window.devicePixelRatio || 1;
      const size = fitSample(box.clientWidth, box.clientHeight, dpr);
      const width = size.pixelWidth;
      const height = size.pixelHeight;
      const draft = rockInput.preview;
      const sceneryDraft = sceneryInput.draft;
      const moss = rocks?.mossDraft;
      const plant = plantInput.draft;
      const frame = [target, art, width, height, dpr, simulationTick, editRevision, daylightPhase, seasonDay,
        pond.id, pond.wet, pond.depthCm, pond.biofilmMgC, biofilmDistributionKey, stoneBiofilmKey, woodBiofilmKey, woodFeedingKey, woodFeedingSourceKey,
        stoneContactsKey, stoneFeedingKey, stoneFeedingSourceKey,
        floatingAnchors, floatingGeometry, pond.floatingCanopy?.lightTransmission, floatingSelectionId,
        sedgeAnchors, sedgeWetness, sedgeSelectionId,
        viewport.x, viewport.y, viewport.width, viewport.height, view?.showScale, selectedKind, selectedId, observedAnimal?.id,
        draft?.id, draft?.kind, draft?.preset, draft?.x, sceneryDraft?.id, sceneryDraft?.style, sceneryDraft?.x, sceneryDraft?.y,
        sceneryDraft?.bedSite,
        moss?.id, moss?.preset, moss?.parent.kind, moss?.parent.id, moss?.offset,
        plant?.id, plant?.x, plant?.ribbonPreset, locator.point?.x, locator.point?.y, locator.state, leafDraft?.id, leafDraft?.preset];
      const backingChanged = target.width !== width || target.height !== height;
      if (!backingChanged && sameComposedFrame(paintedFrame.current, frame)) return;
      if (target.width !== width) target.width = width;
      if (target.height !== height) target.height = height;
      target.style.width = width / dpr + 'px';
      target.style.height = height / dpr + 'px';
      const context = target.getContext('2d', { alpha: false });
      if (!context) return;
      const layers = composedLayers(art, pond, sceneryInput.draft, rockInput.preview, mossDraft, plantInput.draft, leafDraft);
      const selection = layers.find((item) => item.pick.id === selectedId && item.pick.kind === selectedKind)?.pick;
      const sx = width / viewport.width;
      const sy = height / viewport.height;
      context.setTransform(sx, 0, 0, sy, -viewport.x * sx, -viewport.y * sy);
      drawPondComposition(context, art, pond, viewport, daylightPhase, seasonColor(seasonDay, 'light'),
        layers, selection, rockInput.hint, mossDraft?.site, plantInput.site, floatingSelectionId, sedgeSelectionId);
      if (observedAnimal) drawAnimalMarker(context, atStage(pondRockShrimpPoint(observedAnimal, pond)));
      if (locator.point && !leafDraft) {
        const position = leafPlacement ? bedPoint(locator.point.x) : pondRockShrimpPoint(locator.point, pond);
        drawStockMarker(context, atStage(position), locator.state === 'clear');
      }
      context.resetTransform();
      if (view?.showScale) drawRuler(context, viewport, target, R5_WIDTH / composedPixelsPerMm, dpr);
      lastLayers.current = layers;
      target.dataset.pondSceneryAnchors = layers.filter(({ pick, draft }) => pick.kind === 'scenery' && !draft)
        .map(({ pick, pose }) => pick.id + ':' + pose.x + ',' + pose.y).join(';');
      paintedFrame.current = frame;
    };
    render.current();
  });

  useEffect(() => {
    let closed = false;
    void loadComposedArt().then((loaded) => {
      if (closed) return;
      setArt(loaded);
      ready.current?.('canvas2d');
    }).catch((error: unknown) => {
      if (closed) return;
      console.error(error);
      setFailed(true);
      ready.current?.('unavailable');
    });
    return () => { closed = true; };
  }, []);

  useEffect(() => {
    const box = host.current;
    if (!box) return;
    let frame = 0;
    function resize() {
      if (frame) cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => { frame = 0; render.current(); });
    }
    const observer = new ResizeObserver(resize);
    observer.observe(box);
    let resolution: MediaQueryList | undefined;
    function watchResolution() {
      resolution?.removeEventListener('change', watchResolution);
      resolution = matchMedia('(resolution: ' + window.devicePixelRatio + 'dppx)');
      resolution.addEventListener('change', watchResolution);
      resize();
    }
    watchResolution();
    return () => {
      observer.disconnect();
      resolution?.removeEventListener('change', watchResolution);
      if (frame) cancelAnimationFrame(frame);
    };
  }, []);

  const { handlers } = rockInput;
  function pickLeaf(event: PointerEvent<HTMLCanvasElement>) {
    if (!leaf || event.button !== 0 || !event.isPrimary) return false;
    const point = pointerOnSample({ x: event.clientX, y: event.clientY }, event.currentTarget.getBoundingClientRect(), viewport);
    const target = composedPick(lastLayers.current, point)?.pick;
    if (target?.kind !== 'leafDeposit') return false;
    event.preventDefault();
    event.currentTarget.focus({ preventScroll: true });
    leaf.onSelect(target.id);
    return true;
  }
  function abort(event: PointerEvent<HTMLCanvasElement>) {
    if (!plantInput.stop(event, true) && !sceneryInput.stop(event, true)) handlers.onPointerCancel(event);
  }
  const editable = Boolean(rocks || scenery || plantEdit || stockTool || leaf);
  const worldCamera = stageView(viewport);
  return <div ref={host} className="pond-scene" data-pond-id={pond.id} data-wet={pond.wet}
    data-waterline={bedY - stoneWaterHeight(pond)}
    data-daphnia-points={pond.wet ? Math.min(32, Math.ceil(pond.daphniaCount / 6)) : 0} data-zoom={view?.zoom ?? 1}>
    <canvas ref={canvas} className="pond-scene__canvas" data-pond-composed="v2" data-ready={Boolean(art) || undefined}
      role={editable ? undefined : 'img'} tabIndex={editable ? 0 : undefined}
      aria-label={pond.label + (leafPlacement ? '，选择沉叶在池底的显示位置。' : sceneryInput.bedPlacing ? '，在底床放置小砾砂斑，松开鼠标或按回车确认。' : stockTool ? '，选择米虾落点。点击或用方向键选点，回车确认。'
        : plantInput.placing ? '，选择水草的池底落点。' : editable ? '，可拖动石材、沉木、水草和造景。' : '，浅塘生态观赏。')}
      data-pond-framing={framing ?? 'full'} data-daylight-phase={daylightPhase} data-observed-animal-id={observedAnimal?.id}
      data-pond-rock-live="true" data-pond-wood-live="true" data-pond-plant-live="true"
      data-viewport={[worldCamera.x, worldCamera.y, worldCamera.width, worldCamera.height].join(',')}
      data-composed-viewport={[viewport.x, viewport.y, viewport.width, viewport.height].join(',')}
      data-placement-mode={stockTool ? 'stock' : leafPlacement || scenery?.placement || plantInput.placing ? 'add' : rockInput.mode}
      data-placement-preset={leafPlacement?.preset ?? (plantInput.placing ? plantEdit?.ribbonPreset ?? 'hornwort' : scenery?.placement ?? rockInput.preset)}
      data-placement-kind={leafPlacement ? 'leafDeposit' : plantInput.placing ? 'waterPlant' : scenery?.placement ? 'scenery' : rockInput.addKind}
      data-dragging={rockInput.dragging || sceneryInput.dragging || plantInput.dragging || undefined} data-preview-site={rockInput.hint}
      data-bed-y={bedY} data-bed-left={bedLeft} data-bed-width={bedWidth}
      data-pond-rock-anchors={pond.stones?.map(({ id, x }) => id + ':' + bedPoint(x).x + ',' + bedY).join(';')}
      data-pond-wood-anchors={pond.woods?.map(({ id, x }) => id + ':' + bedPoint(x).x + ',' + bedY).join(';')}
      data-pond-structure-sprites={art ? pondTerrainObjects(pond).filter(({ preset }) => pondStructureProfile(preset))
        .map(({ preset, id }) => `${preset}:${id}`).join(',') || undefined : undefined}
      data-pond-wood-contact-feeding={woodFeedingKey || undefined}
      data-pond-wood-contact-feeding-source={woodFeedingSourceKey || undefined}
      data-pond-stone-attachments={stoneContactsKey || undefined}
      data-pond-stone-contact-feeding={stoneFeedingKey || undefined}
      data-pond-stone-contact-feeding-source={stoneFeedingSourceKey || undefined}
      data-pond-biofilm-distribution={biofilmDistributionKey}
      data-pond-stone-biofilm-enabled={pond.stoneBiofilmEnabled === true || undefined}
      data-pond-stone-biofilm={stoneBiofilmKey}
      data-pond-wood-biofilm-enabled={pond.woodBiofilmEnabled === true || undefined}
      data-pond-wood-biofilm={woodBiofilmKey}
      data-pond-scenery-ids={pond.scenery?.map(({ id }) => id).join(',')}
      data-pond-bed-scenery-site={sceneryInput.draft?.bedSite}
      data-pond-water-plant-anchors={pond.waterPlantPatches?.map(({ id, x }) => id + ':' + bedPoint(x).x + ',' + bedY).join(';')}
      data-pond-water-plant-frames={pond.waterPlantPatches?.map((patch) => patch.id + ':' + waterPlantFrame(patch)).join(';')}
      data-pond-ribbon-plants={pond.waterPlantPatches?.flatMap(({ id, ribbonPreset }) => ribbonPreset ? [`${id}:${ribbonPreset}`] : []).join(';') || undefined}
      data-pond-water-plant-draft-site={plantInput.site}
      data-pond-floating-plant-anchors={floatingAnchors || undefined}
      data-pond-floating-canopy-coverage={pond.floatingCanopy?.coverage}
      data-pond-floating-light-transmission={pond.floatingCanopy?.lightTransmission}
      data-selected-pond-floating-plant-id={selectedFloating?.id}
      data-pond-sedge-anchors={sedgeAnchors || undefined}
      data-pond-sedge-wetness={sedgeWetness || undefined}
      data-selected-pond-sedge-id={selectedSedge?.patch.id}
      data-pond-leaf-anchors={pond.leafDeposits?.map(({ id, x }) => `${id}:${x}`).join(';')}
      data-pond-leaf-stages={pond.leafDeposits?.map(({ id, stage }) => `${id}:${stage}`).join(';')}
      data-selected-pond-leaf-id={selectedKind === 'leafDeposit' ? selectedId : undefined}
      data-pond-leaf-draft-site={leafPlacement ? locator.state : undefined}
      data-selected-pond-scenery-id={scenery?.selectedId}
      data-selected-pond-wood-id={selectedKind === 'wood' ? selectedId : undefined}
      data-selected-pond-rock-id={selectedKind === 'stone' ? selectedId : undefined}
      data-selected-pond-plant-id={selectedKind === 'plant' ? selectedId : undefined}
      data-selected-pond-water-plant-id={selectedKind === 'waterPlant' ? selectedId : undefined}
      data-pond-plant-draft-site={mossDraft?.site}
      data-stock-site={stockTool ? locator.state : undefined}
      {...(stockTool || leafPlacement ? locator.handlers : {
        onPointerDown: (event) => {
          if (sceneryInput.bedPlacing && sceneryInput.start(event)) return;
          if (!pickLeaf(event) && !plantInput.start(event) && !sceneryInput.start(event)) handlers.onPointerDown(event);
        },
        onPointerMove: (event) => { if (!plantInput.move(event) && !sceneryInput.move(event)) handlers.onPointerMove(event); },
        onPointerUp: (event) => { if (!plantInput.stop(event) && !sceneryInput.stop(event)) handlers.onPointerUp(event); },
        onPointerCancel: abort, onLostPointerCapture: abort,
        onPointerLeave: () => { plantInput.leave(); sceneryInput.leave(); handlers.onPointerLeave(); },
        onKeyDown: (event) => { if (!plantInput.keyboard(event) && !sceneryInput.keyboard(event)) handlers.onKeyDown(event); },
      })}
    />
    {!art && <p className="pond-scene__status" role={failed ? 'alert' : 'status'}>
      {failed ? '浅塘画面暂时无法显示' : '正在打开浅塘画面'}
    </p>}
  </div>;
}
