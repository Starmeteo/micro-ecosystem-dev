import type { WorldSnapshot } from '../world';
import { ForestScene, type ForestSceneProps } from './ForestScene';
import { ForestComposedScene, type R5ForestPlantEdit } from './ForestComposedScene';
import { PondScene } from './PondScene';
import { PondComposedScene, type R5PondLeafEdit } from './PondComposedScene';
import { ForestLiveScene, type R5ForestEdit, type R5ForestView } from './r5/ForestLiveScene';
import type { R5PondRockEdit } from './r5/pondStoneInput';
import type { R5PondWaterPlantEdit } from './r5/pondWaterPlantInput';
import type { R5SceneryEdit } from './r5/pondSceneryInput';
import type { R5ForestSceneryEdit } from './r5/forestSceneryInput';
import type { R5ForestBrush } from './r5/forestBrushInput';
import type { StockPositionTool } from './r5/stockPosition';

export function WorldScene({ snapshot, basinId, forestEdit, forestBrush, forestMossEdit, forestFernEdit, forestView, forestSceneryEdit, pondRockEdit, pondSceneryEdit, pondWaterPlantEdit, pondLeafEdit, pondFloatingSelectionId, pondSedgeSelectionId, pondFraming, stocking, ...forestProps }: Omit<ForestSceneProps, 'snapshot'> & {
  snapshot: WorldSnapshot;
  basinId?: string;
  forestEdit?: R5ForestEdit;
  forestBrush?: R5ForestBrush;
  forestMossEdit?: R5ForestPlantEdit;
  forestFernEdit?: R5ForestPlantEdit;
  forestView?: R5ForestView;
  forestSceneryEdit?: R5ForestSceneryEdit;
  pondRockEdit?: R5PondRockEdit;
  pondSceneryEdit?: R5SceneryEdit;
  pondWaterPlantEdit?: R5PondWaterPlantEdit;
  pondLeafEdit?: R5PondLeafEdit;
  pondFloatingSelectionId?: number;
  pondSedgeSelectionId?: number;
  pondFraming?: 'underwater';
  stocking?: StockPositionTool;
}) {
  const basins = snapshot.pond?.basins;
  const basin = basins?.find((entry) => entry.id === basinId) ?? basins?.[0];
  if (basin && Array.isArray(basin.scenery)) return <PondComposedScene key={`${snapshot.worldId}/${snapshot.runGeneration}/${basin.id}`}
    pond={basin} daylightPhase={snapshot.daylightPhase} seasonDay={snapshot.season?.dayOfYear}
    active={forestProps.active ?? true} view={forestView} rockEdit={pondRockEdit} sceneryEdit={pondSceneryEdit} stocking={stocking}
    waterPlantEdit={pondWaterPlantEdit} leafEdit={pondLeafEdit} floatingSelectionId={pondFloatingSelectionId} sedgeSelectionId={pondSedgeSelectionId}
    simulationTick={snapshot.simulationTick} editRevision={snapshot.editRevision} framing={pondFraming} onReady={forestProps.onReady} />;
  if (basin) return <PondScene key={Array.isArray(basin.stones) ? `${snapshot.worldId}/${snapshot.runGeneration}/${basin.id}` : undefined}
    pond={basin} daylightPhase={snapshot.daylightPhase} seasonDay={snapshot.season?.dayOfYear}
    active={forestProps.active ?? true} view={forestView} rockEdit={pondRockEdit} editRevision={snapshot.editRevision} framing={pondFraming}
    onReady={forestProps.onReady} />;
  if (Array.isArray(snapshot.forestRocks?.scenery)) return <ForestComposedScene key={`${snapshot.worldId}/${snapshot.runGeneration}`}
    snapshot={snapshot} active={forestProps.active} onReady={forestProps.onReady} forestEdit={forestEdit}
    forestView={forestView} sceneryEdit={forestSceneryEdit} forestBrush={forestBrush} stocking={stocking} mossEdit={forestMossEdit} fernEdit={forestFernEdit} />;
  if (snapshot.forestRocks) return <ForestLiveScene key={`${snapshot.worldId}/${snapshot.runGeneration}`}
    {...{ snapshot, forestEdit, forestView, active: forestProps.active, onReady: forestProps.onReady }} />;
  return <ForestScene snapshot={snapshot} {...forestProps} />;
}
