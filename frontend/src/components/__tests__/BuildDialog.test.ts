import { beforeEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import BuildDialog from '@/components/BuildDialog.vue'
import type { BuildingOption } from '@/services/tauriBridge'

function makeBuildingOption(overrides: Partial<BuildingOption>): BuildingOption {
  return {
    id: 'smelter',
    name: 'Smelter',
    description: '',
    category: 'Industry',
    slot_cost: 2,
    labor_per_turn: 3,
    construction_turns: 5,
    construction_cost: [],
    tech_prerequisite: null,
    starter_kit: false,
    max_instances: null,
    ...overrides,
  }
}

function mountDialog(catalog: BuildingOption[], disabledReason: (b: BuildingOption) => string | null = () => null) {
  return mount(BuildDialog, {
    props: {
      catalog,
      disabledReason,
      slotsAvailable: 3,
      busy: false,
      isTechLocked: () => false,
      isAffordable: () => true,
      requirements: () => [],
      siteYield: () => null,
    },
  })
}

describe('BuildDialog (UI-rework PR5)', () => {
  it('queues a building with quantity 1 by default', async () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mountDialog([option])
    await wrapper.find('[data-testid="btn-queue-research_lab"]').trigger('click')
    expect(wrapper.emitted('queue')).toEqual([[option, 1]])
  })

  it('queues the chosen quantity', async () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mountDialog([option])
    await wrapper.find('[data-testid="qty-research_lab"]').setValue('4')
    await wrapper.find('[data-testid="btn-queue-research_lab"]').trigger('click')
    expect(wrapper.emitted('queue')).toEqual([[option, 4]])
  })

  it('clamps a blank/zero quantity up to 1', async () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mountDialog([option])
    await wrapper.find('[data-testid="qty-research_lab"]').setValue('0')
    await wrapper.find('[data-testid="btn-queue-research_lab"]').trigger('click')
    expect(wrapper.emitted('queue')).toEqual([[option, 1]])
  })

  it('disables Queue and explains why when a building is gated', () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mountDialog([option], () => 'Requires: basic_metallurgy')
    expect(wrapper.find('[data-testid="btn-queue-research_lab"]').attributes('disabled')).toBeDefined()
    // The reason moved from a line of its own to the row's tooltip — the
    // requirement badges below now name every blocker, not just the first, so
    // repeating one of them inline said the same thing twice (issue #423).
    expect(wrapper.get('[data-testid="build-card-research_lab"]').attributes('title')).toBe(
      'Requires: basic_metallurgy',
    )
  })

  it('emits close on the close button', async () => {
    const wrapper = mountDialog([])
    await wrapper.find('[data-testid="btn-close-build"]').trigger('click')
    expect(wrapper.emitted('close')).toHaveLength(1)
  })
})


