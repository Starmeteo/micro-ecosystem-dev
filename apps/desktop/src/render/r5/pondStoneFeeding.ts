import type { PondBasinSnapshot } from '../../world';
import { pondMainStoneRestPoint, stoneWaterHeight } from './pondStoneGeometry';
import { pondFeedingAmountsMatch } from './pondFeedingAmounts';

export function currentPondStoneContacts(basin: PondBasinSnapshot) {
  if (!basin.wet || basin.stoneRestEnabled !== true) return [];
  const column = stoneWaterHeight(basin);
  const contacts = (basin.stoneAttachments ?? []).filter(({ shrimpId, stoneId, offset, restTicks }) => {
    if (!Number.isInteger(restTicks) || restTicks <= 0 || basin.shrimpAttachments?.some(wood => wood.shrimpId === shrimpId)) return false;
    const stone = basin.stones?.find(({ id }) => id === stoneId);
    const shrimp = basin.shrimp.find(({ id }) => id === shrimpId);
    const at = stone && pondMainStoneRestPoint(stone, offset, column);
    return Boolean(at && shrimp?.activity === 'resting' && Math.abs(shrimp.x - at.x) <= 1e-8 && Math.abs(shrimp.y - at.y) <= 1e-8);
  });
  return contacts.filter(({ shrimpId }) => contacts.filter(contact => contact.shrimpId === shrimpId).length === 1);
}

export function currentPondStoneFeeding(basin: PondBasinSnapshot, contacts = currentPondStoneContacts(basin)) {
  const feeding = (basin.stoneContactFeeding ?? []).filter(({ shrimpId, stoneId, offset, biofilmMgC, stoneBiofilmMgC, sharedBiofilmMgC }) =>
    pondFeedingAmountsMatch(biofilmMgC, stoneBiofilmMgC, sharedBiofilmMgC) && contacts.some((contact) =>
      contact.shrimpId === shrimpId && contact.stoneId === stoneId && contact.offset === offset));
  return feeding.filter(({ shrimpId }) => feeding.filter(event => event.shrimpId === shrimpId).length === 1);
}
