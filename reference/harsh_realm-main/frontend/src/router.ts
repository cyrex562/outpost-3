import { createRouter, createWebHistory } from "vue-router";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "game",
      component: () => import("./views/GameView.vue"),
    },
    {
      path: "/admin",
      name: "admin",
      component: () => import("./views/AdminView.vue"),
    },
    {
      path: "/content",
      name: "content",
      component: () => import("./views/ContentView.vue"),
    },
    {
      path: "/difficulty",
      name: "difficulty",
      component: () => import("./views/DifficultyView.vue"),
    },
  ],
});

export default router;
