import type { ITheme } from '@xterm/xterm';
import type { Phosphor } from '../stores/theme';

const PALETTES: Record<Phosphor, { fg: string; dim: string; accent: string }> = {
  green: { fg: '#4dff88', dim: '#1f8c47', accent: '#b6ffd0' },
  amber: { fg: '#ffb000', dim: '#8a5f00', accent: '#ffd98a' },
  red: { fg: '#ff4d4d', dim: '#8c1f1f', accent: '#ffb3b3' },
};

/** ANSI colors stay distinguishable (ls/diff/prompts) while the base tint follows the phosphor. */
export const ansiPalette = {
  black: '#050806',
  red: '#ff5c5c',
  green: '#4dff88',
  yellow: '#ffd24d',
  blue: '#5c9dff',
  magenta: '#d95cff',
  cyan: '#4dffe6',
  white: '#d8e6dc',
};

export function phosphorToXtermTheme(p: Phosphor): ITheme {
  const c = PALETTES[p];
  return {
    background: 'rgba(0,0,0,0)',
    foreground: c.fg,
    cursor: c.accent,
    cursorAccent: '#050806',
    selectionBackground: c.dim,
    ...ansiPalette,
  };
}
