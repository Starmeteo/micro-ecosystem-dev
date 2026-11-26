import { Fragment } from 'react';
import type { PondBasinSnapshot } from '../../world';
import { currentPondStoneContacts, currentPondStoneFeeding } from '../../render/r5/pondStoneFeeding';

const carbon = new Intl.NumberFormat('zh-CN', { maximumSignificantDigits: 3, useGrouping: false });

export function PondStoneFeedingReadout({ basin, editable, enable }: {
  basin: PondBasinSnapshot;
  editable: boolean;
  enable: () => void;
}) {
  const contacts = currentPondStoneContacts(basin);
  const active = currentPondStoneFeeding(basin, contacts);
  return <details className="editor-details editor-details--small" data-pond-stone-feeding-readout={active.length}>
    <summary>主石停靠与摄食{contacts.length ? ` · ${contacts.length} 只停靠` : ''}</summary>
    {basin.stoneRestEnabled ? (
      <>
        {active.length ? <dl className="editor-stock">
          {active.map(({ shrimpId, stoneId, biofilmMgC, stoneBiofilmMgC, sharedBiofilmMgC }) => (
            <Fragment key={shrimpId}>
              <div>
                <dt>主石 {stoneId} · 当步摄入</dt>
                <dd data-pond-stone-feeding-shrimp={shrimpId} data-pond-stone-feeding-mg-c={biofilmMgC}>{carbon.format(biofilmMgC)} mg C</dd>
              </div>
              <div>
                <dt>来自主石表面</dt>
                <dd data-pond-stone-feeding-stone-mg-c={stoneBiofilmMgC}>{carbon.format(stoneBiofilmMgC)} mg C</dd>
              </div>
              <div>
                <dt>来自整池共用份额</dt>
                <dd data-pond-stone-feeding-shared-mg-c={sharedBiofilmMgC}>{carbon.format(sharedBiofilmMgC)} mg C</dd>
              </div>
            </Fragment>
          ))}
        </dl> : <p className="editor-context__hint">当前快照无摄食记录。</p>}
        <p className="editor-context__hint">{basin.stoneBiofilmEnabled
          ? '浸水石面优先供食，剩余需求由整池共用库存补足。'
          : '两份摄入都来自同一份整池生物膜库存。'}</p>
      </>
    ) : (
      <>
        <p className="editor-context__hint">启用后，米虾可在当前剖面的主石上停靠与摄食。</p>
        <button type="button" className="editor-tool" disabled={!editable} onClick={enable}>启用主石停靠</button>
        {!editable && <p className="editor-context__hint">进入环境编辑后可启用。</p>}
      </>
    )}
  </details>;
}
