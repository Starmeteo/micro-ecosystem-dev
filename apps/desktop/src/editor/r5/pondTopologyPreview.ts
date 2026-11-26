import type { PondBasinSnapshot, PondFloatingPlant, PondShapeMode, PondShapeSnapshot } from '../../world';

const areaLimit = { min: 400, max: 10000 };
const minimumWaterLiters = 0.05;
const basinLimit = 4;
const sectionLimit = 4;
const floatingPlantLimit = 8;
const sedgeLimit = 8;
const daphniaLimit = 2048;

type TopologyKind = 'unchanged' | 'reshape' | 'split' | 'merge' | 'remove' | 'detached';
type IssueCode = 'invalid-cells' | 'incompatible-grid' | 'detached-extension' | 'protected-section' | 'whole-basin' |
  'basin-limit' | 'cell-area' | 'depth-limit' | 'area-limit' | 'capacity' | 'minimum-water' | 'missing-section' |
  'lost-anchor' | 'lost-sedge-anchor' | 'population-limit' | 'section-limit' | 'ordinary-brush';

interface TopologyComponent {
  cells: number[];
  sourceBasinIds: string[];
  areaCm2: number | null;
  capacityLiters: number | null;
  waterLiters: number | null;
  depthCm: number | null;
  observationRow: number | null;
  staticSectionRows: number[];
  retainsStaticLayout: boolean;
  retainsObservationSection: boolean;
  shrimp: number;
  daphnia: number;
  floatingPlants: number;
  shoreSedges: number;
  leafDeposits: number;
  waterPlants: number;
  moss: number;
  filmedStones: number;
}

export interface PondTopologyPreview {
  kind: TopologyKind;
  canApply: boolean;
  changedCells: number[];
  components: TopologyComponent[];
  affectedBasins: ReturnType<typeof summarizeBasin>[];
  issues: { code: IssueCode; text: string }[];
  resultingBasinCount: number;
  totalWaterLiters: number;
  sharedCellAreaCm2: number | null;
}

function adjacentCells(cell: number, columns: number, cellCount: number) {
  const column = cell % columns;
  const adjacent = [];
  if (column > 0) adjacent.push(cell - 1);
  if (column + 1 < columns) adjacent.push(cell + 1);
  if (cell >= columns) adjacent.push(cell - columns);
  if (cell + columns < cellCount) adjacent.push(cell + columns);
  return adjacent;
}

function connectedComponents(cells: Set<number>, columns: number, cellCount: number) {
  const remaining = new Set(cells);
  const components: number[][] = [];
  for (const first of cells) {
    if (!remaining.delete(first)) continue;
    const component = [first];
    for (const cell of component) {
      for (const adjacent of adjacentCells(cell, columns, cellCount)) {
        if (remaining.delete(adjacent)) component.push(adjacent);
      }
    }
    components.push(component.sort((left, right) => left - right));
  }
  return components.sort(([left], [right]) => left - right);
}

function commonScale(quantities: readonly number[]) {
  const first = quantities[0];
  return Number.isFinite(first) && first > 0 && quantities.every(value => Math.abs(value - first) <= 1e-8) ? first : null;
}

export function floatingPlantCell({ position }: PondFloatingPlant, { columns, rows }: PondShapeSnapshot) {
  return Math.floor(position.y * rows) * columns + Math.floor(position.x * columns);
}

export function pondProtectedRows(basin: PondBasinSnapshot, shape: PondShapeSnapshot) {
  return new Set([shape.protectedRow, ...(basin.staticSections ?? []).map(section => section.row)]);
}

