import { useState } from 'react';
import { R5SampleScene, type R5SampleDensity, type R5SampleFocus, type R5SampleZoom } from '../../render/r5/R5SampleScene';
import {
  R5DensityControl, R5ObservationControls, R5PreviewHeader, R5ReferenceNote, useR5PreviewTheme, type R5PreviewProps,
} from './R5PreviewControls';
import './R5Preview.css';

export function R5Observation(props: R5PreviewProps) {
  const { theme, selectTheme } = useR5PreviewTheme(props);
  const [zoom, setZoom] = useState<R5SampleZoom>(1);
  const [focus, setFocus] = useState<R5SampleFocus>('center');
  const [density, setDensity] = useState<R5SampleDensity>(480);
  const [showScale, setShowScale] = useState(false);

  return (
    <main className="r5-frame r5-observation">
      <R5PreviewHeader title="生态观察" theme={theme} onThemeChange={selectTheme} onHide={props.onHide} />
      <div className="r5-scene-area">
        <R5SampleScene className="r5-sample-view" {...{ theme, density, zoom, focus, showScale }} />
      </div>
      <div className="r5-viewport__controls">
        <R5ObservationControls {...{ zoom, focus, showScale }} onZoomChange={setZoom} onFocusChange={setFocus}
          onScaleChange={setShowScale} />
        <R5DensityControl value={density} onChange={setDensity} />
      </div>
      <footer className="r5-footer">
        <R5ReferenceNote theme={theme} density={density}><span>姿态样板</span></R5ReferenceNote>
      </footer>
    </main>
  );
}
