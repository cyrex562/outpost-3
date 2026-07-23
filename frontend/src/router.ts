import { createRouter, createWebHashHistory } from 'vue-router'
import MainMenuView from './views/MainMenuView.vue'
import ColonyView from './views/ColonyView.vue'
import SystemMapView from './views/SystemMapView.vue'
import FoundColonyWizardView from './views/FoundColonyWizardView.vue'
import TechTreeView from './views/TechTreeView.vue'
import NewGameView from './views/NewGameView.vue'
import OutpostsView from './views/OutpostsView.vue'
import FacilityView from './views/FacilityView.vue'
import InstallationsView from './views/InstallationsView.vue'
import SystemBodiesView from './views/SystemBodiesView.vue'
import BuildingsListView from './views/BuildingsListView.vue'
import OutpostView from './views/OutpostView.vue'
import OutpostFacilityView from './views/OutpostFacilityView.vue'
import PlanetView from './views/PlanetView.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', component: MainMenuView, name: 'menu' },
    { path: '/system', component: SystemMapView, name: 'system' },
    { path: '/planet', component: PlanetView, name: 'planet' },
    { path: '/new-game', component: NewGameView, name: 'new-game' },
    { path: '/colony/:colonyId?', component: ColonyView, name: 'colony' },
    {
      path: '/colony/:colonyId/facility/:buildingType',
      component: FacilityView,
      name: 'facility',
    },
    { path: '/found', component: FoundColonyWizardView, name: 'found' },
    { path: '/tech', component: TechTreeView, name: 'tech' },
    { path: '/outposts', component: OutpostsView, name: 'outposts' },
    { path: '/installations', component: InstallationsView, name: 'installations' },
    { path: '/bodies', component: SystemBodiesView, name: 'bodies' },
    { path: '/buildings', component: BuildingsListView, name: 'buildings' },
    { path: '/outpost/:outpostId', component: OutpostView, name: 'outpost' },
    {
      path: '/outpost/:outpostId/facility/:buildingType',
      component: OutpostFacilityView,
      name: 'outpost-facility',
    },
  ],
})

export default router
