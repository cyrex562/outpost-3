import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import BuildingsPanel from '@/components/BuildingsPanel.vue'
import type { BuildingRow } from '@/types/screen'

function makeRow(overrides: Partial<BuildingRow>): BuildingRow {
  return {
    building_type: 'smelter',
    labour_assigned: 5,
    slot_cost: 2,
    full_capacity: true,
    scale: 1.0,
    shortfall_reason: null,
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
    })
    expect(wrapper.find('[data-testid="building-reason-smelter"]').text()).toBe(
      'input short: water',
    )
  })

  it('badges an always-on building so the absent recipe picker is explained', () => {
    const wrapper = mountWith({ always_on: true })
    expect(wrapper.find('[data-testid="building-always-on-smelter"]').exists()).toBe(true)
  })

  it('does not badge an ordinary pick-one building as always-on', () => {
    const wrapper = mountWith({ always_on: false })
    expect(wrapper.find('[data-testid="building-always-on-smelter"]').exists()).toBe(false)
  })

  it('shows a loading hint when buildings is null', () => {
    const wrapper = mount(BuildingsPanel, {
      props: { buildings: null, slotsUsed: 0, slotCapacity: 0, labourAvailable: 0, labourTotal: 0 },
    })
    expect(wrapper.find('[data-testid="building-list"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('No building data loaded.')
  })
})

describe('BuildingsPanel labour assignment', () => {
  it('emits assign-labour with the drafted value and clears the draft', async () => {
    const wrapper = mount(BuildingsPanel, {
      props: {
        buildings: [makeRow({ building_type: 'hydroponic_bay', labour_assigned: 3 })],
        slotsUsed: 2,
        slotCapacity: 10,
        labourAvailable: 7,
        labourTotal: 10,
      },
    })
    const input = wrapper.find('[data-testid="labour-input-hydroponic_bay"]')
    await input.setValue('8')
    const assignBtn = wrapper.find('[data-testid="assign-labour-hydroponic_bay"]')
    expect(assignBtn.attributes('disabled')).toBeUndefined()
    await assignBtn.trigger('click')

    const emitted = wrapper.emitted('assign-labour')
    expect(emitted).toBeTruthy()
    expect(emitted?.[0]).toEqual(['hydroponic_bay', 8])

    // Draft is cleared after assigning, so the input reflects the prop value again.
    await wrapper.vm.$nextTick()
    expect(wrapper.find('[data-testid="assign-labour-hydroponic_bay"]').attributes('disabled')).toBeDefined()
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
