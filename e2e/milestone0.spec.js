import { test, expect } from '@playwright/test'

// Helper: hit the backend API directly
async function apiPost(request, path) {
  return request.post(`http://localhost:8000${path}`)
}

async function apiGet(request, path) {
  return request.get(`http://localhost:8000${path}`)
}

async function resetSimulation(request) {
  // Pause and set speed to 1d
  await apiPost(request, '/api/pause')
  await apiPost(request, '/api/speed/1')
}

// ── App loads ────────────────────────────────────────────────────

test.describe('App loads', () => {
  test('page loads with title bar', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('text=Outpost 3')).toBeVisible()
  })

  test('Time Controls panel is visible', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('text=Time Controls')).toBeVisible()
  })

  test('Event Log panel is visible', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('text=Event Log')).toBeVisible()
  })
})

// ── WebSocket connection ─────────────────────────────────────────

test.describe('WebSocket connection', () => {
  test('connects and shows LIVE indicator', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })
  })

  test('displays initial time state', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })
    // Should show Year 1 (or whatever the current state is)
    await expect(page.locator('text=/Year \\d+/')).toBeVisible()
  })
})

// ── Play/Pause controls ─────────────────────────────────────────

test.describe('Play/Pause controls', () => {
  test.beforeEach(async ({ request }) => {
    await resetSimulation(request)
  })

  test('shows Play button when paused', async ({ page, request }) => {
    await apiPost(request, '/api/pause')
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })
    await expect(page.locator('button', { hasText: '▶ Play' })).toBeVisible()
  })

  test('clicking Play starts simulation and shows Pause button', async ({ page, request }) => {
    await apiPost(request, '/api/pause')
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })

    await page.locator('button', { hasText: '▶ Play' }).click()
    await expect(page.locator('button', { hasText: '⏸ Pause' })).toBeVisible({ timeout: 3000 })
  })

  test('clicking Pause stops simulation and shows Play button', async ({ page, request }) => {
    await apiPost(request, '/api/resume')
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })

    // Should show Pause button while running
    await expect(page.locator('button', { hasText: '⏸ Pause' })).toBeVisible({ timeout: 3000 })

    await page.locator('button', { hasText: '⏸ Pause' }).click()
    await expect(page.locator('button', { hasText: '▶ Play' })).toBeVisible({ timeout: 3000 })
  })
})

// ── Speed controls ───────────────────────────────────────────────

test.describe('Speed controls', () => {
  test('all 14 speed buttons are visible', async ({ page }) => {
    await page.goto('/')
    const speedLabels = ['1d', '2d', '1w', '10d', '1mo', '3mo', '6mo', '1y', '2y', '5y', '10y', '25y', '50y', '100y']
    for (const label of speedLabels) {
      await expect(page.locator('.grid button').getByText(label, { exact: true })).toBeVisible()
    }
  })

  test('clicking a speed button highlights it', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })

    const btn = page.locator('.grid button', { hasText: '1w' })
    await btn.click()

    // Should have the accent border class
    await expect(btn).toHaveClass(/border-panel-accent/, { timeout: 3000 })
  })

  test('speed label updates in status text', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })

    await page.locator('.grid button', { hasText: '3mo' }).click()
    await expect(page.locator('text=Speed: 3mo')).toBeVisible({ timeout: 3000 })
  })
})

// ── Time advances ────────────────────────────────────────────────

test.describe('Simulation ticks', () => {
  test.beforeEach(async ({ request }) => {
    await resetSimulation(request)
  })

  test('day counter advances when playing at 1d speed', async ({ page, request }) => {
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })

    // Note the initial state from the API
    const stateResp = await apiGet(request, '/api/state')
    const initialState = await stateResp.json()
    const initialDay = initialState.day_offset

    // Start playing
    await apiPost(request, '/api/resume')

    // Wait for at least one tick (~2.5s)
    await page.waitForTimeout(4000)

    // Check the day has advanced
    const newResp = await apiGet(request, '/api/state')
    const newState = await newResp.json()
    expect(newState.day_offset).toBeGreaterThan(initialDay)

    await apiPost(request, '/api/pause')
  })

  test('higher speed advances more days per cycle', async ({ page, request }) => {
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })

    const stateResp = await apiGet(request, '/api/state')
    const initialState = await stateResp.json()
    const initialDay = initialState.day_offset

    // Set to 30d (1 month per cycle) and play
    await apiPost(request, '/api/speed/30')
    await apiPost(request, '/api/resume')

    // Wait for ~2 cycles
    await page.waitForTimeout(6000)

    const newResp = await apiGet(request, '/api/state')
    const newState = await newResp.json()

    // Should have advanced at least 30 days
    expect(newState.day_offset - initialDay).toBeGreaterThanOrEqual(30)

    await apiPost(request, '/api/pause')
  })
})

