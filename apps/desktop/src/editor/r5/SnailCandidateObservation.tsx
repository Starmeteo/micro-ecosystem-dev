import { useEffect, useRef, useState } from 'react';
import forestAtlas from '../../render/assets/r5/snails/land-snail.png';
import pondAtlas from '../../render/assets/r5/snails/freshwater-snail.png';
import './SnailCandidateObservation.css';

type Pose = 'extended' | 'withdrawn';
type Theme = 'forest' | 'pond';
type Cel = {
  crop: readonly [number, number, number, number];
  shell: readonly [number, number];
};

const candidates = {
  forest: {
    name: '小型陆生蜗牛',
    reference: 'Discus rotundatus',
    habitat: '形态参考来自潮湿林下，常见于落叶和石块下。',
    shellWidth: 6,
    bodyLength: 10,
    atlas: forestAtlas,
    poses: {
      extended: { crop: [84, 374, 703, 242], shell: [88, 605] },
      withdrawn: { crop: [1097, 376, 526, 240], shell: [1101, 1618] },
    },
  },
  pond: {
    name: '小型淡水螺',
    reference: 'Planorbella scalaris',
    habitat: '形态参考来自佛罗里达淡水螺鉴定图谱，壳顶抬高且平，壳口近竖直。',
    shellWidth: 8,
    bodyLength: 12,
    atlas: pondAtlas,
    poses: {
      extended: { crop: [174, 240, 759, 340], shell: [178, 714] },
      withdrawn: { crop: [1320, 240, 535, 330], shell: [1324, 1850] },
    },
  },
} satisfies Record<Theme, {
  name: string; reference: string; habitat: string; shellWidth: number; bodyLength: number;
  atlas: string; poses: Record<Pose, Cel>;
}>;

const drawingWidth = 200;
const drawingHeight = 108;
const shellCenter = 95;
const unitsPerMm = 10;

function SnailProfile({ theme, pose }: { theme: Theme; pose: Pose }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [failed, setFailed] = useState(false);
  const candidate = candidates[theme];

  useEffect(() => {
    const frame = canvas.current;
    const ctx = frame?.getContext('2d');
    if (!frame || !ctx) return;
    ctx.setTransform(2, 0, 0, 2, 0, 0);
    ctx.clearRect(0, 0, drawingWidth, drawingHeight);
    const [sx, sy, sw, sh] = candidate.poses[pose].crop;
    const [left, right] = candidate.poses[pose].shell;
    const shellWidth = candidate.shellWidth * unitsPerMm;
    const from = shellCenter - shellWidth / 2;
    const to = shellCenter + shellWidth / 2;
    const scale = shellWidth / (right - left);
    const beforeShell = (right - sx) * scale;
    const totalWidth = pose === 'extended' ? candidate.bodyLength * unitsPerMm : sw * scale;
    const x = to - beforeShell;
    const height = sh * scale;
    const y = 78 - height;
    const image = new Image();
    image.onload = () => {
      ctx.imageSmoothingEnabled = false;
      // Keep both poses on the same shell scale, then fit the exposed head and foot to their pose length.
      ctx.drawImage(image, sx, sy, right - sx, sh, x, y, beforeShell, height);
      ctx.drawImage(image, right, sy, sx + sw - right, sh, to, y, totalWidth - beforeShell, height);
      ctx.strokeStyle = '#d8edf0';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(from, 84); ctx.lineTo(from, 92); ctx.lineTo(to, 92); ctx.lineTo(to, 84);
      ctx.stroke();
      ctx.fillStyle = '#d8edf0';
      ctx.textAlign = 'center';
      ctx.font = '10px system-ui';
      ctx.fillText(`${candidate.shellWidth} mm`, shellCenter, 104);
      frame.dataset.ready = 'true';
    };
    image.onerror = () => setFailed(true);
    image.src = candidate.atlas;
    return () => { image.onload = null; image.onerror = null; delete frame.dataset.ready; };
  }, [candidate, pose]);

  return failed ? <p role="status">形态图加载失败。</p> :
    <canvas ref={canvas} width={2 * drawingWidth} height={2 * drawingHeight} role="img"
      aria-label={`${candidate.name}${pose === 'extended' ? '伸展' : '缩壳'}放大图，壳宽参考 ${candidate.shellWidth} 毫米。`}
      data-snail-shell-mm={candidate.shellWidth} data-snail-pixels-per-mm={unitsPerMm} />;
}

export function SnailCandidateObservation({ theme }: { theme: Theme }) {
  const [open, setOpen] = useState(false);
  const [pose, setPose] = useState<Pose>('extended');
  const profile = useRef<HTMLDivElement>(null);
  const candidate = candidates[theme];

  useEffect(() => {
    if (open) profile.current?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }, [open]);

  return <section className="editor-context__section" data-snail-candidate={theme}>
    <h2>{candidate.name}</h2>
    <details className="editor-details" onToggle={(event) => setOpen(event.currentTarget.open)}>
      <summary>候选预览</summary>
      {open && <div ref={profile} className={`snail-candidate-profile snail-candidate-profile--${theme}`}>
        <div className="snail-candidate-poses" role="group" aria-label="候选姿态">
          <button className="editor-tool" type="button" aria-pressed={pose === 'extended'} onClick={() => setPose('extended')}>伸展</button>
          <button className="editor-tool" type="button" aria-pressed={pose === 'withdrawn'} onClick={() => setPose('withdrawn')}>缩壳</button>
        </div>
        <SnailProfile key={pose} theme={theme} pose={pose} />
        <p>同尺放大示意。壳宽 {candidate.shellWidth} mm，伸展参考长 {candidate.bodyLength} mm。</p>
        <p>形态参考 <i lang="la">{candidate.reference}</i>。{candidate.habitat}</p>
        <p>此候选仅供观察，暂未开放投放。</p>
      </div>}
    </details>
  </section>;
}
