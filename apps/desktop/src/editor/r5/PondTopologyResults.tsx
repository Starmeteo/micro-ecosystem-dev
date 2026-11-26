import type { ReactNode } from 'react';
import type { PondTopologyPreview } from './pondTopologyPreview';

const number = new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 2 });
const names: Record<PondTopologyPreview['kind'], string> = {
  unchanged: '未改变水域',
  reshape: '调整原水域',
  split: '分成多个水域',
  merge: '连通多个水域',
  remove: '清除原水域',
  detached: '形成分离水格',
};

function measure(value: number | null, unit: string) {
  return value === null ? '待确定' : `${number.format(value)} ${unit}`;
}

export function PondTopologyResults({ result, action }: { result: PondTopologyPreview; action?: ReactNode }) {
  const routesBiology = result.kind === 'split' || result.kind === 'merge';
  const showsShrimp = result.affectedBasins.some(basin => basin.shrimp > 0);
  const showsDaphnia = result.affectedBasins.some(basin => basin.daphnia > 0);
  const showsFloatingPlants = result.affectedBasins.some(basin => basin.floatingPlants > 0);
  const showsLeafDeposits = result.affectedBasins.some(basin => basin.leafDeposits > 0);
  const showsWaterPlants = result.affectedBasins.some(basin => basin.waterPlants > 0);
  const showsMoss = result.affectedBasins.some(basin => basin.moss > 0);
  const showsFilm = result.affectedBasins.some(basin => basin.filmedStones > 0);
  const showsRoutes = showsDaphnia || showsFloatingPlants || result.affectedBasins.some(basin => basin.hasSectionEntities || basin.staticSections > 0);
  return <div className="pond-topology" data-pond-topology-result={result.kind}>
    <div className="pond-topology__summary">
      <p role="status" className="pond-shape__status">
        <strong>{names[result.kind]}</strong> · 全图将有 {result.resultingBasinCount} 个水域
      </p>
      {action}
    </div>
    {result.issues.length > 0 && <ul className="pond-topology__issues" aria-label="待解决条件">
      {result.issues.map(({ code, text }) => <li key={code} data-pond-topology-issue={code}>{text}</li>)}
    </ul>}
    {result.affectedBasins.length > 0 && <details className="editor-details editor-details--small pond-topology__detail">
      <summary>受影响水域 · 原有水量 {measure(result.totalWaterLiters, 'L')}</summary>
      <ul className="pond-topology__list">
        {result.affectedBasins.map(basin => <li key={basin.id}>
          <strong>{basin.label}</strong> · {measure(basin.waterLiters, 'L')}
          <div>米虾 {basin.shrimp} 只，水蚤 {basin.daphnia} 只，浮叶 {basin.floatingPlants} 片，物件 {basin.objects} 件，造景 {basin.scenery} 件</div>
          {basin.leafDeposits > 0 && <div>定位沉叶 {basin.leafDeposits} 处</div>}
          {basin.staticSections > 0 && <div>含 {basin.staticSections} 条副观景剖面</div>}
          {basin.waterPlants > 0 ? <div>扎根水草 {basin.waterPlants} 丛</div> : basin.plantMgC > 0 && <div>含活体水草库存</div>}
          {basin.moss > 0 && <div>活体附着苔藓 {basin.moss} 处</div>}
          {basin.filmedStones > 0 && <div>有膜主石 {basin.filmedStones} 块</div>}
          {basin.hasInventory && <div>含池内资源库存</div>}
        </li>)}
      </ul>
    </details>}
    {result.components.length > 0 && <ul className="pond-topology__list" aria-label="预览水域估算">
      {result.components.map((region, index) => <li key={region.cells[0]} data-pond-topology-component={index + 1}>
        <strong className="pond-topology__region" data-topology-tone={index % 4}>水域 {index + 1}</strong>
        <dl className="editor-stock">
          <div><dt>水面</dt><dd data-pond-topology-area={region.areaCm2 ?? undefined}>{measure(region.areaCm2, 'cm²')}</dd></div>
          <div><dt>容量</dt><dd data-pond-topology-capacity={region.capacityLiters ?? undefined}>{measure(region.capacityLiters, 'L')}</dd></div>
          <div><dt>水量估算</dt><dd data-pond-topology-water={region.waterLiters ?? undefined}>{measure(region.waterLiters, 'L')}</dd></div>
          <div><dt>均深估算</dt><dd>{measure(region.depthCm, 'cm')}</dd></div>
          {routesBiology && <>
            <div><dt>主观景剖面</dt><dd data-pond-topology-observation-row={region.observationRow ?? undefined}>{region.observationRow === null ? '需要完整一行'
              : `第 ${region.observationRow + 1} 行${region.retainsObservationSection ? '（保留）' : ''}`}</dd></div>
            {region.staticSectionRows.length > 0 && <div><dt>副观景剖面</dt>
              <dd data-pond-topology-static-rows={region.staticSectionRows.join(',')}>
                {region.staticSectionRows.map(row => `第 ${row + 1} 行`).join('、')}
              </dd></div>}
            {showsShrimp && <div><dt>米虾</dt><dd data-pond-topology-shrimp={region.shrimp}>{region.shrimp} 只</dd></div>}
            {showsDaphnia && <div><dt>水蚤</dt><dd data-pond-topology-daphnia={region.daphnia}>{region.daphnia} 只</dd></div>}
            {showsFloatingPlants && <div><dt>活体浮叶</dt><dd data-pond-topology-floating={region.floatingPlants}>{region.floatingPlants} 片</dd></div>}
            {showsLeafDeposits && <div><dt>定位沉叶</dt><dd data-pond-topology-leaf-deposits={region.leafDeposits}>{region.leafDeposits} 处</dd></div>}
            {showsWaterPlants && <div><dt>扎根水草</dt><dd data-pond-topology-water-plants={region.waterPlants}>{region.waterPlants} 丛</dd></div>}
            {showsMoss && <div><dt>活体附着苔藓</dt><dd data-pond-topology-moss={region.moss}>{region.moss} 处</dd></div>}
            {showsFilm && <div><dt>有膜主石</dt><dd data-pond-topology-filmed-stones={region.filmedStones}>{region.filmedStones} 块</dd></div>}
          </>}
          {result.canApply && region.retainsStaticLayout && <div data-pond-topology-retains-layout><dt>原有布置</dt><dd>保留石木与造景</dd></div>}
        </dl>
      </li>)}
    </ul>}
    {result.canApply && showsRoutes && <p className="pond-shape__help">
      {result.kind === 'split'
        ? '米虾、扎根水草、沉叶和石木附着物随所在剖面归入水域。浮叶随所在水格归属，水蚤按面积分配。'
        : '所选水域保留主剖面，其余水域的布置和局部生物保留在各自副剖面。各剖面共用水量和水质。'}
    </p>}
  </div>;
}
