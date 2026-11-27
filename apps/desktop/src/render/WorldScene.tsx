import type { WorldSnapshot } from '../world';
import { ForestComposedScene, type R5ForestPlantEdit } from './ForestComposedScene';
import { PondComposedScene, type R5PondLeafEdit } from './PondComposedScene';
import type { R5ForestEdit, R5ForestView } from './r5/forestView';
import type { R5PondRockEdit } from './r5/pondStoneInput';
import type { R5PondWaterPlantEdit } from './r5/pondWaterPlantInput';
import type { R5SceneryEdit } from './r5/pondSceneryInput';
import type { R5ForestSceneryEdit } from './r5/forestSceneryInput';
import type { R5ForestBrush } from './r5/forestBrushInput';
import type { StockPositionTool } from './r5/stockPosition';

export function WorldScene({ snapshot, basinId, forestEdit, forestBrush, forestMossEdit, forestFernEdit, forestView, forestSceneryEdit, pondRockEdit, pondSceneryEdit, pondWaterPlantEdit, pondLeafEdit, pondFloatingSelectionId, pondSedgeSelectionId, pondFraming, stocking, active = true, onReady, onRockMove, onRockSelect, rockSelected }: {
  snapshot: WorldSnapshot;
  basinId?: string;
  active?: boolean;
  onReady?: (backend: string) => void;
  onRockMove?: (x: number, y: number) => void;
  onRockSelect?: () => void;
  rockSelected?: boolean;
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
  if (basin) return <PondComposedScene key={`${snapshot.worldId}/${snapshot.runGeneration}/${basin.id}`}
    pond={basin} daylightPhase={snapshot.daylightPhase} seasonDay={snapshot.season?.dayOfYear}
    active={active} view={forestView} rockEdit={pondRockEdit} sceneryEdit={pondSceneryEdit} stocking={stocking}
    waterPlantEdit={pondWaterPlantEdit} leafEdit={pondLeafEdit} floatingSelectionId={pondFloatingSelectionId} sedgeSelectionId={pondSedgeSelectionId}
    simulationTick={snapshot.simulationTick} editRevision={snapshot.editRevision} framing={pondFraming} onReady={onReady} />;
  return <ForestComposedScene key={`${snapshot.worldId}/${snapshot.runGeneration}`}
    snapshot={snapshot} active={active} onReady={onReady} forestEdit={forestEdit}
    forestView={forestView} sceneryEdit={forestSceneryEdit} forestBrush={forestBrush} stocking={stocking} mossEdit={forestMossEdit} fernEdit={forestFernEdit}
    onRockMove={onRockMove} onRockSelect={onRockSelect} rockSelected={rockSelected} />;
}