function sectionCounts(built: Pick<PondBasinSnapshot,
  'stones' | 'woods' | 'scenery' | 'leafDeposits' | 'waterPlantPatches' | 'shrimp' | 'mossPatches' | 'stoneBiofilm'> | NonNullable<PondBasinSnapshot['staticSections']>[number]) {
  return {
    solids: (built.stones?.length ?? 0) + (built.woods?.length ?? 0),
    scenery: built.scenery?.length ?? 0,
    leafDeposits: built.leafDeposits?.length ?? 0,
    waterPlants: built.waterPlantPatches?.length ?? 0,
    shrimp: built.shrimp?.length ?? 0,
    moss: built.mossPatches?.length ?? 0,
    filmedStones: built.stoneBiofilm?.filter(({ stockMgC }) => stockMgC > 0).length ?? 0,
  };
}

function summarizeBasin(basin: PondBasinSnapshot) {
  const main = sectionCounts(basin);
  const side = (basin.staticSections ?? []).map(sectionCounts);
  const solids = side.reduce((sum, built) => sum + built.solids, main.solids);
  const scenery = side.reduce((sum, built) => sum + built.scenery, main.scenery);
  const leafDeposits = side.reduce((sum, built) => sum + built.leafDeposits, main.leafDeposits);
  const waterPlants = side.reduce((sum, built) => sum + built.waterPlants, main.waterPlants);
  const shrimp = side.reduce((sum, built) => sum + built.shrimp, main.shrimp);
  const moss = side.reduce((sum, built) => sum + built.moss, main.moss);
  const filmedStones = side.reduce((sum, built) => sum + built.filmedStones, main.filmedStones);
  return {
    id: basin.id,
    label: basin.label,
    waterLiters: basin.waterLiters,
    shrimp,
    moss,
    filmedStones,
    daphnia: basin.daphniaCount,
    floatingPlants: basin.floatingPlants?.length ?? 0,
    leafDeposits,
    waterPlants,
    plantMgC: basin.plantMgC,
    objects: solids,
    scenery,
    hasMainStaticLayout: main.solids + main.scenery > 0,
    staticSections: side.length,
    hasSectionEntities: shrimp + solids + scenery + leafDeposits + waterPlants + moss + filmedStones > 0,
    hasInventory: [basin.algaeMgC, basin.biofilmMgC, basin.detritusMgC, basin.availableNitrogenMgPerLiter ?? 0,
      basin.availablePhosphorusMgPerLiter ?? 0].some(value => value > 0),
  };
}

function existingStaticRows(source: PondBasinSnapshot, shape: PondShapeSnapshot, keepMainLayout: boolean) {
  const secondary = (source.staticSections ?? []).map(section => ({ row: section.row, counts: sectionCounts(section) }));
  const main = sectionCounts(source);
  if (keepMainLayout && (source.retainObservationRow || main.solids + main.scenery + main.leafDeposits + main.waterPlants + main.shrimp + main.moss + main.filmedStones > 0)) {
    secondary.push({ row: shape.protectedRow, counts: main });
  }
  return secondary;
}

function splitDaphnia(population: number, components: readonly TopologyComponent[]) {
  const totalCells = components.reduce((sum, { cells }) => sum + cells.length, 0);
  const shares = components.map(({ cells }, index) => {
    const numerator = population * cells.length;
    return { index, count: Math.floor(numerator / totalCells), remainder: numerator % totalCells };
  });
  const remaining = population - shares.reduce((sum, { count }) => sum + count, 0);
  const ranked = [...shares].sort((left, right) => right.remainder - left.remainder || left.index - right.index);
  for (const share of ranked.slice(0, remaining)) share.count += 1;
  return shares.map(({ count }) => count);
}

