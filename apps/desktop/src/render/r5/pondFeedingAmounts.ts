export function pondFeedingAmountsMatch(total: number, local: number | undefined, shared: number | undefined) {
  return Number.isFinite(total) && total > 0 && local !== undefined && shared !== undefined
    && Number.isFinite(local) && local >= 0 && Number.isFinite(shared) && shared >= 0
    && Math.abs(local + shared - total) <= total * Number.EPSILON * 4;
}
