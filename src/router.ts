/** 定义壁纸库、详情和四个设置页的本地哈希路由。 */
import { createRouter, createWebHashHistory } from 'vue-router';
import LibraryView from './views/LibraryView.vue';
import DetailView from './views/DetailView.vue';
import SettingsView from './views/SettingsView.vue';

export default createRouter({
    history: createWebHashHistory(),
    routes: [
        { path: '/', name: 'library', component: LibraryView },
        { path: '/wallpaper/:id', name: 'detail', component: DetailView },
        { path: '/settings/:section?', name: 'settings', component: SettingsView },
        { path: '/:pathMatch(.*)*', redirect: '/' },
    ],
});
