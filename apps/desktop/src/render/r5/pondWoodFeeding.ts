import type { PondBasinSnapshot, PondWoodContactFeeding } from '../../world';
import { pondFeedingAmountsMatch } from './pondFeedingAmounts';

function foodSources({ biofilmMgC, woodBiofilmMgC, sharedBiofilmMgC }: PondWoodContactFeeding) {
  if (woodBiofilmMgC === undefined || sharedBiofilmMgC === undefined
    || !pondFeedingAmountsMatch(biofilmMgC, woodBiofilmMgC, sharedBiofilmMgC)) return;
  return { woodBiofilmMgC, sharedBiofilmMgC };
}

export function currentPondWoodFeeding(basin: PondBasinSnapshot) {
  return basin.wet ? basin.woodContactFeeding?.filter((event) =>
    Number.isFinite(event.biofilmMgC) && event.biofilmMgC > 0
    && basin.woods?.some(({ id }) => id === event.woodId)
    && basin.shrimp.some(({ id }) => id === event.shrimpId)
    && basin.shrimpAttachments?.some(({ shrimpId, woodId, offset }) =>
      shrimpId === event.shrimpId && woodId === event.woodId && offset === event.offset))
    .map(event => ({ ...event, sources: foodSources(event) })) ?? [] : [];
}
