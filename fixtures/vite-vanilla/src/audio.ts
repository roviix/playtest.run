// 门禁页「开始」那一下要能解锁声音，这里用最小的一段 WebAudio 试一次：
// 点按钮 → 建 AudioContext → 响一声 → 把 state 显示出来。
export function setupBeep(button: HTMLButtonElement, output: HTMLElement) {
  let ctx: AudioContext | null = null

  button.addEventListener('click', () => {
    const Ctor = window.AudioContext || (window as unknown as Record<string, typeof AudioContext>).webkitAudioContext
    if (!Ctor) {
      output.textContent = '这个浏览器没有 AudioContext'
      return
    }

    if (!ctx) ctx = new Ctor()
    const before = ctx.state
    void ctx.resume()

    const osc = ctx.createOscillator()
    const gain = ctx.createGain()
    const t = ctx.currentTime
    osc.type = 'sine'
    osc.frequency.value = 660
    // 直接 0 起步会有咔哒声，用一小段淡入淡出包住。
    gain.gain.setValueAtTime(0.0001, t)
    gain.gain.exponentialRampToValueAtTime(0.2, t + 0.01)
    gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.25)
    osc.connect(gain)
    gain.connect(ctx.destination)
    osc.start(t)
    osc.stop(t + 0.26)

    output.textContent = `AudioContext.state: ${before} → ${ctx.state}（${ctx.sampleRate} Hz）`
  })

  output.textContent = 'AudioContext.state: 还没创建'
}
