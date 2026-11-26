import type { MatterMgCnp } from '../../world';

const milligrams = new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 3 });
export const nutrients = ['c', 'n', 'p'] as const;

export function displayMass(amount: number) {
  return amount > 0 && amount < .001 ? '<0.001' : milligrams.format(amount);
}

export function matterSummary(matter: MatterMgCnp) {
  return matter.map((amount, index) => `${displayMass(amount)} mg ${nutrients[index].toUpperCase()}`).join(' · ');
}