export function previewPondTopology(basin: PondBasinSnapshot, shape: PondShapeSnapshot,
  basins: readonly PondBasinSnapshot[], mode: PondShapeMode, touched: readonly number[]): PondTopologyPreview {
  const { columns, rows } = shape;
  const cellCount = columns * rows;
  const inGrid = (cell: number) => Number.isInteger(cell) && cell >= 0 && cell < cellCount;
  const others = basins.filter(other => other.id !== basin.id);
  const footprints = [{ basin, shape }];
  for (const other of others) {
    if (other.shape?.columns === columns && other.shape.rows === rows && other.shape.cells.every(inGrid)) {
      footprints.push({ basin: other, shape: other.shape });
    }
  }
  const unknownCount = others.length + 1 - footprints.length;
  const ownerByCell = new Map<number, string>();
  for (const entry of footprints) {
    for (const cell of entry.shape.cells) ownerByCell.set(cell, entry.basin.id);
  }
  const changedCells = [...new Set(touched)].filter(cell => inGrid(cell) && (mode === 'add'
    ? !ownerByCell.has(cell) : ownerByCell.get(cell) === basin.id)).sort((left, right) => left - right);
  const issues: PondTopologyPreview['issues'] = [];
  if (touched.some(cell => !inGrid(cell))) issues.push({ code: 'invalid-cells', text: '笔画包含俯视格网之外的位置。' });
  if (!changedCells.length) return {
    kind: 'unchanged', canApply: false, changedCells, components: [], affectedBasins: [], issues,
    resultingBasinCount: 1 + others.length, totalWaterLiters: 0, sharedCellAreaCm2: shape.cellAreaCm2,
  };

  const proposedCells = new Set(ownerByCell.keys());
  for (const cell of changedCells) {
    if (mode === 'add') proposedCells.add(cell);
    else proposedCells.delete(cell);
  }
  const addedCells = new Set(mode === 'add' ? changedCells : []);
  const regions = connectedComponents(proposedCells, columns, cellCount).map(cells => {
    const sourceIds = new Set(cells.flatMap(cell => {
      const id = ownerByCell.get(cell);
      return id === undefined ? [] : [id];
    }));
    return { cells, includesAdded: cells.some(cell => addedCells.has(cell)), sourceIds };
  });
  const affectedRegions = regions.filter(region => region.sourceIds.has(basin.id) || region.includesAdded);
  const affectedIds = new Set([basin.id]);
  for (const region of affectedRegions) {
    for (const id of region.sourceIds) affectedIds.add(id);
  }
  const affected = footprints.filter(entry => affectedIds.has(entry.basin.id));
  const affectedBasins = affected.map(entry => summarizeBasin(entry.basin));
  const selectedHasMainLayout = affectedBasins.some(source => source.id === basin.id && source.hasMainStaticLayout);
  const totalWaterLiters = affectedBasins.reduce((sum, entry) => sum + entry.waterLiters, 0);
  const sharedCellAreaCm2 = commonScale(affected.map(entry => entry.shape.cellAreaCm2));
  const resultingBasinCount = regions.length + unknownCount;

  const components = affectedRegions.map(({ cells, sourceIds, includesAdded }): TopologyComponent => {
    const sources = footprints.filter(entry => sourceIds.has(entry.basin.id));
    const sourceBasinIds = sources.map(entry => entry.basin.id);
    const represented = sources.length ? sources : [{ basin, shape }];
    const areaScales = represented.map(entry => entry.shape.cellAreaCm2);
    // Paint uses the selected basin's cell scale; reaching another scale needs a shared map unit.
    if (includesAdded) areaScales.push(shape.cellAreaCm2);
    const cellArea = commonScale(areaScales);
    const maxDepth = commonScale(represented.map(entry => entry.basin.maxDepthCm));
    const areaCm2 = cellArea === null ? null : cells.length * cellArea;
    const capacityLiters = areaCm2 === null || maxDepth === null ? null : areaCm2 * maxDepth / 1000;
    const waterLiters = areaCm2 === null || unknownCount || !sourceIds.has(basin.id) || mode === 'remove' ? null
      : sources.reduce((sum, entry) => sum + entry.basin.waterLiters, 0);
    const componentCells = new Set(cells);
    const fullRows = Array.from({ length: rows }, (_, row) => row).filter(row =>
      Array.from({ length: columns }, (_, column) => row * columns + column).every(cell => componentCells.has(cell)));
    const keepsSection = sourceIds.has(basin.id) && fullRows.includes(shape.protectedRow);
    const storedSections = sources.flatMap(({ basin: source, shape: sourceShape }) =>
      existingStaticRows(source, sourceShape, mode === 'add' && source.id !== basin.id))
      .filter(section => fullRows.includes(section.row))
      .sort((left, right) => left.row - right.row);
    const restoredRow = mode === 'remove' ? storedSections[0]?.row : undefined;
    const observationRow = keepsSection ? shape.protectedRow : restoredRow ?? fullRows[0] ?? null;
    const staticSectionRows = storedSections.map(section => section.row).filter(row => row !== observationRow);
    const floatingPlants = sources.flatMap(({ basin, shape }) =>
      (basin.floatingPlants ?? []).filter(leaf => componentCells.has(floatingPlantCell(leaf, shape)))).length;
    const shoreSedges = sources.flatMap(({ basin }) => basin.shoreSedges ?? [])
      .filter(({ anchor }) => componentCells.has(anchor.waterCell)).length;
    const leafDeposits = storedSections.reduce((sum, { counts }) => sum + counts.leafDeposits,
      keepsSection ? basin.leafDeposits?.length ?? 0 : 0);
    const waterPlants = storedSections.reduce((sum, { counts }) => sum + counts.waterPlants,
      keepsSection ? basin.waterPlantPatches?.length ?? 0 : 0);
    const main = keepsSection ? sectionCounts(basin) : undefined;
    const shrimp = storedSections.reduce((sum, { counts }) => sum + counts.shrimp, main?.shrimp ?? 0);
    const moss = storedSections.reduce((sum, { counts }) => sum + counts.moss, main?.moss ?? 0);
    const filmedStones = storedSections.reduce((sum, { counts }) => sum + counts.filmedStones, main?.filmedStones ?? 0);
    return { cells, sourceBasinIds, areaCm2, capacityLiters, waterLiters,
      depthCm: waterLiters === null || areaCm2 === null ? null : waterLiters * 1000 / areaCm2, observationRow, staticSectionRows,
      retainsStaticLayout: (keepsSection && selectedHasMainLayout) || storedSections.some(({ counts }) => counts.solids + counts.scenery > 0),
      retainsObservationSection: keepsSection || restoredRow !== undefined,
      shrimp, moss, filmedStones,
      daphnia: mode === 'add' ? sources.reduce((sum, entry) => sum + entry.basin.daphniaCount, 0) : 0,
      floatingPlants, shoreSedges, leafDeposits, waterPlants };
  });

  if (mode === 'remove' && components.length && !unknownCount) {
    const totalArea = components.reduce((sum, component) => sum + (component.areaCm2 ?? 0), 0);
    const daphnia = splitDaphnia(basin.daphniaCount, components);
    components.forEach((component, index) => { component.daphnia = daphnia[index]; });
    if (totalArea > 0 && components.every(component => component.areaCm2 !== null)) {
      // Show a common mean depth by apportioning the original water to each remaining footprint.
      for (const component of components) {
        component.waterLiters = basin.waterLiters * (component.areaCm2 ?? 0) / totalArea;
        component.depthCm = basin.waterLiters * 1000 / totalArea;
      }
    }
  }

  let kind: TopologyKind = 'reshape';
  if (!components.length) kind = 'remove';
  else if (mode === 'remove' && components.length > 1) kind = 'split';
  else if (affectedRegions.some(region => !region.sourceIds.has(basin.id))) kind = 'detached';
  else if (affectedIds.size > 1) kind = 'merge';

  if (unknownCount) issues.push({ code: 'incompatible-grid', text: '有水域缺少同一尺寸的俯视格网，水量分配暂留空。' });
  if (kind === 'detached') issues.push({ code: 'detached-extension', text: '部分扩水格与当前水域分离。先画出共边水道才能估算它们的水量。' });
  const protectedRows = pondProtectedRows(basin, shape);
  if (mode === 'remove' && changedCells.some(cell => protectedRows.has(Math.floor(cell / columns)))) {
    issues.push({ code: 'protected-section', text: '填岸已覆盖原有观景剖面。请保留主、副剖面所在的整行。' });
  }
  if (kind === 'remove') issues.push({ code: 'whole-basin', text: '整片水域将消失。需要先确定水量、底床库存和对象的去向。' });
  if (resultingBasinCount < 1 || resultingBasinCount > basinLimit) issues.push({
    code: 'basin-limit', text: `当前模型要求世界内保留 1 至 ${basinLimit} 片水域。`,
  });
  if (components.some(component => component.areaCm2 === null)) issues.push({
    code: 'cell-area', text: '受影响水域的每格面积不同，需统一格面积后才能估算合池的面积、容量和水深。',
  });
  if (components.some(component => component.areaCm2 !== null && component.capacityLiters === null)) issues.push({
    code: 'depth-limit', text: '受影响水域的最大水深口径不一致，需明确共用的容量上限。',
  });
  if (components.some(({ areaCm2 }) => areaCm2 !== null && (areaCm2 < areaLimit.min || areaCm2 > areaLimit.max))) issues.push({
    code: 'area-limit', text: '至少一片预计水域的面积超出 400 至 10,000 cm²。',
  });
  if (components.some(({ waterLiters, capacityLiters }) => waterLiters !== null && capacityLiters !== null &&
    waterLiters > capacityLiters)) issues.push({ code: 'capacity', text: '至少一片预计水域容不下分到的水量，可先排水或调整边界。' });
  if (components.some(({ waterLiters }) => waterLiters !== null && waterLiters > 0 && waterLiters < minimumWaterLiters)) {
    issues.push({ code: 'minimum-water', text: '有水的区域至少需要 0.05 L，可先补水或调整边界。' });
  }
  if (kind === 'split' && components.some(({ observationRow }) => observationRow === null)) issues.push({
    code: 'missing-section', text: `每片水域需要一整行 ${columns} 格作为观赏剖面。可先扩展水域，再填掉连接处。`,
  });
  if (mode === 'remove' && basin.floatingPlants?.some(leaf => !proposedCells.has(floatingPlantCell(leaf, shape)))) {
    issues.push({ code: 'lost-anchor', text: '这次填岸会覆盖活体浮叶的中心。请换一条岸线，或先在活体浮叶中移动它。' });
  }
  if (basins.some(source => source.shoreSedges?.some(({ anchor }) => !proposedCells.has(anchor.waterCell) || proposedCells.has(anchor.shoreCell)))) {
    issues.push({ code: 'lost-sedge-anchor', text: '这笔改形会覆盖莎草所在的岸格或移除其相邻水格。请先移植莎草。' });
  }
  if (kind === 'merge' && components.some(component => component.staticSectionRows.length + 1 > sectionLimit)) {
    issues.push({ code: 'section-limit', text: `每个水域最多保留 ${sectionLimit} 条主、副观景剖面。` });
  }
  if (kind === 'merge') {
    const limits = components.flatMap(({ floatingPlants, shoreSedges, daphnia }) => [
      ...(floatingPlants > floatingPlantLimit ? [`浮叶最多 ${floatingPlantLimit} 片`] : []),
      ...(shoreSedges > sedgeLimit ? [`莎草最多 ${sedgeLimit} 丛`] : []),
      ...(daphnia > daphniaLimit ? [`水蚤最多 ${daphniaLimit} 只`] : []),
    ]);
    if (limits.length) issues.push({ code: 'population-limit', text: `合池后的数量超过单个水域上限（${limits.join('，')}）。` });
  }
  if (kind === 'reshape' && !issues.length) issues.push({ code: 'ordinary-brush', text: '这一笔只调整边界，可用普通池岸笔刷应用。' });
  const canApply = (kind === 'split' || kind === 'merge') && issues.length === 0;
  return { kind, canApply, changedCells, components, affectedBasins, issues, resultingBasinCount, totalWaterLiters, sharedCellAreaCm2 };
}
