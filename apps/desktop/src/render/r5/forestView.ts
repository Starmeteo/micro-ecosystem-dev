import type { BarkShelterPreset, ForestTerrainKind, LeafPatchPreset, ScenePoint, StoneworkPreset } from '../../world';
import type { R5SampleFocus, R5SampleZoom } from './geometry';
import type { ForestSpringtailView } from './forestSpringtails';

export type R5ForestEdit = {
  mode: 'move' | 'add';
  addKind?: ForestTerrainKind;
  stoneworkPreset?: StoneworkPreset;
  barkPreset?: BarkShelterPreset;
  leafPreset?: LeafPatchPreset;
  selectedId?: number | null;
  onSelect: (id: number | null) => void;
  onMove: (id: number, position: ScenePoint) => void;
  onAdd: (position: ScenePoint) => void;
  onCancel?: () => void;
};

export type R5ForestView = {
  zoom?: R5SampleZoom;
  focus?: R5SampleFocus;
  showScale?: boolean;
  springtails?: ForestSpringtailView;
  animalId?: string;
};
