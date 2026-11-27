import { pondBiofilmDisplay } from '../../render/r5/pondBiofilm';
import type { PondBasinSnapshot } from '../../world';

function carbon(mgC: number) {
  return mgC > 0 && mgC < 0.01 ? '<0.01 mg C' : `${mgC.toFixed(2)} mg C`;
}

const precise = new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 8, useGrouping: false });

export function PondBiofilmReadout({ basin, editable, enable, enableWood, localSection = false }: {
  basin: PondBasinSnapshot;
  editable: boolean;
  enable: () => void;
  enableWood: () => void;
  localSection?: boolean;
}) {
  const { bedMgC, inactiveMgC, surfaces } = pondBiofilmDisplay(basin);
  const stonePersistent = basin.stoneBiofilmEnabled === true;
  const woodPersistent = basin.woodBiofilmEnabled === true;
  const persistent = stonePersistent || woodPersistent;
  const stones = basin.stoneBiofilm ?? [];
  const woods = basin.woodBiofilm ?? [];
  return (
    <details className="editor-details editor-details--small" data-pond-biofilm-display
      data-pond-stone-biofilm-status={stonePersistent ? 'enabled' : undefined}
      data-pond-wood-biofilm-status={woodPersistent ? 'enabled' : undefined}>
      <summary>{woodPersistent ? '石木生物膜与库存' : stonePersistent ? '石面生物膜与库存'
        : basin.biofilmDistribution ? '当前生物膜分配' : '附着生物膜的画面分配'}</summary>
      <dl className="editor-stock">
        {basin.wet ? <>
          <div><dt>{persistent ? '池底及共用' : '池底'}</dt><dd data-pond-biofilm-bed={bedMgC}>{carbon(bedMgC)}</dd></div>
          {surfaces.filter(({ kind }) => kind === 'stone' ? !stonePersistent : !woodPersistent).map(({ kind, id, mgC }) => (
            <div key={`${kind}:${id}`}>
              <dt>{kind === 'stone' ? '主石' : '沉木'} {id}</dt>
              <dd data-pond-biofilm-carrier={`${kind}:${id}`} data-pond-biofilm-mg-c={mgC}>{carbon(mgC)}</dd>
            </div>
          ))}
          {persistent && inactiveMgC > 0 &&
            <div><dt>离水保留量</dt><dd data-pond-biofilm-inactive={inactiveMgC}>{carbon(inactiveMgC)}</dd></div>}
        </> : <div><dt>干底保留库存</dt><dd data-pond-biofilm-inactive={inactiveMgC}>{carbon(inactiveMgC)}</dd></div>}
      </dl>
      {stonePersistent ? <>
        {stones.map(({ stoneId, stockMgC, availableMgC }) => (
          <dl className="editor-stock" key={stoneId} data-pond-stone-biofilm-id={stoneId}>
            <div><dt>主石 {stoneId} 库存</dt>
              <dd data-pond-stone-biofilm-stock={stockMgC}>{precise.format(stockMgC)} mg C</dd></div>
            <div><dt>浸水可食</dt>
              <dd data-pond-stone-biofilm-available={availableMgC}>{precise.format(availableMgC)} mg C</dd></div>
          </dl>
        ))}
        {!stones.length && <p className="editor-context__hint">放入主石后可查看该石面的库存。</p>}
        <p className="editor-context__hint">停靠的米虾优先摄食该主石的浸水生物膜。离水时保留库存，移除主石时库存回到整池共用。</p>
      </> : <>
        <p className="editor-context__hint">{basin.wet
          ? basin.biofilmDistribution
            ? '共用库存按当前剖面承载面的浸水部分显示。'
            : '画面按浸水部分分配整池库存，各处读数用于说明膜色浓淡。'
          : '水体干涸时保留库存。'}</p>
        {(basin.biofilmDistribution || localSection) && <>
          <p className="editor-context__hint">启用后，各主石库存随取食和生长变化，同时启用米虾停靠主石。</p>
          <button type="button" className="editor-tool" disabled={!editable} data-pond-stone-biofilm-enable
            onClick={enable}>启用石面生物膜</button>
          {!editable && <p className="editor-context__hint">进入环境编辑后可启用。</p>}
        </>}
      </>}
      {woodPersistent ? <>
        {woods.map(({ woodId, stockMgC, availableMgC }) => (
          <dl className="editor-stock" key={woodId} data-pond-wood-biofilm-id={woodId}>
            <div><dt>沉木 {woodId} 库存</dt>
              <dd data-pond-wood-biofilm-stock={stockMgC}>{precise.format(stockMgC)} mg C</dd></div>
            <div><dt>浸水可食</dt>
              <dd data-pond-wood-biofilm-available={availableMgC}>{precise.format(availableMgC)} mg C</dd></div>
          </dl>
        ))}
        {!woods.length && <p className="editor-context__hint">放入沉木后可查看该木段的库存。</p>}
        <p className="editor-context__hint">沉木移动时库存随木段保留，移除时回到全池共用。离水库存暂时不可取食。</p>
      </> : <>
        <p className="editor-context__hint">启用后，米虾停靠时会优先取食该木段的浸水库存。</p>
        <button type="button" className="editor-tool" disabled={!editable} data-pond-wood-biofilm-enable
          onClick={enableWood}>启用沉木生物膜</button>
        {!editable && <p className="editor-context__hint">进入环境编辑后可启用。</p>}
      </>}
    </details>
  );
}
