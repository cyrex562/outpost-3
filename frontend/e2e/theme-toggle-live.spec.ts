import { test, expect } from '@playwright/test'

/**
 * Covers the light/dark theme toggle end to end: that it actually repaints
 * the page (not just flips an attribute), that an explicit choice overrides
 * the OS preference in both directions, and that it survives a reload.
 *
 * Colours are read back off the live computed style rather than compared to
 * fixed hex values, so the assertions stay true if the palette is retuned —
 * what matters is that light mode is light and dark mode is dark.
 */

/** Perceived lightness (0–255) of a `rgb(r, g, b)` string. */
async function bodyLightness(page: import('@playwright/test').Page): Promise<number> {
  return page.evaluate(() => {
    const bg = getComputedStyle(document.body).backgroundColor
    const m = /rgba?\((\d+),\s*(\d+),\s*(\d+)/.exec(bg)
    if (!m) throw new Error(`unexpected background colour: ${bg}`)
    const [r, g, b] = [Number(m[1]), Number(m[2]), Number(m[3])]
    return 0.2126 * r + 0.7152 * g + 0.0722 * b
  })
}

test('theme toggle switches the palette and persists', async ({ page }) => {
  await page.goto('/')
  await page.getByTestId('btn-new-game').click()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  const toggle = page.getByTestId('theme-toggle')
  await expect(toggle).toBeVisible()

  // Starts following the system, and Playwright's default is a light OS
  // preference — so no data-theme attribute is set at all.
  await expect(toggle).toHaveText('auto')
  expect(await page.getAttribute('html', 'data-theme')).toBeNull()

  // auto → light
  await toggle.click()
  await expect(toggle).toHaveText('light')
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
  const light = await bodyLightness(page)

  // light → dark, and the page genuinely repaints
  await toggle.click()
  await expect(toggle).toHaveText('dark')
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  const dark = await bodyLightness(page)

  expect(light).toBeGreaterThan(200)
  expect(dark).toBeLessThan(60)

  // The explicit choice survives a reload.
  await page.reload()
  await expect(page.getByTestId('theme-toggle')).toHaveText('dark')
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  expect(await bodyLightness(page)).toBeLessThan(60)

  // dark → back to following the system, which clears the attribute.
  await page.getByTestId('theme-toggle').click()
  await expect(page.getByTestId('theme-toggle')).toHaveText('auto')
  expect(await page.getAttribute('html', 'data-theme')).toBeNull()
})

test('both themes define the same tokens, and every referenced token resolves', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByTestId('btn-new-game').click()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  /**
   * Read the theme's custom properties out of the real cascade. This is the
   * half of the token guard a unit test can't do: vitest stubs `.css`
   * imports, so only a browser can say what `--surface-1` actually computes
   * to under a given theme.
   */
  const readTokens = () =>
    page.evaluate(() => {
      const names = new Set<string>()
      const used = new Set<string>()
      for (const sheet of Array.from(document.styleSheets)) {
        let rules: CSSRuleList
        try {
          rules = sheet.cssRules
        } catch {
          continue // cross-origin sheet — none of ours
        }
        for (const rule of Array.from(rules)) {
          const text = rule.cssText
          for (const m of text.matchAll(/(--[a-z0-9-]+)\s*:/g)) names.add(m[1])
          for (const m of text.matchAll(/var\((--[a-z0-9-]+)\)/g)) used.add(m[1])
        }
      }
      const style = getComputedStyle(document.documentElement)
      const resolved: Record<string, string> = {}
      for (const n of names) resolved[n] = style.getPropertyValue(n).trim()
      return { defined: [...names], used: [...used], resolved }
    })

  await page.getByTestId('theme-toggle').click() // auto → light
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
  const light = await readTokens()

  await page.getByTestId('theme-toggle').click() // light → dark
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  const dark = await readTokens()

  // Sanity: the scan found a real palette, not an empty page.
  expect(light.defined.length).toBeGreaterThan(30)

  // Every token a stylesheet references must actually resolve, in both
  // themes — a `var(--typo)` renders as nothing and is easy to miss.
  for (const token of light.used) {
    expect(light.resolved[token], `unresolved in light: ${token}`).not.toBe('')
    expect(dark.resolved[token], `unresolved in dark: ${token}`).not.toBe('')
  }

  // A token defined in only one theme silently inherits the other's value.
  expect([...light.defined].sort()).toEqual([...dark.defined].sort())

  // And the two palettes must genuinely differ, or "light mode" is a no-op.
  const differing = light.defined.filter((t) => light.resolved[t] !== dark.resolved[t])
  expect(differing.length).toBeGreaterThan(20)
})

test('an explicit light choice survives a dark OS preference', async ({ page }) => {
  // The case a boolean toggle gets wrong: the player wants light while the
  // machine is set to dark.
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  await page.getByTestId('btn-new-game').click()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  // Following the OS, so the dark palette applies with no attribute set.
  await expect(page.getByTestId('theme-toggle')).toHaveText('auto')
  expect(await bodyLightness(page)).toBeLessThan(60)

  await page.getByTestId('theme-toggle').click()
  await expect(page.getByTestId('theme-toggle')).toHaveText('light')
  expect(await bodyLightness(page)).toBeGreaterThan(200)
})
