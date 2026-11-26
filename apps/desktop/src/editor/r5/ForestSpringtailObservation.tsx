import type { ForestSpringtailStock, ForestSpringtailView } from '../../render/r5/forestSpringtails';

function carbon(mg: number) {
  return mg > 0 && mg < .01 ? '<0.01 mg C' : mg.toFixed(2) + ' mg C';
}

export function ForestSpringtailObservation({ stock, mode, zoom, observe }: {
  stock?: ForestSpringtailStock;
  mode?: ForestSpringtailView;
  zoom: number;
  observe: (mode?: ForestSpringtailView) => void;
}) {
  return <section className="editor-context__section" data-forest-springtail-observation>
    <h2>弹尾虫群</h2>
    {stock ? <>
      <dl className="editor-stock">
        <div><dt>左区生物量</dt><dd data-springtail-left-mg-c={stock[0]}>{carbon(stock[0])}</dd></div>
        <div><dt>右区生物量</dt><dd data-springtail-right-mg-c={stock[1]}>{carbon(stock[1])}</dd></div>
      </dl>
      <p className="editor-context__hint">微点表示群体密度，无法据此辨认单只或计数。</p>
      <div className="editor-details__options">
        <button className="editor-tool" data-springtail-toggle type="button" aria-pressed={Boolean(mode)}
          onClick={() => observe(mode ? undefined : 'both')}>{mode ? '隐藏观测点' : '显示观测点'}</button>
        <button className="editor-tool" data-springtail-zoom="left" type="button" aria-pressed={mode === 'left' && zoom > 1}
          onClick={() => observe('left')}>4× 左区</button>
        <button className="editor-tool" data-springtail-zoom="right" type="button" aria-pressed={mode === 'right' && zoom > 1}
          onClick={() => observe('right')}>4× 右区</button>
      </div>
    </> : <p className="editor-context__hint">当前快照暂无弹尾虫库存。</p>}
  </section>;
}
