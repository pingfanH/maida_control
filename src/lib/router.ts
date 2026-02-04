import { createRouter, createWebHistory } from "vue-router";
import Aime from "@/components/aime.vue";
import Home from "@/components/home.vue";
import Favorites from "@/components/favorites.vue";
import Clash from "@/components/clash.vue";

const routes = [
  {
    path: "/",
    name: "Aime",
    component: Aime,
  },
  {
    path: "/home",
    name: "Home",
    component: Home,
  },
  {
    path: "/favorites",
    name: "Favorites",
    component: Favorites,
  },
  {
    path: "/clash",
    name: "Clash",
    component: Clash,
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
