import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, useState } from 'react';
import { errorText, native } from '../../world';
import './ObservationTrends.css';

type ForestReading = {
  woodlice: number;
  soilWaterMl: number;
  litterMgC: number;
  springtailMgC: number | null;
};

type BasinReading = {
  id: string;
  shrimp: number;
  daphnia: number;
  waterLiters: number;
  oxygenMgPerLiter: number | null;
  detritusMgC: number;
};

type Reading = { theme: 'forest'; metrics: ForestReading } | { theme: 'pond'; metrics: BasinReading[] };
type Sample = { simulatedMillis: number; reading: Reading };
type History = { recent: Sample[]; day: Sample[] };
type Range = keyof History;
type TrendValue = { time: number; value: number | null };

const forestMetrics = {
  woodlice: { label: '鼠妇', unit: '只', key: 'woodlice', decimals: 0 },
  soilWater: { label: '土壤总水量', unit: 'mL', key: 'soilWaterMl', decimals: 1 },
  litter: { label: '枯落物碳库存', unit: 'mg C', key: 'litterMgC', decimals: 2 },
  springtails: { label: '弹尾虫碳库存', unit: 'mg C', key: 'springtailMgC', decimals: 2 },
} satisfies Record<string, { label: string; unit: string; key: keyof ForestReading; decimals: number }>;

const pondMetrics = {
  oxygen: { label: '溶氧浓度', unit: 'mg/L', key: 'oxygenMgPerLiter', decimals: 2 },
  water: { label: '水量', unit: 'L', key: 'waterLiters', decimals: 2 },
  shrimp: { label: '米虾', unit: '只', key: 'shrimp', decimals: 0 },
  daphnia: { label: '水蚤', unit: '只', key: 'daphnia', decimals: 0 },
  detritus: { label: '碎屑碳库存', unit: 'mg C', key: 'detritusMgC', decimals: 2 },
} satisfies Record<string, { label: string; unit: string; key: keyof BasinReading; decimals: number }>;

const periods = { recent: { label: '近 10 分钟', interval: '10 秒' }, day: { label: '近 24 小时', interval: '10 分钟' } };
const chartWidth = 240;
const chartHeight = 82;

function simulationTime(millis: number) {
  const seconds = Math.floor(millis / 1000);
  const pad = (value: number) => String(value).padStart(2, '0');
  const day = Math.floor(seconds / 86400);
  const time = `${pad(Math.floor(seconds / 3600) % 24)}:${pad(Math.floor(seconds / 60) % 60)}:${pad(seconds % 60)}`;
  return day ? `第 ${day + 1} 天 ${time}` : time;
}

function TrendPlot({ values, label }: { values: readonly TrendValue[]; label: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const ctx = canvas.current?.getContext('2d');
    if (!ctx) return;
    ctx.setTransform(2, 0, 0, 2, 0, 0);
    ctx.clearRect(0, 0, chartWidth, chartHeight);
    const numeric = values.flatMap(({ value }) => value === null ? [] : [value]);
    if (!numeric.length) return;
    const start = values[0].time;
    const end = values[values.length - 1].time;
    const low = Math.min(...numeric);
    const high = Math.max(...numeric);
    const margin = 7;
    const width = chartWidth - 2 * margin;
    const height = chartHeight - 2 * margin;
    const x = (time: number) => margin + (end === start ? .5 : (time - start) / (end - start)) * width;
    const y = (value: number) => margin + (low === high ? .5 : (high - value) / (high - low)) * height;

    ctx.strokeStyle = '#52768b38';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (const row of [margin, chartHeight / 2, chartHeight - margin]) {
      ctx.moveTo(margin, row);
      ctx.lineTo(chartWidth - margin, row);
    }
    ctx.stroke();

    ctx.strokeStyle = '#32647e';
    ctx.fillStyle = '#32647e';
    ctx.lineWidth = 1.5;
    ctx.lineJoin = 'round';
    ctx.beginPath();
    let previous = false;
    for (const point of values) {
      if (point.value === null) { previous = false; continue; }
      if (previous) ctx.lineTo(x(point.time), y(point.value));
      else ctx.moveTo(x(point.time), y(point.value));
      previous = true;
    }
    ctx.stroke();
    for (const point of values) {
      if (point.value === null) continue;
      ctx.beginPath();
      ctx.arc(x(point.time), y(point.value), numeric.length < 20 ? 2.3 : 1.4, 0, Math.PI * 2);
      ctx.fill();
    }
  }, [values]);

  return <canvas ref={canvas} width={2 * chartWidth} height={2 * chartHeight} role="img" aria-label={label} />;
}

