import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { WorldSnapshot } from '../world';
import { forestFernSite, forestMossSite, forestObjects, forestPlacementHint, forestStockSite, leafPatchFrame, legacyForestRockId } from './r5/forestPlacement';
import { fernFrame } from './r5/forestFern';
import { forestSpringtailFocus, forestSpringtailMarkCounts } from './r5/forestSpringtails';
import {
  loadComposedForestArt, forestWidth, forestHeight, forestPixelsPerMm, type ComposedForestArt,
} from './r5/forestComposedArt';
import {
  composedForestCamera, composedForestLayers, drawComposedForest, pickComposedForest, forestStage, forestPosition,
  sameForestLayer, type ForestLayer, type ForestLayerKey,
} from './r5/forestComposition';
import { useComposedForestInput, type R5ForestSceneryEdit } from './r5/forestSceneryInput';
import { drawForestBrush, useForestBrush, type R5ForestBrush } from './r5/forestBrushInput';
import type { R5ForestEdit, R5ForestView } from './r5/forestView';
import { useCanvasFit } from './r5/useCanvasFit';
import { drawRuler } from './r5/drawRuler';
import { R5_WIDTH, type Point } from './r5/geometry';
import { sameComposedFrame } from './r5/composedFrame';
import { drawStockMarker, useStockPosition, type StockPositionTool } from './r5/stockPosition';
import { drawAnimalMarker } from './r5/animalMarker';
import './r5/sample.css';

export interface R5ForestPlantEdit {
  selectedId?: number;
  onSelect: (id: number) => void;
  placement?: StockPositionTool & { moving?: { id: number; position: Point } };
}

interface ForestComposedSceneProps {
  snapshot: WorldSnapshot;
  active?: boolean;
  forestEdit?: R5ForestEdit;
  forestBrush?: R5ForestBrush;
  stocking?: StockPositionTool;
  mossEdit?: R5ForestPlantEdit;
  fernEdit?: R5ForestPlantEdit;
  sceneryEdit?: R5ForestSceneryEdit;
  forestView?: R5ForestView;
  onReady?: (backend: string) => void;
  onRockMove?: (x: number, y: number) => void;
  onRockSelect?: () => void;
  rockSelected?: boolean;
}

const center = { x: forestWidth / 2, y: forestHeight / 2 };

