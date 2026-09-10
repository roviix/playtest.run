// 没有封面的作品用一块按 slug 定色的色田（DESIGN §3.9、§3.13）。
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
