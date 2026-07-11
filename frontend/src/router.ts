import { createRouter, createWebHashHistory } from 'vue-router'
import MainMenuView from './views/MainMenuView.vue'
import ColonyView from './views/ColonyView.vue'
import SystemMapView from './views/SystemMapView.vue'
import FoundColonyWizardView from './views/FoundColonyWizardView.vue'
import TechTreeView from './views/TechTreeView.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', component: MainMenuView, name: 'menu' },
    { path: '/system', component: SystemMapView, name: 'system' },
    { path: '/colony', component: ColonyView, name: 'colony' },
    { path: '/found', component: FoundColonyWizardView, name: 'found' },
    { path: '/tech', component: TechTreeView, name: 'tech' },
  ],
})

export default router
