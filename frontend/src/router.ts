import { createRouter, createWebHashHistory } from 'vue-router'
import MainMenuView from './views/MainMenuView.vue'
import ColonyView from './views/ColonyView.vue'
import SystemMapView from './views/SystemMapView.vue'
import FoundColonyWizardView from './views/FoundColonyWizardView.vue'
import TechTreeView from './views/TechTreeView.vue'
import NewGameView from './views/NewGameView.vue'
import OutpostsView from './views/OutpostsView.vue'
import InstallationsView from './views/InstallationsView.vue'
import SystemBodiesView from './views/SystemBodiesView.vue'
import BuildingsListView from './views/BuildingsListView.vue'
import OutpostView from './views/OutpostView.vue'
import OutpostFacilityView from './views/OutpostFacilityView.vue'
import PlanetView from './views/PlanetView.vue'
import SurfaceView from './views/SurfaceView.vue'
import ColoniesListView from './views/ColoniesListView.vue'
import BalanceView from './views/BalanceView.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', component: MainMenuView, name: 'menu' },
    { path: '/system', component: SystemMapView, name: 'system' },
    { path: '/planet', component: PlanetView, name: 'planet' },
    { path: '/surface/:bodyId', component: SurfaceView, name: 'surface' },
    { path: '/new-game', component: NewGameView, name: 'new-game' },
    { path: '/colonies', component: ColoniesListView, name: 'colonies' },
    { path: '/colony/:colonyId?', component: ColonyView, name: 'colony' },
    {
      // Deep link into the colony dashboard's building-details dock panel
      // (issue #322) rather than a separate routed page — same component as
      // the `colony` route above, just with `buildingType` also present.
      path: '/colony/:colonyId/facility/:buildingType',
      component: ColonyView,
      name: 'facility',
    },
    { path: '/found', component: FoundColonyWizardView, name: 'found' },
    { path: '/tech', component: TechTreeView, name: 'tech' },
    { path: '/outposts', component: OutpostsView, name: 'outposts' },
    { path: '/installations', component: InstallationsView, name: 'installations' },
    { path: '/bodies', component: SystemBodiesView, name: 'bodies' },
    { path: '/buildings', component: BuildingsListView, name: 'buildings' },
    { path: '/balance', component: BalanceView, name: 'balance' },
    { path: '/outpost/:outpostId', component: OutpostView, name: 'outpost' },
    {
      path: '/outpost/:outpostId/facility/:buildingType',
      component: OutpostFacilityView,
      name: 'outpost-facility',
    },
  ],
})

export default router
