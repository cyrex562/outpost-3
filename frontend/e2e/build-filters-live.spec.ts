import { test, expect } from '@playwright/test'

/**
 * Covers the build dialog's two catalogue filters against a live
 * `outpost_web` backend: hiding tech-locked entries and hiding ones the
 * colony cannot currently fund.
 *
 * A fresh colony is the right fixture for both — most of the roster is
 * tech-locked at tech 0, and a colony with an empty stockpile can afford
 * almost nothing.
 */

async function newGameAndOpenBuild(page: import('@playwright/test').Page) {
  await page.goto('/')
  await page.getByTestId('btn-new-game').click()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  await page.getByTestId('btn-open-build').click()
  await expect(page.getByTestId('build-dialog')).toBeVisible()
  await expect
    .poll(async () => page.locator('[data-testid^="build-card-"]').count(), { timeout: 15_000 })
    .toBeGreaterThan(0)
}

/** Ids of the building cards currently rendered (excluding their reason spans). */
async function visibleCards(page: import('@playwright/test').Page): Promise<string[]> {
  return page
    .locator('[data-testid^="build-card-"]')
    .evaluateAll((els) =>
      els
        .map((e) => e.getAttribute('data-testid') ?? '')
        .filter((id) => !id.startsWith('build-card-reason-'))
        .map((id) => id.replace('build-card-', '')),
    )
}

test('both filters start off and the full roster is shown', async ({ page }) => {
  await newGameAndOpenBuild(page)

  await expect(page.getByTestId('filter-hide-tech-locked')).not.toBeChecked()
  await expect(page.getByTestId('filter-hide-unaffordable')).not.toBeChecked()

  const all = await visibleCards(page)
  // The pack has a large roster, most of it tech-gated at tech 0.
  expect(all.length).toBeGreaterThan(40)
  expect(all).toContain('colony_hq')
  expect(all).toContain('fusion_reactor_prototype')
})

test('the tech filter hides tech-locked buildings and keeps tech-0 ones', async ({ page }) => {
  await newGameAndOpenBuild(page)
  const before = await visibleCards(page)

  await page.getByTestId('filter-hide-tech-locked').check()
  const after = await visibleCards(page)

  expect(after.length).toBeLessThan(before.length)
  // Nothing researched yet, so a deep-tech building must be gone...
  expect(after).not.toContain('fusion_reactor_prototype')
  // ...while a tech-0 one stays.
  expect(after).toContain('colony_hq')
  await expect(page.getByTestId('build-dialog-hidden-count')).toBeVisible()
})

test('the affordability filter is independent of the tech filter', async ({ page }) => {
  await newGameAndOpenBuild(page)
  const before = await visibleCards(page)

  await page.getByTestId('filter-hide-unaffordable').check()
  const afterAfford = await visibleCards(page)

  // A brand-new colony has an empty stockpile, so anything with a material
  // cost drops out — but the two filters must not be the same filter.
  expect(afterAfford.length).toBeLessThan(before.length)
  await expect(page.getByTestId('filter-hide-tech-locked')).not.toBeChecked()

  // Turning the tech filter on as well narrows it further (or holds), never
  // widens it.
  await page.getByTestId('filter-hide-tech-locked').check()
  const afterBoth = await visibleCards(page)
  expect(afterBoth.length).toBeLessThanOrEqual(afterAfford.length)
})

test('each filter is remembered separately across reopening the dialog', async ({ page }) => {
  await newGameAndOpenBuild(page)

  await page.getByTestId('filter-hide-tech-locked').check()
  await page.getByTestId('btn-close-build').click()
  await page.getByTestId('btn-open-build').click()

  await expect(page.getByTestId('filter-hide-tech-locked')).toBeChecked()
  await expect(page.getByTestId('filter-hide-unaffordable')).not.toBeChecked()
})


test('the catalogue is a collapsible tree grouped by category', async ({ page }) => {
  await newGameAndOpenBuild(page)

  const groups = page.locator('[data-testid^="build-cat-"]:not([data-testid^="build-cat-toggle-"])')
  await expect.poll(async () => groups.count()).toBeGreaterThan(5)

  // Every category starts expanded, so the whole roster is reachable without
  // a click.
  const power = page.getByTestId('build-cat-toggle-Power')
  await expect(power).toHaveAttribute('aria-expanded', 'true')
  await expect(page.getByTestId('build-card-solar_array_mk1')).toBeVisible()

  // Collapsing hides that category's rows and nothing else.
  await power.click()
  await expect(power).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByTestId('build-card-solar_array_mk1')).toBeHidden()
  await expect(page.getByTestId('build-card-colony_hq')).toBeVisible()

  // ...and the choice survives closing and reopening the dialog.
  await page.getByTestId('btn-close-build').click()
  await page.getByTestId('btn-open-build').click()
  await expect(page.getByTestId('build-cat-toggle-Power')).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByTestId('build-cat-toggle-Extraction')).toHaveAttribute('aria-expanded', 'true')
})


test('every requirement is listed and marked, met and unmet alike', async ({ page }) => {
  await newGameAndOpenBuild(page)

  // A fresh colony has an empty stockpile and nothing researched, so
  // `habitat_dome` is short on its tech *and* both its commodities — the case
  // `disabledReason` could only ever report one of.
  const dome = page.locator('[data-testid^="build-req-habitat_dome-"]')
  await expect.poll(async () => dome.count()).toBeGreaterThan(2)

  const unmet = page.locator('[data-testid^="build-req-habitat_dome-"][data-met="false"]')
  await expect.poll(async () => unmet.count()).toBeGreaterThanOrEqual(3)

  // Its tech and both commodities each say what is missing.
  await expect(page.getByTestId('build-req-habitat_dome-tech-planetary_life_support')).toHaveAttribute(
    'data-met',
    'false',
  )
  await expect(page.getByTestId('build-req-habitat_dome-cost-structural_metal')).toContainText('have 0')

  // Met requirements are listed too: the colony has all 10 slots free, so the
  // slot requirement is satisfied on the same building that fails three others.
  await expect(page.getByTestId('build-req-habitat_dome-slots')).toHaveAttribute('data-met', 'true')

  // Status is not carried by colour alone — the marks differ in shape.
  const metMark = await page.locator('[data-met="true"] .req-mark').first().textContent()
  const unmetMark = await page.locator('[data-met="false"] .req-mark').first().textContent()
  expect(metMark?.trim()).not.toBe(unmetMark?.trim())
  expect(metMark?.trim()).toBeTruthy()
  expect(unmetMark?.trim()).toBeTruthy()
})