// ── Event log ────────────────────────────────────────────────────

test.describe('Event log', () => {
  test.beforeEach(async ({ request }) => {
    await resetSimulation(request)
  })

  test('events appear in the log when simulation runs', async ({ page, request }) => {
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })

    // Set speed to 30d so we generate month/year events quickly
    await apiPost(request, '/api/speed/30')
    await apiPost(request, '/api/resume')

    // Wait for events to stream in
    await page.waitForTimeout(5000)
    await apiPost(request, '/api/pause')

    // Event log should have entries (not the empty state message)
    await expect(page.locator('text=No events yet')).not.toBeVisible()

    // Should have at least one event row with a timestamp
    const eventRows = page.locator('.overflow-y-auto .flex.gap-2')
    await expect(eventRows.first()).toBeVisible()
  })

  test('severity filter buttons work', async ({ page, request }) => {
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })

    // Run simulation to generate events
    await apiPost(request, '/api/speed/30')
    await apiPost(request, '/api/resume')
    await page.waitForTimeout(5000)
    await apiPost(request, '/api/pause')

    // Get count with All filter
    const allEvents = await page.locator('.overflow-y-auto .flex.gap-2').count()

    // Click "Notable+" filter
    await page.locator('button', { hasText: 'Notable+' }).click()

    // Should have fewer (or equal) events
    const notableEvents = await page.locator('.overflow-y-auto .flex.gap-2').count()
    expect(notableEvents).toBeLessThanOrEqual(allEvents)
  })

  test('clear button removes all events', async ({ page, request }) => {
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })

    // Generate some events
    await apiPost(request, '/api/speed/30')
    await apiPost(request, '/api/resume')
    await page.waitForTimeout(5000)
    await apiPost(request, '/api/pause')

    // Click Clear
    await page.locator('button', { hasText: 'Clear' }).click()

    // Should show the empty state
    await expect(page.locator('text=No events yet')).toBeVisible()
  })
})

// ── Auto-pause ───────────────────────────────────────────────────

test.describe('Auto-pause', () => {
  test('simulation auto-pauses at year boundary', async ({ page, request }) => {
    // Reset to day 0, speed 1y per cycle
    await apiPost(request, '/api/pause')
    await apiPost(request, '/api/speed/365')
    await page.goto('/')
    await expect(page.locator('text=LIVE')).toBeVisible({ timeout: 5000 })

    // Resume — should auto-pause when hitting year 2
    await apiPost(request, '/api/resume')

    // Wait for the auto-pause to trigger (one cycle = 2.5s + processing)
    await page.waitForTimeout(5000)

    // Should be paused now
    const stateResp = await apiGet(request, '/api/state')
    const state = await stateResp.json()
    expect(state.paused).toBe(true)
    expect(state.year).toBeGreaterThanOrEqual(2)

    // Play button should be visible (paused state)
    await expect(page.locator('button', { hasText: '▶ Play' })).toBeVisible({ timeout: 3000 })
  })
})

// ── Panel window management ──────────────────────────────────────

test.describe('Panel management', () => {
  test('panels can be minimized and restored', async ({ page }) => {
    await page.goto('/')

    // Find the minimize button on the Event Log panel
    const eventLogPanel = page.locator('.absolute.border', { has: page.locator('text=Event Log') })
    await expect(eventLogPanel).toBeVisible()

    // Click minimize
    const minimizeBtn = eventLogPanel.locator('button', { hasText: '_' })
    await minimizeBtn.click()

    // Panel content should be hidden
    await expect(eventLogPanel).not.toBeVisible()

    // Minimized bar should show the panel name
    const restoreBtn = page.locator('.absolute.bottom-0 button', { hasText: 'Event Log' })
    await expect(restoreBtn).toBeVisible()

    // Click to restore
    await restoreBtn.click()
    await expect(eventLogPanel).toBeVisible()
  })

  test('panels can be closed', async ({ page }) => {
    await page.goto('/')

    const eventLogPanel = page.locator('.absolute.border', { has: page.locator('text=Event Log') })
    await expect(eventLogPanel).toBeVisible()

    // Click close
    const closeBtn = eventLogPanel.locator('button', { hasText: '×' })
    await closeBtn.click()

    // Panel should be gone
    await expect(eventLogPanel).not.toBeVisible()
  })
})
