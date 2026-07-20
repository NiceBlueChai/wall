<!-- Wall 共用窗口外壳：自定义标题栏、侧栏和当前壁纸状态。 -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { bootstrap, createCategory, deleteCategory, listenToSnapshots, renameCategory } from './api';
import WallIcon from './components/WallIcon.vue';
import { wallStore } from './store';

const route = useRoute();
const router = useRouter();
const windowError = ref('');
const categoryMenuOpen = ref(false);
const categoryDialog = ref<'create' | 'rename' | 'delete' | null>(null);
const categoryName = ref('');
const categoryError = ref('');
const categoryBusy = ref(false);
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
const showCategories = computed(() => route.name === 'library' || route.name === 'detail');
const activeCategory = computed(() =>
    wallStore.snapshot.categories.find((item) => item.id === wallStore.activeCategoryId),
);
const categoryDialogTitle = computed(() => {
    if (categoryDialog.value === 'create') return '创建分类';
    if (categoryDialog.value === 'rename') return '重命名分类';
    return '删除分类';
});

onMounted(async () => {
    await bootstrap();
    unlisten = await listenToSnapshots();
});
onUnmounted(() => unlisten());

function preventBrowserContextMenu(event: MouseEvent) {
    const target = event.target;
    if (target instanceof Element && target.closest('input, textarea, [contenteditable="true"]')) return;
    event.preventDefault();
}

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

function categoryCount(categoryId: string | null): number {
    if (!categoryId) return wallStore.snapshot.library.length;
    return wallStore.snapshot.library.filter((item) => item.categoryIds.includes(categoryId)).length;
}

function selectCategory(categoryId: string | null) {
    wallStore.activeCategoryId = categoryId;
    categoryMenuOpen.value = false;
    if (route.name !== 'library') router.push('/');
}

function openCategoryDialog(kind: 'create' | 'rename' | 'delete') {
    categoryDialog.value = kind;
    categoryName.value = kind === 'rename' ? (activeCategory.value?.name ?? '') : '';
    categoryError.value = '';
    categoryMenuOpen.value = false;
}

function enterBatchMode() {
    wallStore.enterBatchMode();
    categoryMenuOpen.value = false;
    router.push('/');
}

async function submitCategoryDialog() {
    categoryError.value = '';
    if (categoryDialog.value !== 'delete' && !categoryName.value.trim()) {
        categoryError.value = '请输入分类名称';
        return;
    }
    categoryBusy.value = true;
    try {
        if (categoryDialog.value === 'create') await createCategory(categoryName.value);
        else if (categoryDialog.value === 'rename' && activeCategory.value) {
            await renameCategory(activeCategory.value.id, categoryName.value);
        } else if (categoryDialog.value === 'delete' && activeCategory.value) {
            await deleteCategory(activeCategory.value.id);
        }
        categoryDialog.value = null;
    } catch (error) {
        categoryError.value = error instanceof Error ? error.message : String(error);
    } finally {
        categoryBusy.value = false;
    }
}
</script>

<template>
    <div class="app-window" @contextmenu="preventBrowserContextMenu">
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
                <template v-if="showCategories">
                    <div class="category-heading">
                        <span>分类</span>
                        <button aria-label="管理分类" @click="categoryMenuOpen = !categoryMenuOpen">
                            <WallIcon name="settings" :size="16" />
                        </button>
                    </div>
                    <div class="category-list">
                        <button
                            data-category-id="all"
                            :class="{ selected: wallStore.activeCategoryId === null }"
                            @click="selectCategory(null)"
                        >
                            <span>全部壁纸</span><small>{{ categoryCount(null) }}</small>
                        </button>
                        <button
                            v-for="category in wallStore.snapshot.categories"
                            :key="category.id"
                            :data-category-id="category.id"
                            :class="{ selected: wallStore.activeCategoryId === category.id }"
                            @click="selectCategory(category.id)"
                        >
                            <span>{{ category.name }}</span
                            ><small>{{ categoryCount(category.id) }}</small>
                        </button>
                        <button class="category-add" aria-label="添加分类" @click="openCategoryDialog('create')">
                            <WallIcon name="add" :size="16" /><span>添加分类</span>
                        </button>
                    </div>
                    <div v-if="categoryMenuOpen" class="category-action-menu">
                        <button data-category-action="create" @click="openCategoryDialog('create')">
                            <WallIcon name="add" :size="18" />创建分类
                        </button>
                        <button data-category-action="batch" @click="enterBatchMode">
                            <WallIcon name="check" :size="18" />批量管理
                        </button>
                        <button
                            data-category-action="rename"
                            :disabled="!activeCategory"
                            @click="openCategoryDialog('rename')"
                        >
                            <WallIcon name="settings" :size="18" />重命名分类
                        </button>
                        <button
                            class="danger-action"
                            data-category-action="delete"
                            :disabled="!activeCategory"
                            @click="openCategoryDialog('delete')"
                        >
                            <WallIcon name="trash" :size="18" />删除分类
                        </button>
                    </div>
                </template>
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
        <div v-if="categoryDialog" class="modal-scrim">
            <div class="dialog category-dialog">
                <form @submit.prevent="submitCategoryDialog">
                    <h2>{{ categoryDialogTitle }}</h2>
                    <p v-if="categoryDialog === 'delete'">
                        删除“{{ activeCategory?.name }}”后，壁纸文件和媒体库条目都会保留。
                    </p>
                    <label v-else>
                        分类名称
                        <input v-model="categoryName" maxlength="40" autofocus />
                    </label>
                    <p v-if="categoryError" class="inline-error">{{ categoryError }}</p>
                    <div class="dialog-actions">
                        <button type="button" class="secondary" @click="categoryDialog = null">取消</button>
                        <button
                            type="submit"
                            :class="categoryDialog === 'delete' ? 'danger' : 'primary'"
                            :disabled="categoryBusy"
                        >
                            {{ categoryDialog === 'delete' ? '删除' : '确认' }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    </div>
</template>