export function ForestComposedScene({ snapshot, active = true, forestEdit, forestBrush, stocking, mossEdit, fernEdit, sceneryEdit, forestView, onReady, onRockMove, onRockSelect, rockSelected }: ForestComposedSceneProps) {
  const host = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const layers = useRef<ForestLayer[]>([]);
  const paintedFrame = useRef<readonly unknown[] | null>(null);
  const [art, setArt] = useState<ComposedForestArt | null>(null);
  const [failed, setFailed] = useState(false);
  const fit = useCanvasFit(host);
  const ready = useRef(onReady);
  ready.current = onReady;
  const terrainEdit = active && art ? forestEdit : undefined;
  const decorEdit = active && art ? sceneryEdit : undefined;
  const brushTool = active && art ? forestBrush : undefined;
  const stockTool = active && art ? stocking : undefined;
  const mossTool = active && art ? mossEdit?.placement : undefined;
  const fernTool = active && art ? fernEdit?.placement : undefined;
  const positionTool = fernTool ?? mossTool ?? stockTool;
  const legacyRock = !snapshot.forestRocks && active && art && (onRockMove || onRockSelect)
    ? { position: snapshot.rock, onMove: onRockMove, onSelect: onRockSelect } : undefined;
  const editable = Boolean(terrainEdit || decorEdit || brushTool || positionTool || legacyRock);
  const items = snapshot.forestRocks?.scenery ?? [];
  const ferns = snapshot.forestRocks?.fernPatches ?? [];
  const fernPreset = snapshot.forestRocks?.fernPreset;
  const terrain = forestObjects(snapshot);
  const selection: ForestLayerKey | undefined = fernEdit?.selectedId != null ? { kind: 'fern', id: fernEdit.selectedId }
    : mossEdit?.selectedId != null ? { kind: 'moss', id: mossEdit.selectedId }
    : decorEdit?.selectedId != null ? { kind: 'scenery', id: decorEdit.selectedId }
    : terrainEdit?.selectedId != null ? { kind: 'terrain', id: terrainEdit.selectedId }
    : legacyRock && rockSelected ? { kind: 'terrain', id: legacyForestRockId } : undefined;
  const selected = selection ? layers.current.find(layer => sameForestLayer(layer.key, selection)) : undefined;
  const observedAnimal = snapshot.animals.find(({ id }) => id === forestView?.animalId);
  const animal = observedAnimal ?? snapshot.animals[0];
  const contact = selected?.anchor ?? (terrain[0] && forestStage(terrain[0].position));
  const placing = terrainEdit?.mode === 'add' || Boolean(decorEdit?.placement || brushTool || positionTool);
  const springtailMode = placing ? undefined : forestView?.springtails;
  const springtails = springtailMode ? snapshot.forest?.life?.springtailMgC : undefined;
  const focused = springtails && (springtailMode === 'left' || springtailMode === 'right') ? forestSpringtailFocus(springtailMode)
    : forestView?.focus === 'animal' && animal ? forestStage(animal)
    : forestView?.focus === 'contact' && contact ? contact : center;
  const camera = composedForestCamera(placing ? 1 : forestView?.zoom ?? 1, focused);
  const input = useComposedForestInput({
    terrain: terrainEdit, scenery: decorEdit, legacyRock, revision: snapshot.editRevision, items, canvas, viewport: camera, selected,
    onMossSelect: mossEdit?.onSelect,
    onFernSelect: fernEdit?.onSelect,
    pick: (at) => pickComposedForest(layers.current, at),
  });
  const viewport = input.latched ?? camera;
  const brush = useForestBrush(brushTool, snapshot.editRevision, viewport, terrain, snapshot.animals);
  const positionKey = fernTool ? 'fern/' + (fernTool.moving?.id ?? 'new') : mossTool ? 'moss/' + (mossTool.moving?.id ?? 'new') : 'stock';
  const stock = useStockPosition(positionTool, `${snapshot.editRevision}/${positionKey}`,
    viewport, forestPosition, fernTool?.moving?.position ?? mossTool?.moving?.position ?? { x: .50, y: .75 }, (position) => fernTool
      ? forestFernSite(position, terrain, ferns, fernTool.moving?.id)
      : mossTool ? forestMossSite(position, terrain, snapshot.forestRocks?.mossPatches ?? [], mossTool.moving?.id)
      : forestStockSite(position, terrain));
  const fernDraft = fernTool && stock.point && stock.state === 'clear'
    ? { id: fernTool.moving?.id ?? null, position: stock.point } : undefined;
  const brushCells = brush.cells?.map(({ index, occupied, unchanged, overCapacity }) =>
    `${index}:${overCapacity ? 'overflow' : unchanged ? 'same' : Number(occupied)}`).join(',');
  const hint = input.terrainDraft && !legacyRock ? forestPlacementHint(input.terrainDraft, terrain, snapshot.animals, ferns) : undefined;
  const viewportText = [viewport.x, viewport.y, viewport.width, viewport.height].join(',');

  useEffect(() => {
    let live = true;
    void loadComposedForestArt().then((loaded) => {
      if (!live) return;
      setArt(loaded);
      ready.current?.('canvas2d-r5');
    }).catch((error: unknown) => {
      if (!live) return;
      console.error(error);
      setFailed(true);
      ready.current?.('unavailable');
    });
    return () => { live = false; };
  }, []);

  useLayoutEffect(() => {
    const output = canvas.current;
    if (!art || !output || !active || !fit.cssWidth || !fit.cssHeight) {
      paintedFrame.current = null;
      return;
    }
    const draft = input.terrainDraft;
    const sceneryDraft = input.sceneryDraft;
    const frame = [output, art, output.width, output.height, fit.devicePixelRatio,
      snapshot.simulationTick, snapshot.editRevision, snapshot.daylightPhase, snapshot.season?.dayOfYear,
      viewport.x, viewport.y, viewport.width, viewport.height, forestView?.showScale, selection?.kind, selection?.id, observedAnimal?.id,
      draft?.id, draft?.kind, draft?.preset, draft?.mirrored, draft?.position.x, draft?.position.y,
      sceneryDraft?.id, sceneryDraft?.style, sceneryDraft?.x, sceneryDraft?.y, brushCells,
      stock.point?.x, stock.point?.y, stock.state, positionKey, springtails?.[0], springtails?.[1]];
    if (sameComposedFrame(paintedFrame.current, frame)) return;
    const ctx = output.getContext('2d', { alpha: false });
    if (!ctx) return;
    layers.current = composedForestLayers(art, snapshot, terrain, input.terrainDraft, input.sceneryDraft, fernDraft);
    const sx = output.width / viewport.width;
    const sy = output.height / viewport.height;
    ctx.setTransform(sx, 0, 0, sy, -viewport.x * sx, -viewport.y * sy);
    drawComposedForest(ctx, art, snapshot, layers.current, selection, hint, springtails);
    if (observedAnimal) drawAnimalMarker(ctx, forestStage(observedAnimal));
    if (brush.cells?.length) drawForestBrush(ctx, brush.cells);
    if (stock.point) drawStockMarker(ctx, forestStage(stock.point), stock.state === 'clear');
    ctx.resetTransform();
    if (forestView?.showScale) drawRuler(ctx, viewport, output, R5_WIDTH / forestPixelsPerMm, fit.devicePixelRatio);
    output.dataset.forestSceneryAnchors = layers.current.filter(({ key, draft }) => key?.kind === 'scenery' && !draft)
      .map(({ key, anchor }) => key!.id + ':' + anchor.x + ',' + anchor.y).join(';');
    output.dataset.forestMycelium = layers.current.flatMap(({ terrain, mycelium }) =>
      terrain && mycelium?.spanMm ? [`${terrain.id}:${mycelium.cellIndex}:${mycelium.spanMm}`] : []).join(',');
    paintedFrame.current = frame;
  });

  const physical = selection?.kind === 'terrain' ? terrain.find(({ id }) => selection.id === id) : undefined;
  return <div ref={host} className="r5-sample">
    <canvas ref={canvas} className="r5-sample__canvas" data-r5-live data-forest-composed="v2"
      width={fit.pixelWidth} height={fit.pixelHeight} style={{ width: fit.cssWidth, height: fit.cssHeight }}
      role={editable ? undefined : 'img'} tabIndex={editable ? 0 : undefined} data-ready={Boolean(art) || undefined}
      aria-label={fernTool ? '选择蕨丛根部位置。点击或用方向键选点，回车确认。'
        : mossTool ? '选择苔藓位置。点击或用方向键选点，回车确认。'
        : stockTool ? '选择鼠妇落点。点击或用方向键选点，回车确认。'
        : brushTool?.kind === 'substrate' ? '林下基质笔刷。拖动选取土格，松开提交。也可用方向键选一格、回车提交。按 Esc 取消。'
        : brushTool ? '林下落叶笔刷。按住拖动铺设，按 Esc 取消。'
        : legacyRock ? '林下。选择石块，拖动或用方向键移动。'
        : editable ? '林下。拖动物件，或用方向键调整，回车确认。' : '林下生态观赏'}
      data-editable={editable || undefined} data-viewport={viewportText} data-composed-viewport={viewportText}
      data-daylight-phase={snapshot.daylightPhase} data-observed-animal-id={observedAnimal?.id}
      data-forest-terrain-anchors={terrain.map(({ position, id }) => {
        const { x, y } = forestStage(position);
        return id + ':' + x + ',' + y;
      }).join(';')}
      data-forest-scenery-ids={items.map(({ id }) => id).join(',')}
      data-rock-ids={snapshot.forestRocks?.rocks.map(({ id }) => id).join(',')}
      data-log-ids={snapshot.forestRocks?.logs?.map(({ id }) => id).join(',')}
      data-stonework-ids={snapshot.forestRocks?.stonework?.map(({ id }) => id).join(',')}
      data-bark-shelter-ids={snapshot.forestRocks?.barkShelters?.map(({ id }) => id).join(',')}
      data-forest-bridge-ids={snapshot.forestRocks?.barkShelters?.filter(({ bridgeClearanceMm }) => bridgeClearanceMm)
        .map(({ id }) => id).join(',')}
      data-bark-occupant-ids={snapshot.forestRocks?.barkShelters?.flatMap(({ occupantIds }) => occupantIds).join(',')}
      data-leaf-patch-ids={snapshot.forestRocks?.leafPatches?.map(({ id }) => id).join(',')}
      data-leaf-patch-frames={snapshot.forestRocks?.leafPatches?.map((patch) => patch.id + ':' + leafPatchFrame(patch)).join(',')}
      data-selected-object-id={physical?.id}
      data-selected-rock-id={physical?.kind === 'rock' ? physical.id : undefined}
      data-selected-log-id={physical?.kind === 'log' ? physical.id : undefined}
      data-selected-bark-shelter-id={physical?.kind === 'barkShelter' ? physical.id : undefined}
      data-selected-leaf-patch-id={physical?.kind === 'leafPatch' ? physical.id : undefined}
      data-selected-forest-scenery-id={selection?.kind === 'scenery' ? selection.id : undefined}
      data-selected-forest-moss-id={selection?.kind === 'moss' ? selection.id : undefined}
      data-forest-moss-ids={snapshot.forestRocks?.mossPatches?.map(({ id }) => id).join(',')}
      data-selected-forest-fern-id={selection?.kind === 'fern' ? selection.id : undefined}
      data-forest-fern-ids={snapshot.forestRocks?.fernPatches?.map(({ id }) => id).join(',')}
      data-forest-fern-anchors={snapshot.forestRocks?.fernPatches?.map(({ id, position }) =>
        id + ':' + position.x + ',' + position.y).join(';')}
      data-forest-fern-frames={fernPreset ? ferns.map((patch) => patch.id + ':' + fernFrame(patch, fernPreset.initialMgCnp[0])).join(',') : undefined}
      data-forest-springtail-view={springtails ? springtailMode : undefined}
      data-forest-springtail-marks={springtails ? forestSpringtailMarkCounts(springtails).join(',') : undefined}
      data-forest-substrate-cells={snapshot.forestRocks?.substrateCells?.filter(({ substrate }) => substrate !== 'native')
        .map(({ cellIndex, substrate }) => cellIndex + ':' + substrate).join(',')}
      data-placement-mode={fernTool ? fernTool.moving ? 'move' : 'add' : mossTool ? 'add' : stockTool ? 'stock' : brushTool ? 'brush' : input.placementMode}
      data-placement-kind={fernTool ? 'forestFern' : mossTool ? 'forestMoss' : stockTool ? 'stockWoodlice'
        : brushTool?.kind === 'substrate' ? 'substrateBrush' : brushTool ? 'leafBrush' : input.placementKind}
      data-placement-preset={brushTool?.kind === 'substrate' ? brushTool.preset.substrate : brushTool?.preset ?? input.placementPreset}
      data-dragging={input.dragging || undefined}
      data-brush-cells={brushCells}
      data-stock-site={stock.state}
      data-preview-site={hint?.site} data-preview-cells={hint?.cells.map(({ index }) => index).join(',')}
      data-preview-foot-cells={input.terrainDraft?.kind === 'barkShelter'
        ? hint?.cells.filter(({ role }) => role === 'solid').map(({ index }) => index).join(',') : undefined}
      data-preview-passage-cells={input.terrainDraft?.kind === 'barkShelter'
        ? hint?.cells.filter(({ role }) => role === 'passage').map(({ index }) => index).join(',') : undefined}
      {...(positionTool ? stock.handlers : brushTool ? brush.handlers : input.handlers)}
    />
    {!art && <span className="r5-sample__status" role={failed ? 'alert' : 'status'}>
      {failed ? '林下画面载入失败' : '正在载入林下画面'}
    </span>}
  </div>;
}
