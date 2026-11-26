/** Identifies the pixels in a composed canvas backing store. */
export function sameComposedFrame(previous: readonly unknown[] | null, next: readonly unknown[]) {
  return previous?.length === next.length && next.every((value, index) => Object.is(value, previous[index]));
}
