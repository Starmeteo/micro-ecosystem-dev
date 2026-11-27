import { useEffect, useRef } from 'react';
import type { PondBasinSnapshot } from '../../world';
import { matterSummary } from './formatMatter';
import './PondDaphniaObservation.css';

type DaphniaSnapshot = Partial<Pick<PondBasinSnapshot, 'wet' | 'daphniaCount' | 'daphniaStockMgCnp'>>;

function drawSideProfile(canvas: HTMLCanvasElement) {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  ctx.setTransform(2, 0, 0, 2, 0, 0);
  ctx.clearRect(0, 0, 264, 140);
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';

  // Subtle stereomicroscope field ring & reticle ticks
  ctx.strokeStyle = 'rgba(170, 232, 226, 0.11)';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.arc(134, 68, 60, 0, Math.PI * 2);
  ctx.moveTo(12, 68); ctx.lineTo(22, 68);
  ctx.moveTo(242, 68); ctx.lineTo(252, 68);
  ctx.stroke();

  const shell = ctx.createLinearGradient(95, 36, 177, 105);
  shell.addColorStop(0, 'rgba(190, 251, 235, .88)');
  shell.addColorStop(1, 'rgba(75, 183, 178, .26)');
  ctx.beginPath();
  ctx.moveTo(78, 63);
  ctx.bezierCurveTo(84, 45, 101, 33, 124, 34);
  ctx.bezierCurveTo(142, 27, 171, 36, 186, 56);
  ctx.bezierCurveTo(199, 73, 202, 91, 194, 103);
  ctx.bezierCurveTo(181, 118, 152, 118, 130, 107);
  ctx.bezierCurveTo(107, 99, 101, 82, 93, 76);
  ctx.quadraticCurveTo(84, 75, 78, 63);
  ctx.fillStyle = shell;
  ctx.fill();
  ctx.strokeStyle = '#abf5df';
  ctx.lineWidth = 1.3;
  ctx.stroke();

  ctx.beginPath();
  ctx.moveTo(111, 56);
  ctx.bezierCurveTo(124, 67, 131, 80, 146, 91);
  ctx.quadraticCurveTo(162, 103, 181, 96);
  ctx.moveTo(117, 48);
  ctx.bezierCurveTo(152, 39, 181, 58, 188, 85);
  ctx.strokeStyle = 'rgba(23, 84, 87, .65)';
  ctx.lineWidth = 1.05;
  ctx.stroke();

  ctx.beginPath();
  ctx.moveTo(97, 54);
  ctx.quadraticCurveTo(72, 47, 60, 38);
  ctx.lineTo(44, 31);
  ctx.lineTo(26, 16);
  ctx.moveTo(59, 38);
  ctx.lineTo(43, 45);
  ctx.lineTo(24, 42);
  ctx.moveTo(51, 34);
  ctx.lineTo(48, 23);
  ctx.lineTo(36, 12);
  ctx.moveTo(41, 43);
  ctx.lineTo(35, 52);
  ctx.lineTo(25, 55);
  ctx.moveTo(192, 104);
  ctx.quadraticCurveTo(203, 110, 220, 107);
  ctx.strokeStyle = '#b4e7d9';
  ctx.lineWidth = 1.5;
  ctx.stroke();

  ctx.fillStyle = '#152536';
  ctx.beginPath();
  ctx.ellipse(95, 63, 5.6, 5, -.26, 0, Math.PI * 2);
  ctx.fill();

  // The body reference spans 78–198 units, excluding the antennae and tail spine.
  ctx.strokeStyle = '#a2d0d6';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(78, 117); ctx.lineTo(78, 125);
  ctx.lineTo(198, 125); ctx.lineTo(198, 117);
  ctx.stroke();
  ctx.fillStyle = '#d7ebed';
  ctx.font = '10px system-ui';
  ctx.textAlign = 'center';
  ctx.fillText('2 mm', 138, 137);
}

function DaphniaSideProfile() {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => { if (ref.current) drawSideProfile(ref.current); }, []);
  return <figure className="pond-daphnia-profile">
    <canvas ref={ref} width={528} height={280} role="img"
      aria-label="水蚤侧视形态示意，透明卵形壳、眼、分枝触角和尾刺。标尺为躯体参考长 2 毫米。" />
    <figcaption>侧视形态示意。躯体参考长 2 mm，触角和尾刺另计。</figcaption>
  </figure>;
}

export function PondDaphniaObservation({ basin }: { basin?: DaphniaSnapshot }) {
  const rawCount = basin?.daphniaCount;
  const count = typeof rawCount === 'number' && Number.isSafeInteger(rawCount) && rawCount >= 0 ? rawCount : undefined;
  const wet = basin?.wet;
  const points = wet === false ? 0 : wet && count !== undefined ? Math.min(32, Math.ceil(count / 6)) : undefined;
  const stock = basin?.daphniaStockMgCnp;
  const hasStock = stock?.length === 3 && stock.every(value => Number.isFinite(value) && value >= 0);
  const state = count === undefined || wet === undefined ? 'unavailable' : !wet ? 'dry' : count === 0 ? 'empty' : 'present';
  const status = {
    unavailable: '当前快照缺少完整的水蚤观测数据。',
    dry: '当前水域已干涸，画面无水蚤代表点。',
    empty: '当前有水，群体数量为零。',
    present: '代表点用于呈现群体密度。各剖面共用同一群体。',
  };
  return <section className="editor-context__section" data-pond-daphnia-observation={state}>
    <h2>水蚤群</h2>
    <dl className="editor-stock">
      <div><dt>整池数量</dt><dd data-pond-daphnia-count={count}>{count === undefined ? '暂无数据' : `${count} 只`}</dd></div>
      <div><dt>全景代表点</dt><dd data-pond-daphnia-representative-points={points}>{points === undefined ? '暂无数据' : `${points} 点`}</dd></div>
      <div><dt>群体库存</dt><dd data-pond-daphnia-stock={hasStock ? stock.join(',') : undefined}>{hasStock ? matterSummary(stock) : '暂无数据'}</dd></div>
    </dl>
    <p className="editor-context__hint">{status[state]}</p>
    <details className="editor-details pond-daphnia-detail" data-pond-daphnia-magnifier>
      <summary>放大观察</summary>
      <DaphniaSideProfile />
    </details>
  </section>;
}
