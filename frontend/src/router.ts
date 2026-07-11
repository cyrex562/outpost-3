import { createRouter, createWebHistory } from 'vue-router'
import ColonyView from './views/ColonyView.vue'
import NewGameView from './views/NewGameView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/new-game' },
    { path: '/new-game', component: NewGameView, name: 'new-game' },
    { path: '/colony', component: ColonyView, name: 'colony' },
  ],
})

export default router