describe('BuildDialog catalog failure vs empty pack', () => {
  it('shows the failure reason instead of the empty-pack hint when loading failed', () => {
    const wrapper = mount(BuildDialog, {
      props: {
        catalog: [],
        disabledReason: () => null,
        slotsAvailable: 3,
        busy: false,
        isTechLocked: () => false,
        isAffordable: () => true,
        requirements: () => [],
        siteYield: () => null,
        catalogError: 'Could not load the building catalog: no content registry loaded',
      },
    })
    const err = wrapper.find('[data-testid="build-dialog-error"]')
    expect(err.exists()).toBe(true)
    expect(err.text()).toContain('no content registry loaded')
    // The two used to render identically, which is what hid the bug.
    expect(wrapper.text()).not.toContain('No buildings available in the loaded content pack')
  })

  it('still shows the empty-pack hint when the catalog is genuinely empty', () => {
    const wrapper = mount(BuildDialog, {
      props: {
        catalog: [],
        disabledReason: () => null,
        slotsAvailable: 3,
        busy: false,
        isTechLocked: () => false,
        isAffordable: () => true,
        requirements: () => [],
        siteYield: () => null,
      },
    })
    expect(wrapper.find('[data-testid="build-dialog-error"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('No buildings available in the loaded content pack')
  })
})


describe('BuildDialog catalogue filters', () => {
  const TECH_LOCKED = 'reactor'
  const UNAFFORDABLE = 'foundry'
  const BUILDABLE = 'shed'

  function mountFiltered() {
    return mount(BuildDialog, {
      props: {
        catalog: [
          makeBuildingOption({ id: TECH_LOCKED, name: 'Reactor' }),
          makeBuildingOption({ id: UNAFFORDABLE, name: 'Foundry' }),
          makeBuildingOption({ id: BUILDABLE, name: 'Shed' }),
        ],
        disabledReason: () => null,
        slotsAvailable: 5,
        busy: false,
        isTechLocked: (b: BuildingOption) => b.id === TECH_LOCKED,
        isAffordable: (b: BuildingOption) => b.id !== UNAFFORDABLE,
        requirements: () => [],
        siteYield: () => null,
      },
    })
  }

  const visibleIds = (w: ReturnType<typeof mount>) =>
    w
      .findAll('[data-testid^="build-card-"]')
      .map((el) => el.attributes('data-testid')?.replace('build-card-', ''))
      .filter((id) => id && !id.startsWith('reason-'))

  beforeEach(() => {
    window.localStorage.clear()
  })

  it('shows the whole roster with both filters off', () => {
    const wrapper = mountFiltered()
    expect(visibleIds(wrapper)).toEqual([TECH_LOCKED, UNAFFORDABLE, BUILDABLE])
    expect(wrapper.find('[data-testid="build-dialog-hidden-count"]').exists()).toBe(false)
  })

  it('hides only tech-locked entries when that filter is on', async () => {
    const wrapper = mountFiltered()
    await wrapper.get('[data-testid="filter-hide-tech-locked"]').setValue(true)

    // The unaffordable one must survive — the two filters are independent.
    expect(visibleIds(wrapper)).toEqual([UNAFFORDABLE, BUILDABLE])
    expect(wrapper.get('[data-testid="build-dialog-hidden-count"]').text()).toContain('1')
  })

  it('hides only unaffordable entries when that filter is on', async () => {
    const wrapper = mountFiltered()
    await wrapper.get('[data-testid="filter-hide-unaffordable"]').setValue(true)

    expect(visibleIds(wrapper)).toEqual([TECH_LOCKED, BUILDABLE])
  })

  it('combines both filters', async () => {
    const wrapper = mountFiltered()
    await wrapper.get('[data-testid="filter-hide-tech-locked"]').setValue(true)
    await wrapper.get('[data-testid="filter-hide-unaffordable"]').setValue(true)

    expect(visibleIds(wrapper)).toEqual([BUILDABLE])
    expect(wrapper.get('[data-testid="build-dialog-hidden-count"]').text()).toContain('2')
  })

  it('explains an empty list caused by filters, rather than looking like missing content', async () => {
    const wrapper = mount(BuildDialog, {
      props: {
        catalog: [makeBuildingOption({ id: TECH_LOCKED })],
        disabledReason: () => null,
        slotsAvailable: 5,
        busy: false,
        isTechLocked: () => true,
        isAffordable: () => true,
        requirements: () => [],
        siteYield: () => null,
      },
    })
    await wrapper.get('[data-testid="filter-hide-tech-locked"]').setValue(true)

    const notice = wrapper.find('[data-testid="build-dialog-all-filtered"]')
    expect(notice.exists()).toBe(true)
    expect(notice.text()).toMatch(/hidden by the filters/i)
    // Must not be confused with a genuinely empty pack.
    expect(wrapper.text()).not.toContain('No buildings available in the loaded content pack')
  })

  it('defaults both filters off, so the full roster is the starting view', () => {
    const wrapper = mountFiltered()
    expect(
      (wrapper.get('[data-testid="filter-hide-tech-locked"]').element as HTMLInputElement).checked,
    ).toBe(false)
    expect(
      (wrapper.get('[data-testid="filter-hide-unaffordable"]').element as HTMLInputElement).checked,
    ).toBe(false)
  })

  it('remembers each filter independently across reopens', async () => {
    const first = mountFiltered()
    await first.get('[data-testid="filter-hide-unaffordable"]').setValue(true)
    first.unmount()

    const second = mountFiltered()
    expect(
      (second.get('[data-testid="filter-hide-unaffordable"]').element as HTMLInputElement).checked,
    ).toBe(true)
    expect(
      (second.get('[data-testid="filter-hide-tech-locked"]').element as HTMLInputElement).checked,
    ).toBe(false)
    expect(visibleIds(second)).toEqual([TECH_LOCKED, BUILDABLE])
  })
})


describe('BuildDialog category tree', () => {
  function catalogueOf(...pairs: [string, string][]) {
    return pairs.map(([id, category]) => makeBuildingOption({ id, name: id, category }))
  }

  function mountTree(catalog = catalogueOf(
    ['smelter', 'Processing'],
    ['refinery', 'Processing'],
    ['mine', 'Extraction'],
    ['reactor', 'Power'],
  )) {
    return mount(BuildDialog, {
      props: {
        catalog,
        disabledReason: () => null,
        slotsAvailable: 5,
        busy: false,
        isTechLocked: () => false,
        isAffordable: () => true,
        requirements: () => [],
        siteYield: () => null,
      },
    })
  }

  beforeEach(() => {
    window.localStorage.clear()
  })

  it('groups the catalogue into one section per category', () => {
    const wrapper = mountTree()
    const groups = wrapper.findAll('[data-testid^="build-cat-"]:not([data-testid^="build-cat-toggle-"])')
    expect(groups.map((g) => g.attributes('data-testid'))).toEqual([
      'build-cat-Processing',
      'build-cat-Extraction',
      'build-cat-Power',
    ])
  })

  it('shows a per-category count so a collapsed group still says what is inside', () => {
    const wrapper = mountTree()
    expect(wrapper.get('[data-testid="build-cat-toggle-Processing"]').text()).toContain('2')
    expect(wrapper.get('[data-testid="build-cat-toggle-Power"]').text()).toContain('1')
  })

  it('starts every category expanded, so nothing is hidden by default', () => {
    const wrapper = mountTree()
    for (const cat of ['Processing', 'Extraction', 'Power']) {
      expect(wrapper.get(`[data-testid="build-cat-toggle-${cat}"]`).attributes('aria-expanded')).toBe('true')
    }
    expect(wrapper.findAll('[data-testid^="build-card-"]').length).toBe(4)
  })

  it('collapses and re-expands one category without touching the others', async () => {
    const wrapper = mountTree()
    const toggle = wrapper.get('[data-testid="build-cat-toggle-Processing"]')

    await toggle.trigger('click')
    expect(toggle.attributes('aria-expanded')).toBe('false')
    expect(wrapper.get('[data-testid="build-cat-toggle-Extraction"]').attributes('aria-expanded')).toBe('true')

    await toggle.trigger('click')
    expect(toggle.attributes('aria-expanded')).toBe('true')
  })

  it('persists collapsed categories across reopening the dialog', async () => {
    const first = mountTree()
    await first.get('[data-testid="build-cat-toggle-Processing"]').trigger('click')
    first.unmount()

    const second = mountTree()
    expect(second.get('[data-testid="build-cat-toggle-Processing"]').attributes('aria-expanded')).toBe('false')
    expect(second.get('[data-testid="build-cat-toggle-Extraction"]').attributes('aria-expanded')).toBe('true')
  })

  it('expands a category the stored state has never seen', () => {
    // Storing the *collapsed* set means a category added to the pack later
    // shows up rather than silently starting hidden.
    window.localStorage.setItem('outpost3.build-dialog.collapsed-categories', JSON.stringify(['Processing']))
    const wrapper = mountTree()
    expect(wrapper.get('[data-testid="build-cat-toggle-Processing"]').attributes('aria-expanded')).toBe('false')
    expect(wrapper.get('[data-testid="build-cat-toggle-Power"]').attributes('aria-expanded')).toBe('true')
  })

  it('falls back to all-expanded when the stored state is corrupt', () => {
    window.localStorage.setItem('outpost3.build-dialog.collapsed-categories', 'not json')
    const wrapper = mountTree()
    expect(wrapper.get('[data-testid="build-cat-toggle-Processing"]').attributes('aria-expanded')).toBe('true')
  })

  it('drops a category the filters have emptied, rather than leaving a zero heading', async () => {
    const wrapper = mount(BuildDialog, {
      props: {
        catalog: catalogueOf(['smelter', 'Processing'], ['mine', 'Extraction']),
        disabledReason: () => null,
        slotsAvailable: 5,
        busy: false,
        isTechLocked: (b: BuildingOption) => b.id === 'smelter',
        isAffordable: () => true,
        requirements: () => [],
        siteYield: () => null,
      },
    })
    expect(wrapper.find('[data-testid="build-cat-Processing"]').exists()).toBe(true)

    await wrapper.get('[data-testid="filter-hide-tech-locked"]').setValue(true)

    expect(wrapper.find('[data-testid="build-cat-Processing"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="build-cat-Extraction"]').exists()).toBe(true)
  })

  it('splits a CamelCase category into readable words', () => {
    const wrapper = mountTree(catalogueOf(['scrubber', 'LifeSupport']))
    // Would otherwise render as "LIFESUPPORT" under the heading's uppercasing.
    expect(wrapper.get('[data-testid="build-cat-toggle-LifeSupport"]').text()).toContain('Life Support')
  })

  it('exposes the group as a disclosure the heading controls', () => {
    const wrapper = mountTree()
    const toggle = wrapper.get('[data-testid="build-cat-toggle-Power"]')
    expect(toggle.element.tagName).toBe('BUTTON')
    expect(toggle.attributes('aria-controls')).toBe('build-cat-body-Power')
    expect(wrapper.find('#build-cat-body-Power').exists()).toBe(true)
  })
})

describe('BuildDialog expected site yield (issue #414)', () => {
  function mountWithYield(multiplier: number | null) {
    return mount(BuildDialog, {
      props: {
        catalog: [makeBuildingOption({ id: 'geothermal_plant', name: 'Geothermal Plant' })],
        disabledReason: () => null,
        slotsAvailable: 5,
        busy: false,
        isTechLocked: () => false,
        isAffordable: () => true,
        requirements: () => [],
        siteYield: () => multiplier,
      },
    })
  }

  const yieldText = (w: ReturnType<typeof mount>) =>
    w.find('[data-testid="build-site-yield-geothermal_plant"]').text()

  it('warns before committing when the site is poor', () => {
    // The trap this closes: a geothermal plant on cold crust is worthless,
    // and without this the player only finds out once it is built.
    const wrapper = mountWithYield(0.4)
    expect(yieldText(wrapper)).toContain('40%')
    expect(yieldText(wrapper)).toMatch(/barely worth the slot/i)
  })

  it('calls out a strong site', () => {
    expect(yieldText(mountWithYield(1.3))).toMatch(/strong site/i)
  })

  it('reports an ordinary site plainly', () => {
    const text = yieldText(mountWithYield(1))
    expect(text).toContain('100%')
    expect(text).not.toMatch(/strong|weak|barely/i)
  })

  it('shows nothing for a building whose output does not depend on the site', () => {
    const wrapper = mountWithYield(null)
    expect(wrapper.find('[data-testid="build-site-yield-geothermal_plant"]').exists()).toBe(false)
  })
})
