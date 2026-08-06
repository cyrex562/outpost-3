import { describe, expect, it } from 'vitest'

/**
 * Guards the design-token refactor.
 *
 * A light theme only works if components resolve their colours through the
 * custom properties in `src/assets/theme.css`. A single raw `#334` slipped
 * back into a component would look correct in dark mode and be invisible in
 * light — exactly the kind of regression that survives review, because the
 * author is almost always looking at the default theme.
 *
 * Sources are pulled in with `import.meta.glob(..., '?raw')` rather than
 * `node:fs`: this file lives under `src/`, which the app's tsconfig
 * type-checks without Node types, so a filesystem read here would break
 * `npm run build`.
 *
 * The complementary half of this guard — that every token actually resolves,
 * and that light and dark define the same set — lives in
 * `e2e/theme-toggle-live.spec.ts`, where a real browser can be asked what a
 * custom property computes to. Vitest stubs `.css` imports to an empty
 * string even through `?raw`, so a text-matching check here would pass
 * vacuously.
 */

const SOURCES = import.meta.glob('../**/*.vue', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

/** Component sources, excluding anything under a `__tests__` directory. */
const COMPONENTS = Object.entries(SOURCES).filter(([path]) => !path.includes('__tests__'))

/** Colour literals allowed to remain, with why. */
const ALLOWED = new Set([
  // Deposit swatches and their code labels are drawn *on top of* the
  // deposit's own saturated fill colour, not on a themed surface. Black
  // reads correctly against every one of those fills in either theme, so
  // theming it would make it worse, not better.
  '#000',
])

const COLOR_RE = /#[0-9a-fA-F]{3,8}\b|rgba?\([^)]*\)/g
const STYLE_RE = /<style[^>]*>([\s\S]*?)<\/style>/g
const CSS_COMMENT_RE = /\/\*[\s\S]*?\*\//g

describe('theme tokens', () => {
  it('reads the sources it is meant to be checking', () => {
    // A glob that silently matched nothing would make every assertion below
    // pass vacuously.
    expect(COMPONENTS.length).toBeGreaterThan(30)
  })

  it('has no raw colour literals left in component <style> blocks', () => {
    const offenders: string[] = []

    for (const [path, source] of COMPONENTS) {
      for (const block of source.matchAll(STYLE_RE)) {
        // Strip CSS comments so prose mentioning a colour isn't flagged.
        const css = block[1].replace(CSS_COMMENT_RE, '')
        for (const match of css.matchAll(COLOR_RE)) {
          if (ALLOWED.has(match[0].toLowerCase())) continue
          offenders.push(`${path}: ${match[0]}`)
        }
      }
    }

    expect(offenders).toEqual([])
  })

  it('has no raw colour literals in inline SVG fill/stroke attributes', () => {
    // These bypass CSS entirely, so they can't inherit a token — a themed
    // SVG paint has to go through a class instead.
    const offenders: string[] = []
    const attrRe = /\b(?:fill|stroke)="(#[0-9a-fA-F]{3,8})"/g

    for (const [path, source] of COMPONENTS) {
      for (const match of source.matchAll(attrRe)) {
        if (ALLOWED.has(match[1].toLowerCase())) continue
        offenders.push(`${path}: ${match[0]}`)
      }
    }

    expect(offenders).toEqual([])
  })

})
