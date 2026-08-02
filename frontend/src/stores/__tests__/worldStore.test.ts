import { describe, expect, it, beforeEach } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useWorldStore } from '@/stores/worldStore'
import type { ServerEvent } from '@/types/events'

const COLONY_ID = 'colony-1'

function foundColony() {
  return { type: 'event' as const, event: { kind: 'colony_founded', colony_id: COLONY_ID, name: 'Alpha Base', starting_population: 100 } satisfies ServerEvent }
}

describe('worldStore — unified colony log (events/alerts unification)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('logs a non-alert event as a single ambient entry, without popping the alert toast', () => {
    const store = useWorldStore()
    store.handleServerMessage(foundColony())
    store.handleServerMessage({
      type: 'event',
      event: { kind: 'construction_queued', colony_id: COLONY_ID, building_type: 'greenhouse', project_id: 'p1' },
    })

    const entries = store.logEntries
    const queued = entries.find((e) => e.event.kind === 'construction_queued')
    expect(queued).toBeDefined()
    expect(queued!.tier).toBe('ambient')
    expect(store.alertToast).toBeNull()
  })

  it('logs an alert-worthy event with the reducer\'s curated tier/message, and pops the toast', () => {
    const store = useWorldStore()
    store.handleServerMessage(foundColony())
    store.handleServerMessage({
      type: 'event',
      event: { kind: 'production_shortfall', colony_id: COLONY_ID, building_type: 'mine', scale: 0.5, reason: 'Labour' },
    })

    const entries = store.logEntries
    const shortfall = entries.find((e) => e.event.kind === 'production_shortfall')
    expect(shortfall).toBeDefined()
    expect(shortfall!.tier).toBe('notable')
    expect(shortfall!.message).toContain('mine')
    expect(store.alertToast).not.toBeNull()
    expect(store.alertToast!.event.kind).toBe('production_shortfall')
  })

  it('a later alert overwrites the toast; an ambient event afterward does not clear it', () => {
    const store = useWorldStore()
    store.handleServerMessage(foundColony())
    store.handleServerMessage({
      type: 'event',
      event: { kind: 'production_shortfall', colony_id: COLONY_ID, building_type: 'mine', scale: 0.5, reason: 'Labour' },
    })
    expect(store.alertToast!.event.kind).toBe('production_shortfall')

    store.handleServerMessage({
      type: 'event',
      event: { kind: 'construction_queued', colony_id: COLONY_ID, building_type: 'greenhouse', project_id: 'p1' },
    })
    // Ambient event doesn't touch the toast — still the shortfall.
    expect(store.alertToast!.event.kind).toBe('production_shortfall')

    store.handleServerMessage({
      type: 'event',
      event: { kind: 'hazard_occurred', colony_id: COLONY_ID, hazard_kind: 'fire', severity: 0.5, stability_delta: -0.1, commodity_losses: [], population_lost: 0 },
    })
    expect(store.alertToast!.event.kind).toBe('hazard_occurred')
  })

  it('dismissAlertToast clears the toast without touching the log', () => {
    const store = useWorldStore()
    store.handleServerMessage(foundColony())
    store.handleServerMessage({
      type: 'event',
      event: { kind: 'production_shortfall', colony_id: COLONY_ID, building_type: 'mine', scale: 0.5, reason: 'Labour' },
    })
    expect(store.alertToast).not.toBeNull()
    const logLengthBefore = store.logEntries.length

    store.dismissAlertToast()

    expect(store.alertToast).toBeNull()
    expect(store.logEntries).toHaveLength(logLengthBefore)
  })

  it('clearLog empties the log without touching the curated notifications list', () => {
    const store = useWorldStore()
    store.handleServerMessage(foundColony())
    store.handleServerMessage({
      type: 'event',
      event: { kind: 'production_shortfall', colony_id: COLONY_ID, building_type: 'mine', scale: 0.5, reason: 'Labour' },
    })
    expect(store.logEntries.length).toBeGreaterThan(0)
    expect(store.notifications.length).toBeGreaterThan(0)

    store.clearLog()

    expect(store.logEntries).toHaveLength(0)
    expect(store.notifications.length).toBeGreaterThan(0)
  })

  it('trims the log once it exceeds the cap, keeping the most recent entries', () => {
    const store = useWorldStore()
    store.handleServerMessage(foundColony())
    for (let i = 0; i < 210; i++) {
      store.handleServerMessage({
        type: 'event',
        event: { kind: 'construction_queued', colony_id: COLONY_ID, building_type: 'greenhouse', project_id: `p${i}` },
      })
    }
    expect(store.logEntries.length).toBeLessThanOrEqual(200)
    // The most recent entry survives the trim.
    expect(store.logEntries[store.logEntries.length - 1]!.event).toMatchObject({ project_id: 'p209' })
  })

  it('reset() clears the log and dismisses any pending toast', () => {
    const store = useWorldStore()
    store.handleServerMessage(foundColony())
    store.handleServerMessage({
      type: 'event',
      event: { kind: 'production_shortfall', colony_id: COLONY_ID, building_type: 'mine', scale: 0.5, reason: 'Labour' },
    })

    store.reset()

    expect(store.logEntries).toHaveLength(0)
    expect(store.alertToast).toBeNull()
  })
})
