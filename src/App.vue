<!-- Wall 共用窗口外壳：自定义标题栏、侧栏和当前壁纸状态。 -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { RouterLink, RouterView, useRoute } from 'vue-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { bootstrap, listenToSnapshots } from './api';
import { wallStore } from './store';

const route = useRoute();
let unlisten: () => void = () => undefined;

const activeWallpaper = computed(() => {
    const id = wallStore.snapshot.playback.activeId;
    return wallStore.snapshot.library.find((item) => item.id === id) ?? null;
});
const statusText = computed(() => {
    if (!activeWallpaper.value) return '未运行壁纸';
    const status = wallStore.snapshot.playback.status === 'paused' ? '已暂停' : '运行中';
    return `${activeWallpaper.value.name} · ${status}`;
});

onMounted(async () => {
    await bootstrap();
    unlisten = await listenToSnapshots();
});
onUnmounted(() => unlisten());

function windowAction(action: 'minimize' | 'maximize' | 'close') {
    const window = getCurrentWindow();
    if (action === 'minimize') return window.minimize();
    if (action === 'maximize') return window.toggleMaximize();
    return window.close();
}
</script>

<template>
    <div class="app-window">
        <header class="titlebar" data-tauri-drag-region>
            <div class="brand" data-tauri-drag-region>
                <img :src="'/wall-app-icon.png'" alt="" />
                <span>Wall</span>
            </div>
            <div class="window-controls">
                <button aria-label="最小化" @click="windowAction('minimize')">—</button>
                <button aria-label="最大化" @click="windowAction('maximize')">□</button>
                <button aria-label="关闭" class="close" @click="windowAction('close')">×</button>
            </div>
        </header>
        <div class="body-shell">
            <aside class="sidebar">
                <div class="sidebar-label">BROWSE</div>
                <RouterLink
                    to="/"
                    class="nav-item"
                    :class="{ selected: route.name === 'library' || route.name === 'detail' }"
                >
                    <span class="nav-icon">▦</span><span>壁纸库</span>
                </RouterLink>
                <RouterLink to="/settings/general" class="nav-item" :class="{ selected: route.name === 'settings' }">
                    <span class="nav-icon">⚙</span><span>设置</span>
                </RouterLink>
                <div class="sidebar-spacer" />
                <div class="sidebar-status"><i />{{ statusText }}</div>
            </aside>
            <main class="main-content"><RouterView /></main>
        </div>
        <div v-if="wallStore.snapshot.playback.lastError" class="toast error-toast">
            {{ wallStore.snapshot.playback.lastError }}
        </div>
    </div>
</template>
