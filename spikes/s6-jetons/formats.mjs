// Formats personnalisés Style Dictionary 5.8 : Rust (ratatui), CSS, Dart.
// Convention : les modes vivent sous le groupe racine `mode.<nom>` du fichier DTCG ;
// les primitives sous `color.*`. Les alias sont déjà résolus par Style Dictionary.
const HEADER = 'GÉNÉRÉ par Style Dictionary depuis tokens/*.tokens.json — NE PAS ÉDITER.';

const hex = (t) => {
  const h = String(t.$value).replace('#', '').toLowerCase();
  if (!/^[0-9a-f]{6}([0-9a-f]{2})?$/.test(h)) throw new Error(`${t.path.join('.')} : couleur non convertie en hex (${JSON.stringify(t.$value)})`);
  if (h.length === 8 && !h.endsWith('ff')) throw new Error(`${t.path.join('.')} : alpha non supporté par les sorties (TUI/Dart sans transparence)`);
  return h.slice(0, 6);
};
const rgb = (t) => [0, 2, 4].map((i) => parseInt(hex(t).slice(i, i + 2), 16));
const modeTokens = (dictionary) => {
  const modes = new Map();
  for (const t of dictionary.allTokens) {
    if (t.path[0] !== 'mode') continue;
    const [, mode, name] = t.path;
    if (!modes.has(mode)) modes.set(mode, new Map());
    modes.get(mode).set(name, t);
  }
  return modes;
};
const MODS = { bold: 'BOLD', italic: 'ITALIC', underline: 'UNDERLINED', reversed: 'REVERSED', dim: 'DIM' };
const mods = (t) => {
  const m = t.$extensions?.['atelier.tui']?.modifiers ?? [];
  for (const x of m) if (!MODS[x]) throw new Error(`modificateur TUI inconnu : ${x}`);
  return m.length ? m.map((x) => `Modifier::${MODS[x]}`).join('.union(').concat(')'.repeat(m.length - 1)) : 'Modifier::empty()';
};
const snake = (s) => s.replace(/-/g, '_');
const camel = (s) => s.replace(/-(\w)/g, (_, c) => c.toUpperCase());

export const rustFormat = ({ dictionary }) => {
  const modes = modeTokens(dictionary);
  const names = [...[...modes.values()][0].keys()];
  const colorNames = names.filter((n) => modes.values().next().value.get(n).$type === 'color');
  const modTokens = names.filter((n) => modes.values().next().value.get(n).$type !== 'color');
  let o = `// ${HEADER}\n#![allow(missing_docs)]\n\nuse ratatui::style::{Color, Modifier};\n\n`;
  o += '/// Palette sémantique d\'un mode (couleurs truecolor + modificateurs).\n';
  o += '#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct Palette {\n';
  for (const n of colorNames) o += `    pub ${snake(n)}: Color,\n`;
  for (const n of modTokens) o += `    pub ${snake(n)}: Modifier,\n`;
  o += '}\n\n';
  for (const [mode, toks] of modes) {
    const M = mode.toUpperCase();
    for (const n of colorNames) {
      const [r, g, b] = rgb(toks.get(n));
      o += `pub const ${M}_${snake(n).toUpperCase()}: Color = Color::Rgb(${r}, ${g}, ${b});\n`;
    }
    o += `\npub const ${M}: Palette = Palette {\n`;
    for (const n of colorNames) o += `    ${snake(n)}: ${M}_${snake(n).toUpperCase()},\n`;
    for (const n of modTokens) o += `    ${snake(n)}: ${mods(toks.get(n))},\n`;
    o += '};\n\n';
  }
  return o.trimEnd() + '\n';
};

export const cssFormat = ({ dictionary }) => {
  const modes = modeTokens(dictionary);
  let o = `/* ${HEADER} */\n:root {\n`;
  for (const t of dictionary.allTokens.filter((t) => t.path[0] === 'color')) {
    o += `  --color-${t.path.slice(1).join('-')}: #${hex(t)};\n`;
  }
  o += '}\n';
  for (const [mode, toks] of modes) {
    const sel = mode === 'light' ? ':root' : `[data-theme='${mode}']`;
    o += `\n${sel} {\n`;
    for (const [n, t] of toks) {
      if (t.$type === 'color') o += `  --${n}: #${hex(t)};\n`;
      else if (t.$type === 'fontWeight') o += `  --${n}-weight: ${t.$value};\n`;
    }
    o += '}\n';
  }
  return o;
};

export const dartFormat = ({ dictionary }) => {
  const modes = modeTokens(dictionary);
  let o = `// ${HEADER}\nimport 'dart:ui' show Color;\n`;
  for (const [mode, toks] of modes) {
    const C = mode[0].toUpperCase() + mode.slice(1);
    o += `\nabstract final class AtelierPalette${C} {\n`;
    for (const [n, t] of toks) {
      if (t.$type === 'color') o += `  static const Color ${camel(n)} = Color(0xFF${hex(t).toUpperCase()});\n`;
    }
    o += '}\n';
  }
  return o;
};