export function ObservationTrends({ worldId, generation, basinId, basinLabel, springtails, trial, active }: {
  worldId: string; generation: string; basinId?: string; basinLabel?: string; springtails: boolean; trial: boolean; active: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [range, setRange] = useState<Range>('recent');
  const [metric, setMetric] = useState(basinId ? 'oxygen' : 'woodlice');
  const [history, setHistory] = useState<History | null>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!open || !native || !active) return;
    let closed = false;
    let timer: number | undefined;
    const poll = async () => {
      try {
        const next = await invoke<History>('observation_history', { worldId, expectedGeneration: generation });
        if (!closed) {
          setHistory((last) => {
            const previousTick = last?.recent.at(-1)?.simulatedMillis;
            return next.recent.length > 1 && previousTick === next.recent.at(-1)?.simulatedMillis
              && last?.recent.length === next.recent.length ? last : next;
          });
          setError('');
        }
      } catch (reason) {
        if (!closed) setError(errorText(reason));
      }
      if (!closed) timer = window.setTimeout(poll, 1_500);
    };
    void poll();
    return () => { closed = true; window.clearTimeout(timer); };
  }, [open, worldId, generation, active]);

  const options = Object.entries(basinId ? pondMetrics : forestMetrics).filter(([key]) => key !== 'springtails' || springtails);
  const [key, selected] = options.find(([name]) => name === metric) ?? options[0];
  const values: TrendValue[] = (history?.[range] ?? []).map(({ simulatedMillis: time, reading }) => {
    if (basinId && reading.theme === 'pond') {
      const pondKey = pondMetrics[key as keyof typeof pondMetrics].key;
      return { time, value: reading.metrics.find(({ id }) => id === basinId)?.[pondKey] ?? null };
    }
    if (!basinId && reading.theme === 'forest') {
      const forestKey = forestMetrics[key as keyof typeof forestMetrics].key;
      return { time, value: reading.metrics[forestKey] };
    }
    return { time, value: null };
  });
  const available = values.filter((point): point is { time: number; value: number } => point.value !== null);
  const latest = values.at(-1);
  const valueLabel = (value: number) => `${value.toFixed(selected.decimals)} ${selected.unit}`;
  const note = error ? error : !native ? 'Windows 客户端提供模拟记录。' : !history ? '正在读取观察记录…'
    : !available.length ? '这段时间没有该指标读数。' : available.length === 1 ? '已记录第一个读数，继续运行后可查看变化。' : '';

  return <details className="editor-context__section editor-details observation-trends" data-observation-trends
    onToggle={(event) => setOpen(event.currentTarget.open)}>
    <summary>观察记录与趋势</summary>
    {open && <div className="observation-trends__body">
      <p className="editor-context__hint">{trial ? '试运行副本' : basinLabel ?? '整个林下世界'} · 每 {periods[range].interval}模拟时间记录一次</p>
      <div className="observation-trends__controls">
        <label>时间范围
          <select data-trend-range value={range} onChange={(event) => setRange(event.currentTarget.value as Range)}>
            {Object.entries(periods).map(([id, period]) => <option key={id} value={id}>{period.label}</option>)}
          </select>
        </label>
        <label>指标
          <select data-trend-metric value={key} onChange={(event) => setMetric(event.currentTarget.value)}>
            {options.map(([id, option]) => <option key={id} value={id}>{option.label}</option>)}
          </select>
        </label>
      </div>
      {latest && <p className="observation-trends__latest" data-trend-samples={values.length} data-trend-points={available.length}>
        <span>最近记录 · {simulationTime(latest.time)}</span>
        <strong data-trend-latest={latest.value ?? undefined}>{latest.value === null ? '无读数' : valueLabel(latest.value)}</strong>
      </p>}
      {available.length > 0 && <>
        <TrendPlot values={values} label={`${selected.label}趋势，单位${selected.unit}；数值见下方记录。`} />
        <p className="observation-trends__axis"><span>{simulationTime(values[0].time)}</span><span>{simulationTime(values.at(-1)!.time)}</span></p>
        {available.length > 1 && <p className="editor-context__hint">范围 {valueLabel(Math.min(...available.map(({ value }) => value)))} – {valueLabel(Math.max(...available.map(({ value }) => value)))}</p>}
      </>}
      {note && <p className="editor-context__hint" role="status">{note}</p>}
      {basinId && key === 'oxygen' && <p className="editor-context__hint">干涸时没有溶氧浓度读数，曲线会断开。</p>}
      {values.length > 0 && <details className="observation-trends__records">
        <summary>查看记录（{values.length}）</summary>
        <div className="observation-trends__table"><table>
          <thead><tr><th scope="col">累计模拟时间</th><th scope="col">{selected.label}（{selected.unit}）</th></tr></thead>
          <tbody>{[...values].reverse().map(({ time, value }) => <tr key={time}>
            <td>{simulationTime(time)}</td><td>{value === null ? '无读数' : value.toFixed(selected.decimals)}</td>
          </tr>)}</tbody>
        </table></div>
      </details>}
    </div>}
  </details>;
}
