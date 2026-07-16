<!-- Wall 共用窗口外壳：自定义标题栏、侧栏和当前壁纸状态。 -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { RouterLink, RouterView, useRoute } from 'vue-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { bootstrap, listenToSnapshots } from './api';
import WallIcon from './components/WallIcon.vue';
import { wallStore } from './store';

const route = useRoute();
const windowError = ref('');
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

async function windowAction(action: 'minimize' | 'maximize' | 'close') {
    windowError.value = '';
    try {
        const window = getCurrentWindow();
        if (action === 'minimize') await window.minimize();
        else if (action === 'maximize') await window.toggleMaximize();
        else await window.close();
    } catch (error) {
        windowError.value = error instanceof Error ? error.message : String(error);
    }
}
</script>

<template>
    <div class="app-window">
        <header class="titlebar" data-tauri-drag-region>
            <div class="brand" data-tauri-drag-region>
                <WallIcon name="app" :size="20" />
                <span>Wall</span>
            </div>
            <div class="window-controls">
                <button title="最小化" aria-label="最小化" @click="windowAction('minimize')">
                    <WallIcon name="minimize" :size="16" />
                </button>
                <button title="最大化或还原" aria-label="最大化" @click="windowAction('maximize')">
                    <WallIcon name="maximize" :size="16" />
                </button>
                <button title="关闭" aria-label="关闭" class="close" @click="windowAction('close')">
                    <WallIcon name="close" :size="16" />
                </button>
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
                    <WallIcon class="nav-icon" name="library" :size="18" /><span>壁纸库</span>
                </RouterLink>
                <RouterLink to="/settings/general" class="nav-item" :class="{ selected: route.name === 'settings' }">
                    <WallIcon class="nav-icon" name="settings" :size="18" /><span>设置</span>
                </RouterLink>
                <div class="sidebar-spacer" />
                <div class="sidebar-status">
                    <i :class="{ inactive: !wallStore.snapshot.playback.activeId }" />{{ statusText }}
                </div>
            </aside>
            <main class="main-content"><RouterView /></main>
        </div>
        <div v-if="wallStore.snapshot.playback.lastError" class="toast error-toast">
            {{ wallStore.snapshot.playback.lastError }}
        </div>
        <div v-if="windowError" class="toast error-toast">{{ windowError }}</div>
    </div>
</template>
