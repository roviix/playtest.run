// 跳跃音效。不加载音频文件，直接用 WebAudio 合成，
// 顺便验证「玩家第一次点屏幕」这一下手势能不能解锁声音。
let ctx: AudioContext | null = null

export function blip(): string {
  const Ctor = window.AudioContext || (window as unknown as Record<string, typeof AudioContext>).webkitAudioContext
  if (!Ctor) return '这个浏览器没有 AudioContext'

  if (!ctx) ctx = new Ctor()
  void ctx.resume()

  const t = ctx.currentTime
  const osc = ctx.createOscillator()
  const gain = ctx.createGain()
  osc.type = 'square'
  osc.frequency.setValueAtTime(420, t)
  osc.frequency.exponentialRampToValueAtTime(880, t + 0.09)
  gain.gain.setValueAtTime(0.0001, t)
  gain.gain.exponentialRampToValueAtTime(0.12, t + 0.01)
  gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.18)
  osc.connect(gain)
  gain.connect(ctx.destination)
  osc.start(t)
  osc.stop(t + 0.19)

  return ctx.state
}
