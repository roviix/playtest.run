// 没有封面的作品是一张字卡：按 slug 定色的色田，左下角作品名的第一个字排成花押，右上角 slug（DESIGN §3.9、§3.13）。
//
// 算法和边缘 edge/src/html.rs 的 `hue()` 是同一个 FNV-1a：同一个 slug 在广场、门禁页和
// 控制台上是同一种颜色，开发者在三个地方认的是同一件东西。

export function hue(slug: string): number {
  let h = 2166136261;
  for (const byte of new TextEncoder().encode(slug)) {
    h = (h ^ byte) >>> 0;
    h = Math.imul(h, 16777619) >>> 0;
  }
  return h % 360;
}

/** 花押：作品名的第一个字；名字是空的就退到 slug 的第一个字母。和边缘 plaza.rs 的 `monogram()` 同一条规则。 */
export function monogram(title: string, slug: string): string {
  const first = Array.from(title.trim())[0];
  return first ?? Array.from(slug)[0] ?? "";
}
