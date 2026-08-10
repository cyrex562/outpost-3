import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import BuildingsPanel from '@/components/BuildingsPanel.vue'
import type { BuildingRow } from '@/types/screen'

function makeRow(overrides: Partial<BuildingRow>): BuildingRow {
  return {
    building_id: 'b-1',
    name: 'Smelter 1',
    building_type: 'smelter',
    labour_assigned: 5,
    labour_demand: 5,
    priority: 5,
    labour_lock: null,
    paused: false,
    slot_cost: 2,
    full_capacity: true,
    scale: 1.0,
    shortfall_reason: null,
    shortfall_kind: null,
    always_on: false,
    running_recipe_ids: ['smelt_iron'],
    inputs: [{ commodity_id: 'structural_ore', quantity: 2 }],
    outputs: [{ commodity_id: 'structural_metal', quantity: 1 }],
    ...overrides,
  }
}

describe('BuildingsPanel status derivation (#169, corrected in #303)', () => {
  function mountWith(row: Partial<BuildingRow>) {
    return mount(BuildingsPanel, {
      props: {
        buildings: [makeRow(row)],
        slotsUsed: 2,
        slotCapacity: 10,
        labourAvailable: 8,
        labourTotal: 10,
      },
    })
  }

  it('shows Running at full capacity', () => {
    const wrapper = mountWith({ full_capacity: true, scale: 1.0 })
    expect(wrapper.find('[data-testid="building-status-smelter"]').text()).toBe('Running')
  })

  it('shows Partial when it produced something below full output', () => {
    const wrapper = mountWith({ full_capacity: false, scale: 0.4 })
    expect(wrapper.find('[data-testid="building-status-smelter"]').text()).toBe('Partial')
  })

  it('shows Idle only when it genuinely produced nothing', () => {
    const wrapper = mountWith({ full_capacity: false, scale: 0 })
    expect(wrapper.find('[data-testid="building-status-smelter"]').text()).toBe('Idle')
  })

  // Regression for #303: status must not be inferred from `labour_assigned`,
  // which is always 0 because per-building assignment has no backing state.
  // Doing so reported *every* building as Idle regardless of its output.
  it('does not report Idle just because no labour is assigned', () => {
    const wrapper = mountWith({ labour_assigned: 0, full_capacity: true, scale: 1.0 })
    expect(wrapper.find('[data-testid="building-status-smelter"]').text()).toBe('Running')
  })

  it('surfaces the shortfall reason when output fell short', () => {
    const wrapper = mountWith({
      full_capacity: false,
      scale: 0.6,
      shortfall_reason: 'input short: water',
      shortfall_kind: null,
    })
    expect(wrapper.find('[data-testid="building-reason-smelter"]').text()).toBe(
      'input short: water',
    )
  })

  // ── Transient vs real shortfalls (issue #308) ──

  it('styles a pipeline still filling as information, not a fault', () => {
    const wrapper = mountWith({
      full_capacity: false,
      scale: 0,
      shortfall_reason: 'awaiting 10.0 ore from upstream',
      shortfall_kind: 'awaiting_upstream',
    })
    const reason = wrapper.get('[data-testid="building-reason-smelter"]')
    expect(reason.classes()).toContain('is-transient')
    expect(reason.attributes('title')).toContain('resolve on its own')
  })

  it('leaves a real supply problem styled as a fault', () => {
    const wrapper = mountWith({
      full_capacity: false,
      scale: 0,
      shortfall_reason: 'no source of 10.0 ore',
      shortfall_kind: 'input_short',
    })
    const reason = wrapper.get('[data-testid="building-reason-smelter"]')
    expect(reason.classes()).not.toContain('is-transient')
    expect(reason.attributes('title')).toContain('needs attention')
  })

  it('badges an always-on building so the absent recipe picker is explained', () => {
    const wrapper = mountWith({ always_on: true })
    expect(wrapper.find('[data-testid="building-always-on-smelter"]').exists()).toBe(true)
  })

  it('does not badge an ordinary pick-one building as always-on', () => {
    const wrapper = mountWith({ always_on: false })
    expect(wrapper.find('[data-testid="building-always-on-smelter"]').exists()).toBe(false)
  })

  // ── Pause / resume (issue #309) ──

  it('shows Paused and dims the row, overriding scale-derived status', () => {
    const wrapper = mountWith({ paused: true, full_capacity: true, scale: 1.0 })
    expect(wrapper.find('[data-testid="building-status-smelter"]').text()).toBe('Paused')
    expect(wrapper.get('[data-testid="building-row-smelter"]').classes()).toContain('is-paused')
  })

  it('does not dim a running row', () => {
    const wrapper = mountWith({ paused: false, full_capacity: true, scale: 1.0 })
    expect(wrapper.get('[data-testid="building-row-smelter"]').classes()).not.toContain(
      'is-paused',
    )
  })

  it('shows a loading hint when buildings is null', () => {
    const wrapper = mount(BuildingsPanel, {
      props: { buildings: null, slotsUsed: 0, slotCapacity: 0, labourAvailable: 0, labourTotal: 0 },
    })
    expect(wrapper.find('[data-testid="building-list"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('No building data loaded.')
  })
})

describe('BuildingsPanel labour (issue #307)', () => {
  // The per-building labour input and Assign button are gone. They were
  // inert: `Command::AssignLabour` persists nothing, `PlacedBuilding` has no
  // labour field, and `BuildingRow.labour_assigned` is hardcoded 0 — so the
  // typed value was discarded and the displayed number never moved. Labour is
  // allocated automatically as a colony-wide ratio. A real per-building
  // override belongs in the building details page once #307 gives labour
  // backing state; this pins that no inert control comes back meanwhile.
  it('offers no per-building labour control', () => {
    const wrapper = mount(BuildingsPanel, {
      props: {
        buildings: [makeRow({ building_type: 'hydroponic_bay', labour_assigned: 3 })],
        slotsUsed: 2,
        slotCapacity: 10,
        labourAvailable: 7,
        labourTotal: 10,
      },
    })
    expect(wrapper.find('[data-testid="labour-input-hydroponic_bay"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="assign-labour-hydroponic_bay"]').exists()).toBe(false)
  })

  it('still reports the colony-wide labour figures, which are real', () => {
    const wrapper = mount(BuildingsPanel, {
      props: {
        buildings: [makeRow({ building_type: 'hydroponic_bay' })],
        slotsUsed: 2,
        slotCapacity: 10,
        labourAvailable: 7,
        labourTotal: 10,
      },
    })
    expect(wrapper.get('[data-testid="slots-summary"]').text()).toContain('7 of 10')
  })
})

describe('BuildingsPanel details HUD trigger (#182)', () => {
  it('emits view-details with the building type when the name is clicked', async () => {
    const wrapper = mount(BuildingsPanel, {
      props: {
        buildings: [makeRow({ building_type: 'research_lab' })],
        slotsUsed: 1,
        slotCapacity: 10,
        labourAvailable: 9,
        labourTotal: 10,
      },
    })
    await wrapper.find('[data-testid="view-details-research_lab"]').trigger('click')
    expect(wrapper.emitted('view-details')?.[0]).toEqual(['research_lab'])
  })
})

// ── Multi-function building I/O summary (issue #272) ─────────────────────────

describe('BuildingsPanel multi-function I/O summary (#272)', () => {
  function mountRows(rows: Partial<BuildingRow>[]) {
    return mount(BuildingsPanel, {
      props: {
        buildings: rows.map(makeRow),
        slotsUsed: 1,
        slotCapacity: 10,
        labourAvailable: 9,
        labourTotal: 10,
      },
    })
  }

  /**
   * The motivating case from the issue: `colony_hq`'s recipes are *all*
   * always-on, so the row used to show nothing about what it does. It must now
   * name every commodity it produces.
   */
  it('shows the merged outputs of an all-concurrent building', () => {
    const wrapper = mountRows([
      {
        building_type: 'colony_hq',
        always_on: true,
        running_recipe_ids: ['hq_generate_power', 'hq_pump_water', 'hq_scrub_oxygen'],
        inputs: [],
        outputs: [
          { commodity_id: 'oxygen', quantity: 7 },
          { commodity_id: 'power', quantity: 24 },
          { commodity_id: 'water', quantity: 24 },
        ],
      },
    ])
    const io = wrapper.get('[data-testid="building-outputs-colony_hq"]')
    for (const needle of ['power', 'water', 'oxygen', '24', '7']) {
      expect(io.text()).toContain(needle)
    }
  })

  it('shows inputs as well as outputs when the building consumes something', () => {
    const wrapper = mountRows([{ building_type: 'smelter' }])
    expect(wrapper.get('[data-testid="building-inputs-smelter"]').text()).toContain(
      'structural_ore',
    )
    expect(wrapper.get('[data-testid="building-outputs-smelter"]').text()).toContain(
      'structural_metal',
    )
  })

  it('badges how many recipes run at once, but only when there is more than one', () => {
    const wrapper = mountRows([
      {
        building_type: 'colony_hq',
        running_recipe_ids: ['a', 'b', 'c'],
        outputs: [{ commodity_id: 'power', quantity: 1 }],
      },
      { building_type: 'smelter' },
    ])
    const badge = wrapper.get('[data-testid="building-recipe-count-colony_hq"]')
    expect(badge.text()).toContain('3')
    expect(badge.attributes('title')).toContain('a, b, c')
    expect(wrapper.find('[data-testid="building-recipe-count-smelter"]').exists()).toBe(false)
  })

  it('renders no I/O line for a building with no flows', () => {
    const wrapper = mountRows([
      { building_type: 'storage_depot', running_recipe_ids: [], inputs: [], outputs: [] },
    ])
    expect(wrapper.find('[data-testid="building-io-storage_depot"]').exists()).toBe(false)
  })

  it('formats whole quantities without a trailing decimal', () => {
    const wrapper = mountRows([
      {
        building_type: 'well',
        inputs: [],
        outputs: [
          { commodity_id: 'water', quantity: 24 },
          { commodity_id: 'brine', quantity: 1.5 },
        ],
      },
    ])
    const text = wrapper.get('[data-testid="building-outputs-well"]').text()
    expect(text).toContain('24 water')
    expect(text).not.toContain('24.0')
    expect(text).toContain('1.5 brine')
  })
})

// ── Review follow-ups (issue #272) ───────────────────────────────────────────

describe('BuildingsPanel I/O honesty and resilience (#272 review)', () => {
  function mountRows(rows: Partial<BuildingRow>[]) {
    return mount(BuildingsPanel, {
      props: {
        buildings: rows.map(makeRow),
        slotsUsed: 1,
        slotCapacity: 10,
        labourAvailable: 9,
        labourTotal: 10,
      },
    })
  }

  /**
   * The figures are nominal recipe rates, so a throttled building would
   * otherwise show full output right beside its own shortfall reason. The line
   * has to say which number it is.
   */
  it('labels the I/O line as rated rather than actual', () => {
    const wrapper = mountRows([
      { building_type: 'smelter', scale: 0.3, full_capacity: false, shortfall_reason: 'input short: water' },
    ])
    expect(wrapper.get('[data-testid="building-io-rated-smelter"]').text()).toBe('rated')
    // Both readings are on the row, and neither is disguised as the other.
    expect(wrapper.get('[data-testid="building-outputs-smelter"]').text()).toContain(
      'structural_metal',
    )
    expect(wrapper.get('[data-testid="building-reason-smelter"]').text()).toContain('water')
  })

  it('says the figures are rated in the recipe tooltip too', () => {
    const wrapper = mountRows([
      { building_type: 'colony_hq', running_recipe_ids: ['a', 'b'], outputs: [{ commodity_id: 'power', quantity: 5 }] },
    ])
    expect(
      wrapper.get('[data-testid="building-recipe-count-colony_hq"]').attributes('title'),
    ).toContain('rated')
  })

  /**
   * The Rust fields are `#[serde(default)]`, so a host on an older build can
   * legitimately omit them. Dereferencing `.length` on the absent value would
   * take the whole panel down rather than degrading.
   */
  it('survives a payload missing the new I/O fields entirely', () => {
    const legacyRow = {
      building_type: 'legacy_shed',
      labour_assigned: 0,
      slot_cost: 1,
      full_capacity: true,
      scale: 1,
      shortfall_reason: null,
      shortfall_kind: null,
      always_on: false,
    } as unknown as BuildingRow

    const wrapper = mount(BuildingsPanel, {
      props: {
        buildings: [legacyRow],
        slotsUsed: 1,
        slotCapacity: 10,
        labourAvailable: 9,
        labourTotal: 10,
      },
    })

    // The row still renders, just without an I/O line.
    expect(wrapper.find('[data-testid="building-row-legacy_shed"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="building-io-legacy_shed"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="building-recipe-count-legacy_shed"]').exists()).toBe(false)
  })
})

describe('BuildingsPanel per-building staffing (#307 stage 4)', () => {
  function mountRows(rows: Partial<BuildingRow>[]) {
    return mount(BuildingsPanel, {
      props: {
        buildings: rows.map(makeRow),
        slotsUsed: 2,
        slotCapacity: 10,
        labourAvailable: 8,
        labourTotal: 10,
      },
    })
  }

  /**
   * Regression: rows are per placed instance, but were keyed by `building_type`.
   * Two mines therefore shared a Vue key, so Vue treated them as one element and
   * reused the wrong DOM node between them.
   */
  it('gives two instances of one type distinct rows', () => {
    const wrapper = mountRows([
      { building_id: 'mine-a', name: 'Mine 1', building_type: 'mine' },
      { building_id: 'mine-b', name: 'Mine 2', building_type: 'mine' },
    ])

    const ids = wrapper
      .findAll('[data-building-id]')
      .map((el) => el.attributes('data-building-id'))
    expect(ids).toEqual(['mine-a', 'mine-b'])
    expect(new Set(ids).size).toBe(2)

    // And each renders its own controls, addressable by instance.
    expect(wrapper.find('[data-testid="building-priority-mine-a"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="building-priority-mine-b"]').exists()).toBe(true)
  })

  it('shows the instance name rather than the bare type key', () => {
    const wrapper = mountRows([{ building_id: 'm1', name: 'North Vein', building_type: 'mine' }])
    expect(wrapper.find('[data-testid="view-details-mine"]').text()).toBe('North Vein')
  })

  it('falls back to the type when a host omits the newer name field', () => {
    const wrapper = mountRows([
      { building_id: 'm1', name: undefined as unknown as string, building_type: 'mine' },
    ])
    expect(wrapper.find('[data-testid="view-details-mine"]').text()).toBe('mine')
  })

  it('emits the new priority when the selector changes', async () => {
    const wrapper = mountRows([{ building_id: 'm1', priority: 5 }])
    const select = wrapper.get('[data-testid="building-priority-m1"]')
    await select.setValue('2')
    expect(wrapper.emitted('set-priority')).toEqual([['m1', 2]])
  })

  it('does not emit when the selector is set to the value it already has', async () => {
    const wrapper = mountRows([{ building_id: 'm1', priority: 5 }])
    await wrapper.get('[data-testid="building-priority-m1"]').setValue('5')
    expect(wrapper.emitted('set-priority')).toBeUndefined()
  })

  it('pins at the building’s current demand', async () => {
    const wrapper = mountRows([{ building_id: 'm1', labour_demand: 4, labour_lock: null }])
    await wrapper.get('[data-testid="building-pin-m1"]').trigger('click')
    expect(wrapper.emitted('set-lock')).toEqual([['m1', 4]])
  })

  /**
   * A building that couldn't run reports demand 0. Pinning 0 would be a no-op the
   * player didn't ask for, so the floor is one worker.
   */
  it('pins at least one worker even when the building reports no demand', async () => {
    const wrapper = mountRows([{ building_id: 'm1', labour_demand: 0, labour_lock: null }])
    await wrapper.get('[data-testid="building-pin-m1"]').trigger('click')
    expect(wrapper.emitted('set-lock')).toEqual([['m1', 1]])
  })

  it('shows the pinned count and unpins with null', async () => {
    const wrapper = mountRows([{ building_id: 'm1', labour_lock: 3 }])
    expect(wrapper.find('[data-testid="building-locked-m1"]').text()).toContain('3')
    // The pin button is replaced by unpin while locked.
    expect(wrapper.find('[data-testid="building-pin-m1"]').exists()).toBe(false)

    await wrapper.get('[data-testid="building-unpin-m1"]').trigger('click')
    expect(wrapper.emitted('set-lock')).toEqual([['m1', null]])
  })

  it('pausing and resuming emits set-paused with the toggled value', async () => {
    const wrapper = mountRows([{ building_id: 'm1', paused: false }])
    const btn = wrapper.get('[data-testid="building-pause-m1"]')
    expect(btn.text()).toBe('Pause')
    await btn.trigger('click')
    expect(wrapper.emitted('set-paused')).toEqual([['m1', true]])
  })

  it('shows Resume for an already-paused building', () => {
    const wrapper = mountRows([{ building_id: 'm1', paused: true }])
    expect(wrapper.get('[data-testid="building-pause-m1"]').text()).toBe('Resume')
  })

  it('renames with the surrounding whitespace trimmed', async () => {
    const wrapper = mountRows([{ building_id: 'm1', name: 'Mine 1' }])
    await wrapper.get('[data-testid="building-rename-m1"]').trigger('click')
    const input = wrapper.get('[data-testid="building-rename-input-m1"]')
    await input.setValue('  North Vein  ')
    await wrapper.get('[data-testid="building-rename-save-m1"]').trigger('click')
    expect(wrapper.emitted('rename')).toEqual([['m1', 'North Vein']])
  })

  /**
   * Clearing the box is the natural way to ask for the default name back, so it
   * sends `null` rather than an empty string the engine would reject.
   */
  it('treats a cleared name as a request to revert to the default', async () => {
    const wrapper = mountRows([{ building_id: 'm1', name: 'North Vein' }])
    await wrapper.get('[data-testid="building-rename-m1"]').trigger('click')
    await wrapper.get('[data-testid="building-rename-input-m1"]').setValue('   ')
    await wrapper.get('[data-testid="building-rename-save-m1"]').trigger('click')
    expect(wrapper.emitted('rename')).toEqual([['m1', null]])
  })

  it('cancelling a rename emits nothing and closes the editor', async () => {
    const wrapper = mountRows([{ building_id: 'm1', name: 'Mine 1' }])
    await wrapper.get('[data-testid="building-rename-m1"]').trigger('click')
    await wrapper.get('[data-testid="building-rename-input-m1"]').setValue('Discarded')
    await wrapper.get('[data-testid="building-rename-cancel-m1"]').trigger('click')

    expect(wrapper.emitted('rename')).toBeUndefined()
    expect(wrapper.find('[data-testid="building-rename-input-m1"]').exists()).toBe(false)
  })

  it('flags a building that wanted workers and did not get them all', () => {
    const wrapper = mountRows([{ building_id: 'm1', labour_assigned: 2, labour_demand: 5 }])
    expect(wrapper.find('[data-testid="building-understaffed-m1"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="building-staffed-m1"]').text()).toBe('2/5 staffed')
  })

  it('does not flag a fully staffed building', () => {
    const wrapper = mountRows([{ building_id: 'm1', labour_assigned: 5, labour_demand: 5 }])
    expect(wrapper.find('[data-testid="building-understaffed-m1"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="building-staffed-m1"]').text()).toBe('5/5 staffed')
  })

  /**
   * A silo offers no jobs. Reporting it as understaffed would flag every storage
   * structure in the colony.
   */
  it('does not flag a building with no jobs to offer', () => {
    const wrapper = mountRows([{ building_id: 's1', labour_assigned: 0, labour_demand: 0 }])
    expect(wrapper.find('[data-testid="building-understaffed-s1"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="building-staffed-s1"]').text()).toBe('no jobs')
  })
})

describe('BuildingsPanel site multiplier (issue #411)', () => {
  function mountRow(row: Partial<BuildingRow>) {
    return mount(BuildingsPanel, {
      props: {
        buildings: [makeRow(row)],
        slotsUsed: 1,
        slotCapacity: 10,
        labourAvailable: 10,
        labourTotal: 10,
      },
    })
  }

  const detail = (w: ReturnType<typeof mount>) =>
    w.find('[data-testid="building-status-smelter"]').attributes('title') ?? ''

  it('says what a poor site is doing to output, even at full capacity', () => {
    // The case a plain "Running at full output" hides: nothing is throttling
    // the building, but full output here is only 40% of nominal.
    const wrapper = mountRow({ full_capacity: true, scale: 1.0, site_multiplier: 0.4 })
    expect(detail(wrapper)).toContain('Running at full output')
    expect(detail(wrapper)).toMatch(/site yields 40% of nominal/i)
  })

  it('says nothing when the site is neutral', () => {
    const wrapper = mountRow({ full_capacity: true, scale: 1.0, site_multiplier: 1 })
    expect(detail(wrapper)).toBe('Running at full output')
  })

  it('says nothing when the payload predates the field', () => {
    const wrapper = mountRow({ full_capacity: true, scale: 1.0 })
    expect(detail(wrapper)).toBe('Running at full output')
  })

  it('reports a site bonus as well as a penalty', () => {
    const wrapper = mountRow({ full_capacity: true, scale: 1.0, site_multiplier: 1.5 })
    expect(detail(wrapper)).toMatch(/site yields 150% of nominal/i)
  })

  it('appends the site note alongside a shortfall rather than replacing it', () => {
    const wrapper = mountRow({
      full_capacity: false,
      scale: 0.5,
      site_multiplier: 0.4,
      shortfall_reason: 'not enough ore',
    })
    expect(detail(wrapper)).toContain('not enough ore')
    expect(detail(wrapper)).toMatch(/site yields 40% of nominal/i)
  })
})

describe('BuildingsPanel condition and breakdown (#384)', () => {
  function mountWith(row: Partial<BuildingRow>) {
    return mount(BuildingsPanel, {
      props: {
        buildings: [makeRow(row)],
        slotsUsed: 2,
        slotCapacity: 10,
        labourAvailable: 8,
        labourTotal: 10,
      },
    })
  }

  it('reports Broken ahead of every other status', () => {
    // A wreck reported as "Idle" buries the one row needing attention.
    const wrapper = mountWith({ broken: true, paused: true, full_capacity: false, scale: 0 })
    expect(wrapper.find('[data-testid="building-status-smelter"]').text()).toBe('Broken')
  })

  it('offers a repair action only for a broken building', () => {
    const sound = mountWith({ broken: false })
    expect(sound.find('[data-testid="building-repair-b-1"]').exists()).toBe(false)

    const wrecked = mountWith({
      broken: true,
      repair_cost: [{ commodity_id: 'structural_metal', quantity: 14 }],
    })
    expect(wrecked.find('[data-testid="building-repair-b-1"]').exists()).toBe(true)
  })

  it('emits repair with the building id', async () => {
    const wrapper = mountWith({ broken: true })
    await wrapper.find('[data-testid="building-repair-b-1"]').trigger('click')
    expect(wrapper.emitted('repair')?.[0]).toEqual(['b-1'])
  })

  it('names the repair cost so the player knows the price before clicking', () => {
    const wrapper = mountWith({
      broken: true,
      repair_cost: [{ commodity_id: 'structural_metal', quantity: 14 }],
    })
    const title = wrapper.find('[data-testid="building-repair-b-1"]').attributes('title') ?? ''
    expect(title).toContain('structural_metal')
    expect(title).toContain('14')
  })

  it('shows wear as a visible badge, not only a tooltip', () => {
    // Condition below pristine but above the risk threshold: the player should
    // see the slope well before the cliff, without having to hover.
    const wrapper = mountWith({ condition: 0.8, breakdown_risk: 0 })
    const badge = wrapper.find('[data-testid="building-condition-b-1"]')
    expect(badge.exists()).toBe(true)
    expect(badge.text()).toContain('condition 80%')
    expect(badge.text()).not.toContain('risk')
  })

  it('shows the failure risk once the building is in danger', () => {
    const wrapper = mountWith({ condition: 0.25, breakdown_risk: 0.02 })
    const badge = wrapper.find('[data-testid="building-condition-b-1"]')
    expect(badge.text()).toContain('2.0%/sol risk')
  })

  it('drops the condition badge for a broken building, whose status says it', () => {
    const wrapper = mountWith({ condition: 0.2, broken: true })
    expect(wrapper.find('[data-testid="building-condition-b-1"]').exists()).toBe(false)
  })

  it('says nothing about condition for a pristine building', () => {
    const wrapper = mountWith({ condition: 1 })
    expect(wrapper.find('[data-testid="building-condition-b-1"]').exists()).toBe(false)
  })

  it('stays quiet when the backend predates the field', () => {
    // `condition` is optional on the wire; an older payload must not render
    // "condition NaN%".
    const wrapper = mountWith({})
    expect(wrapper.find('[data-testid="building-condition-b-1"]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('NaN')
  })
})
