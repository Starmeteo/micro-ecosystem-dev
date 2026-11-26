import { useState, type ReactNode } from 'react';
import type { R5SampleDensity, R5SampleFocus, R5SampleTheme, R5SampleZoom } from '../../render/r5/R5SampleScene';

export interface R5PreviewProps {
  onHide: () => void;
  initialTheme?: R5SampleTheme;
  theme?: R5SampleTheme;
  onDesktopThemeChange?: (theme: R5SampleTheme) => void;
}

interface PreviewHeaderProps {
  title: string;
  theme: R5SampleTheme;
  onThemeChange: (theme: R5SampleTheme) => void;
  onHide: () => void;
}

const themes = [
  { id: 'forest', label: '林下' },
  { id: 'pond', label: '浅塘' },
] as const;

export function useR5PreviewTheme({ theme, initialTheme = 'forest', onDesktopThemeChange }: R5PreviewProps) {
  const [localTheme, setLocalTheme] = useState(initialTheme);
  const activeTheme = theme ?? localTheme;
  return {
    theme: activeTheme,
    selectTheme: (next: R5SampleTheme) => {
      if (next === activeTheme) return;
      setLocalTheme(next);
      onDesktopThemeChange?.(next);
    },
  };
}

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

export function R5PreviewHeader({ title, theme, onThemeChange, onHide }: PreviewHeaderProps) {
  return (
    <header className="r5-titlebar" data-tauri-drag-region>
      <div className="r5-titlebar__identity" data-tauri-drag-region>
        <h1 data-tauri-drag-region>{title}</h1>
        <span className="r5-titlebar__tag" data-tauri-drag-region>R5 样板</span>
      </div>
      <div className="r5-titlebar__tools">
        <Segments label="预览主题" choices={themes} value={theme} onChange={onThemeChange} />
        <button className="r5-button r5-hide" type="button" aria-label="收起预览" onClick={onHide}>收起</button>
      </div>
    </header>
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

export function R5DensityControl({ value, onChange }: {
  value: R5SampleDensity;
  onChange: (density: R5SampleDensity) => void;
}) {
  return (
    <label className="r5-density">
      <span>逻辑栅格</span>
      <select aria-label="视图逻辑栅格" value={value} onChange={(event) => {
        const densities = { '240': 240, '480': 480, '960': 960 } as const;
        const density = densities[event.target.value as keyof typeof densities];
        if (density) onChange(density);
      }}>
        <option value="240">240 × 135</option>
        <option value="480">480 × 270</option>
        <option value="960">960 × 540</option>
      </select>
    </label>
  );
}

export function R5ReferenceNote({ theme, density, children }: {
  theme: R5SampleTheme;
  density: R5SampleDensity;
  children?: ReactNode;
}) {
  return (
    <div className="r5-reference">
      <span>{theme === 'forest' ? '林下参考画宽 450 mm' : '浅塘参考画宽 600 mm'}</span>
      <span>{density} × {density * 9 / 16} 逻辑像素{density === 960 ? ' · 480 素材采样' : ''}</span>
      {children}
    </div>
  );
}
