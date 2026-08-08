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

  it('disables Queue and shows the reason when a building is gated', () => {
    const option = makeBuildingOption({ id: 'research_lab' })
    const wrapper = mountDialog([option], () => 'Requires: basic_metallurgy')
    expect(wrapper.find('[data-testid="btn-queue-research_lab"]').attributes('disabled')).toBeDefined()
    expect(wrapper.find('[data-testid="build-card-reason-research_lab"]').text()).toBe(
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
