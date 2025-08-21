import { createRouter, createWebHistory } from 'vue-router';
import Aime from "@/components/aime.vue";
import Home from "@/components/home.vue";
const routes = [
    {
        path: '/',
        name: 'Aime',
        component: Aime
    },    {
        path: '/home',
        name: 'Home',
        component: Home
    }
];

const router = createRouter({
    history: createWebHistory(), // hash 模式: createWebHashHistory()
    routes
});

export default router;
