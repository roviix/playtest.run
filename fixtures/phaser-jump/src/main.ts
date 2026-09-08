import Phaser from 'phaser'
import { blip } from './audio.ts'

const WIDTH = 480
const HEIGHT = 720
const GROUND_TOP = HEIGHT - 96

class JumpScene extends Phaser.Scene {
  private player!: Phaser.Physics.Arcade.Sprite
  private status!: Phaser.GameObjects.Text
  private jumps = 0

  constructor() {
    super('jump')
  }

  // 整个 fixture 不带任何图片和音频文件，两张纹理都画出来。
  private drawTextures() {
    const g = this.add.graphics()

    g.fillStyle(0xffd166, 1).fillRoundedRect(0, 0, 48, 48, 10)
    g.fillStyle(0x10131a, 1).fillRect(13, 17, 6, 8).fillRect(29, 17, 6, 8)
    g.generateTexture('player', 48, 48)

    g.clear()
    g.fillStyle(0x2f3b52, 1).fillRect(0, 0, WIDTH, 24)
    g.fillStyle(0x4a5a7a, 1).fillRect(0, 0, WIDTH, 4)
    g.generateTexture('ground', WIDTH, 24)

    g.destroy()
  }

  create() {
    this.drawTextures()

    this.add
      .text(WIDTH / 2, 96, 'phaser-jump', { fontFamily: 'sans-serif', fontSize: '34px', color: '#e7ecf5' })
      .setOrigin(0.5)
    this.add
      .text(WIDTH / 2, 138, '点一下（或按空格）跳一下', {
        fontFamily: 'sans-serif',
        fontSize: '18px',
        color: '#8b97ad',
      })
      .setOrigin(0.5)

    this.status = this.add
      .text(WIDTH / 2, HEIGHT - 40, '', { fontFamily: 'sans-serif', fontSize: '16px', color: '#8b97ad' })
      .setOrigin(0.5)

    const ground = this.physics.add.staticImage(WIDTH / 2, GROUND_TOP + 12, 'ground')

    this.player = this.physics.add.sprite(WIDTH / 2, GROUND_TOP - 200, 'player')
    this.player.setBounce(0.05)
    this.player.setCollideWorldBounds(true)
    this.physics.add.collider(this.player, ground)

    this.input.on('pointerdown', () => this.jump())
    this.input.keyboard?.on('keydown-SPACE', () => this.jump())
    this.input.keyboard?.on('keydown-UP', () => this.jump())

    this.report('还没跳过')
  }

  private jump() {
    const body = this.player.body as Phaser.Physics.Arcade.Body
    if (!body.blocked.down && !body.touching.down) return

    body.setVelocityY(-820)
    this.jumps += 1

    // 起跳时压扁一下再弹回，没有这一下会显得很木。
    this.player.setScale(0.82, 1.18)
    this.tweens.add({ targets: this.player, scaleX: 1, scaleY: 1, duration: 180, ease: 'Back.easeOut' })

    this.report(blip())
  }

  private report(audioState: string) {
    this.status.setText(`跳了 ${this.jumps} 次 · AudioContext.state: ${audioState}`)
  }
}

new Phaser.Game({
  type: Phaser.AUTO,
  parent: 'game',
  width: WIDTH,
  height: HEIGHT,
  backgroundColor: '#10131a',
  // 手机竖屏、桌面大窗口都靠这两行等比铺满并居中。
  scale: { mode: Phaser.Scale.FIT, autoCenter: Phaser.Scale.CENTER_BOTH },
  input: { activePointers: 2 },
  // 声音走 src/audio.ts 里自己的 AudioContext，不用 Phaser 的声音管理器。
  audio: { noAudio: true },
  physics: { default: 'arcade', arcade: { gravity: { x: 0, y: 1800 } } },
  scene: [JumpScene],
})
