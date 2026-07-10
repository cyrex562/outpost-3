import { createRouter, createWebHistory } from 'vue-router'
import ColonyView from './views/ColonyView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/colony' },
    { path: '/colony', component: ColonyView, name: 'colony' },
  ],
})

export default router
