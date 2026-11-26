import { Fragment } from 'react';
import type { PondBasinSnapshot } from '../../world';
import { currentPondWoodFeeding } from '../../render/r5/pondWoodFeeding';

const carbon = new Intl.NumberFormat('zh-CN', { maximumSignificantDigits: 3, useGrouping: false });

export function PondWoodFeedingReadout({ basin }: { basin: PondBasinSnapshot }) {
  const active = currentPondWoodFeeding(basin);
  return <details className="editor-details editor-details--small" data-pond-wood-feeding-readout={active.length}>
    <summary>接触沉木时的摄食{active.length ? ` · ${active.length} 只` : ''}</summary>
    {active.length ? <dl className="editor-stock">
      {active.map(({ shrimpId, woodId, biofilmMgC, sources }) => (
        <Fragment key={shrimpId}>
          <div>
            <dt>沉木 {woodId} · 当步摄入</dt>
            <dd data-pond-wood-feeding-shrimp={shrimpId} data-pond-wood-feeding-mg-c={biofilmMgC}>{carbon.format(biofilmMgC)} mg C</dd>
          </div>
          {sources && <>
            <div>
              <dt>来自沉木表面</dt>
              <dd data-pond-wood-feeding-wood-mg-c={sources.woodBiofilmMgC}>{carbon.format(sources.woodBiofilmMgC)} mg C</dd>
            </div>
            <div>
              <dt>来自整池共用份额</dt>
              <dd data-pond-wood-feeding-shared-mg-c={sources.sharedBiofilmMgC}>{carbon.format(sources.sharedBiofilmMgC)} mg C</dd>
            </div>
          </>}
        </Fragment>
      ))}
    </dl> : <p className="editor-context__hint">当前快照无记录。</p>}
    <p className="editor-context__hint">{active.some(event => event.sources)
      ? '两份摄入都来自同一份整池生物膜库存。'
      : '显示米虾接触沉木期间从整池生物膜库存获得的食物量。'}</p>
  </details>;
}
