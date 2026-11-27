import type { R5SampleFocus, R5SampleZoom } from '../../render/r5/geometry';
import './R5ObservationControls.css';

function Segments<T extends string | number>({ label, choices, value, onChange }: {
  label: string;
  choices: ReadonlyArray<{ id: T; label: string }>;
  value?: T;
  onChange: (value: T) => void;
}) {
  return (
    <div className="r5-segments" role="group" aria-label={label}>
      {choices.map(({ id, label: text }) => (
        <button key={id} type="button" aria-pressed={value === id} onClick={() => onChange(id)}>
          {text}
        </button>
      ))}
    </div>
  );
}

const zooms = [
  { id: 1, label: '1×' },
  { id: 2, label: '2×' },
  { id: 4, label: '4×' },
] as const;

const focuses = [
  { id: 'center', label: '中央' },
  { id: 'contact', label: '石木' },
  { id: 'animal', label: '生物' },
] as const;

export function R5ObservationControls({ zoom, focus, showScale, onZoomChange, onFocusChange, onScaleChange }: {
  zoom: R5SampleZoom;
  focus?: R5SampleFocus;
  showScale: boolean;
  onZoomChange: (zoom: R5SampleZoom) => void;
  onFocusChange: (focus: R5SampleFocus) => void;
  onScaleChange: (show: boolean) => void;
}) {
  return (
    <div className="r5-view-controls" aria-label="画面观察">
      <Segments label="观察倍率" choices={zooms} value={zoom} onChange={onZoomChange} />
      <Segments
        label="观察位置"
        choices={focuses}
        value={focus}
        onChange={(next) => {
          onFocusChange(next);
          if (zoom === 1) onZoomChange(2);
        }}
      />
      <button className="r5-button" type="button" aria-pressed={showScale} onClick={() => onScaleChange(!showScale)}>
        标尺
      </button>
    </div>
  );
}
