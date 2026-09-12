const { readFileSync } = require('node:fs');
const { resolve } = require('node:path');
const { runInNewContext } = require('node:vm');
const { test } = require('node:test');
const assert = require('node:assert/strict');

const root = resolve(__dirname, '../..');
const html = readFileSync(resolve(root, 'docs/avatar-concept.html'), 'utf8');
const embedded = html.match(/<script type="application\/json" id="avatar-assets">([\s\S]*?)<\/script>/)[1];
const source = html.match(/<script>\s*'use strict';([\s\S]*?)<\/script>/)[1].split('let config=')[0];
const engine = runInNewContext(`${source};({VERSION, AXES, PALETTES, derive, fingerprint, svgFor, drawAccessory, drawCreature, compatibleTraits, activeAxis, hash32})`, {
  TextEncoder,
  document: { getElementById: () => ({ textContent: embedded }) },
});

test('preview embeds the exact production artwork catalog', () => {
  assert.deepEqual(JSON.parse(embedded), JSON.parse(readFileSync(resolve(root, 'common/src/avatar-assets.json'), 'utf8')));
});

test('UTF-8 seeds and signatures match the shared Rust vectors', () => {
  const vectors = JSON.parse(readFileSync(resolve(__dirname, 'avatar-vectors.json'), 'utf8'));
  for (const vector of vectors) {
    const traits = engine.derive({ seed: vector.seed, overrides: {} });
    assert.deepEqual(JSON.parse(JSON.stringify(traits)), vector.traits, vector.seed);
    assert.equal(engine.fingerprint(traits), vector.fingerprint, vector.seed);
  }
});

test('production and preview agree on every weight and palette', () => {
  const rust = readFileSync(resolve(root, 'common/src/avatar.rs'), 'utf8');
  for (const [axis, spec] of Object.entries(engine.AXES)) {
    const declaration = rust.match(new RegExp(`AXES_${axis.toUpperCase()}_WEIGHTS:[^=]+=[\\s]*\\[([^\\]]+)\\]`));
    assert.ok(declaration, axis);
    const weights = declaration[1].split(',').map(value => value.trim()).filter(Boolean).map(Number);
    assert.deepEqual(weights, Array.from(spec.weights), axis);
  }
  const paletteSource = rust.split('pub const PALETTES:')[1].split('];')[0];
  const palettes = Array.from(paletteSource.matchAll(/Palette\s*\{([^}]+)\}/g), match =>
    Object.fromEntries(Array.from(match[1].matchAll(/(\w+):\s*"(#[a-f0-9]+)"/g), field => [field[1], field[2]])));
  assert.deepEqual(palettes, JSON.parse(JSON.stringify(engine.PALETTES)));
});

test('all palette and material selections render on all ten silhouettes', () => {
  const assets = JSON.parse(embedded);
  const bases = { hat: 6, outfit: 5, eyes: 5, accessory: 5, background: 4 };
  for (const [axis, spec] of Object.entries(engine.AXES)) {
    assert.equal(spec.names.length, spec.weights.length);
    assert.equal(spec.weights.reduce((sum, weight) => sum + weight, 0), 100);
    if (axis in bases) assert.deepEqual(Array.from(spec.names).slice(bases[axis]), assets[axis].map(asset => asset.name));
    for (let species = 0; species < engine.AXES.species.names.length; species++) {
      for (let value = 0; value < spec.names.length; value++) {
        const svg = engine.svgFor({ seed: 'material-audit', overrides: { species, [axis]: value } }, 'audit');
        assert.ok(svg.startsWith('<svg') && svg.endsWith('</svg>'));
        assert.doesNotMatch(svg, /undefined|NaN|\{(?:skin|coat|ink|shade|trim|accent|light)\}|<text/);
        assert.doesNotMatch(svg, /<script|<image|<foreignObject|<filter|href=/i);
      }
    }
  }
  assert.equal(engine.PALETTES.length, engine.AXES.palette.names.length);
  assert.equal(engine.VERSION, 'odd-folk/3.0.0');
});

test('earrings and bandages follow the tilted face rather than the body', () => {
  for (const accessory of [1, 2, 5, 6, 7]) {
    const config = { seed: 'layer-audit', overrides: { species: 0, accessory } };
    const traits = engine.derive(config);
    const layer = engine.drawAccessory(traits, engine.PALETTES[traits.palette]);
    const svg = engine.svgFor(config, 'audit');
    assert.ok(svg.includes(accessory <= 2 ? `${layer}</g></g></svg>` : `</g>${layer}</g></svg>`));
  }
});

test('native creatures have independent artwork and no invisible wardrobe DNA', () => {
  const creatures = JSON.parse(embedded).creatures;
  assert.equal(creatures.length, 4);
  assert.equal(new Set(creatures.map(creature => creature.body)).size, 4);
  for (let species = 6; species < 10; species++) {
    const config = { seed: 'native', overrides: { species, palette: 0, eyes: 1, hat: 0, outfit: 0, accessory: 0 } };
    const original = engine.svgFor(config, 'audit');
    const changed = { ...config, overrides: { ...config.overrides, hat: 9, outfit: 7, accessory: 7, eyes: 7 } };
    assert.equal(engine.svgFor(changed, 'audit'), original);
    assert.equal(engine.fingerprint(engine.derive(changed)), engine.fingerprint(engine.derive(config)));
    for (const axis of ['hat', 'outfit', 'accessory']) assert.equal(engine.activeAxis(engine.derive(config), axis), false);
    const creature = creatures[species - 6];
    assert.equal(creature.eyes.length, 3);
    assert.equal(creature.mouths.length, 4);
    for (let palette = 0; palette < engine.PALETTES.length; palette++) {
      const seen = new Set();
      for (let eyes = 0; eyes < 3; eyes++) {
        for (let mood = 0; mood < 4; mood++) {
          const svg = engine.svgFor({ seed: 'native', overrides: { species, palette, eyes, mood } }, 'audit');
          assert.ok(svg.includes(`data-creature="${creature.name}"`));
          assert.doesNotMatch(svg, /undefined|NaN|\{\w+\}|rotate\(-4 335 370\)/);
          const ids = new Set(Array.from(svg.matchAll(/\bid="([^"]+)"/g), match => match[1]));
          for (const match of svg.matchAll(/url\(#([^)]+)\)/g)) assert.ok(ids.has(match[1]), match[1]);
          seen.add(svg);
        }
      }
      assert.equal(seen.size, 12);
    }
  }
});

test('native render layers match Rust golden artwork hashes', () => {
  const vectors = JSON.parse(readFileSync(resolve(__dirname, 'avatar-vectors.json'), 'utf8'));
  for (const vector of vectors.filter(vector => vector.creature_hash !== undefined)) {
    const traits = engine.derive({ seed: vector.seed, overrides: {} });
    assert.equal(engine.hash32(engine.drawCreature(traits, engine.PALETTES[traits.palette], 'audit')), vector.creature_hash);
  }
});
