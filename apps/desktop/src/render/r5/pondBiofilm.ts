import type { PondBasinSnapshot, PondBiofilmDistribution } from '../../world';
import { bedWidth, pondTerrainObjects, pondTerrainWettedWeight, stoneWaterHeight } from './pondStoneGeometry';

// Side profiles set the maximum visual share. The exposed portion of each profile returns to the bed display.
const bedWeight = bedWidth * 36;

export function pondBiofilmDisplay(basin: PondBasinSnapshot): PondBiofilmDistribution {
  if (basin.biofilmDistribution) return basin.biofilmDistribution;
  const total = Number.isFinite(basin.biofilmMgC) ? Math.max(0, basin.biofilmMgC) : 0;
  if (!basin.wet) return { bedMgC: 0, inactiveMgC: total, surfaces: [] };
  if (total === 0) return { bedMgC: 0, inactiveMgC: 0, surfaces: [] };

  const waterColumn = stoneWaterHeight(basin);
  const supported = pondTerrainObjects(basin)
    .filter((item) => item.kind === 'wood' || item.preset === 'main');
  const combinedWeight = supported.reduce((sum, item) => sum + pondTerrainWettedWeight(item, Infinity), bedWeight);
  const surfaces = supported
    .map((item) => ({ kind: item.kind, id: item.id, mgC: total * (pondTerrainWettedWeight(item, waterColumn) / combinedWeight) }))
    .filter(({ mgC }) => mgC > 0);
  const bedMgC = total - surfaces.reduce((sum, surface) => sum + surface.mgC, 0);
  return { bedMgC, inactiveMgC: 0, surfaces };
}
