<!-- Wall 共用窗口外壳：自定义标题栏、侧栏和当前壁纸状态。 -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, provide, ref } from 'vue';
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
    bootstrap,
    createCategory,
    deleteCategory,
    listenToSnapshots,
    renameCategory,
    setCategoryMembership,
} from './api';
import { openCategoryCreatorKey } from './categoryCreator';
import WallIcon from './components/WallIcon.vue';
import { wallStore } from './store';

const route = useRoute();
const router = useRouter();
const appError = ref('');
const categoryMenuOpen = ref(false);
const categoryDialog = ref<'create' | 'rename' | 'delete' | null>(null);
const categoryName = ref('');
const categoryError = ref('');
const categoryBusy = ref(false);
const categoryAssignmentIds = ref<string[]>([]);
const categoryMenuTrigger = ref<HTMLButtonElement | null>(null);
const categoryMenu = ref<HTMLElement | null>(null);
const categoryInput = ref<HTMLInputElement | null>(null);
const categoryCancel = ref<HTMLButtonElement | null>(null);
const categoryDialogElement = ref<HTMLElement | null>(null);
let categoryReturnFocus: HTMLElement | null = null;
let unlisten: () => void = () => undefined;

const activeWallpaper = computed(() => {
    const id = wallStore.snapshot.playback.activeId;
    return wallStore.snapshot.library.find((item) => item.id === id) ?? null;
});
const statusText = computed(() => {
    const assignments = wallStore.snapshot.playback.displayAssignments ?? [];
    if (!assignments.length) {
        if (!activeWallpaper.value) return '未运行壁纸';
        const status = wallStore.snapshot.playback.status === 'paused' ? '已暂停' : '运行中';
        return `${activeWallpaper.value.name} · ${status}`;
    }

    const displays = wallStore.snapshot.displays ?? [];
    const offlineCount = assignments.filter((assignment) =>
        assignment.displayIds.some((id) => !displays.some((display) => display.id === id && display.connected)),
    ).length;
    if (assignments.length > 1) {
        if (offlineCount) return `${assignments.length} 个显示目标 · ${offlineCount} 个离线`;
        return `${assignments.length} 个显示目标 · ${assignmentStatus(assignments.map((item) => item.status))}`;
    }

    const assignment = assignments[0];
    const status = assignmentStatus([assignment.status], false);
    if (offlineCount) return `${targetLabel(assignment.mode)} · 离线`;
    if (assignment.mode === 'clone') return `复制组 · ${assignment.displayIds.length} 块屏幕 · ${status}`;
    if (assignment.mode === 'span') return `铺展组 · ${assignment.displayIds.length} 块屏幕 · ${status}`;
    const display = displays.find((item) => item.id === assignment.displayIds[0]);
    return `${display?.name ?? activeWallpaper.value?.name ?? '显示目标'} · ${status}`;
});
const statusIndicator = computed(() => {
    const playback = wallStore.snapshot.playback;
    const assignments = playback.displayAssignments ?? [];
    return {
        inactive: !assignments.length && !playback.activeId,
        error: assignments.some((item) => item.status === 'error') || playback.status === 'error',
    };
});
const showCategories = computed(() => route.name === 'library' || route.name === 'detail');
const activeCategory = computed(() =>
    wallStore.snapshot.categories.find((item) => item.id === wallStore.activeCategoryId),
);
const categoryDialogTitle = computed(() => {
    if (categoryDialog.value === 'create') {
        return categoryAssignmentIds.value.length ? '新建分类' : '创建分类';
    }
    if (categoryDialog.value === 'rename') return '重命名分类';
    return '删除分类';
});
const categoryDialogConfirmLabel = computed(() => {
    if (categoryDialog.value === 'delete') return '删除';
    if (categoryDialog.value === 'create' && categoryAssignmentIds.value.length) return '创建并添加';
    return '确认';
});

provide(openCategoryCreatorKey, (mediaIds, trigger) => openCategoryDialog('create', trigger, mediaIds));

onMounted(async () => {
    document.addEventListener('keydown', handleDocumentKeydown);
    document.addEventListener('pointerdown', handleDocumentPointerDown);
    await bootstrap();
    unlisten = await listenToSnapshots();
});
onUnmounted(() => {
    document.removeEventListener('keydown', handleDocumentKeydown);
    document.removeEventListener('pointerdown', handleDocumentPointerDown);
    unlisten();
});

function preventBrowserContextMenu(event: MouseEvent) {
    const target = event.target;
    if (target instanceof Element && target.closest('input, textarea, [contenteditable="true"]')) return;
    event.preventDefault();
}

async function windowAction(action: 'minimize' | 'maximize' | 'close') {
    appError.value = '';
    try {
        const window = getCurrentWindow();
        if (action === 'minimize') await window.minimize();
        else if (action === 'maximize') await window.toggleMaximize();
        else await window.close();
    } catch (error) {
        appError.value = error instanceof Error ? error.message : String(error);
    }
}

function categoryCount(categoryId: string | null): number {
    if (!categoryId) return wallStore.snapshot.library.length;
    return wallStore.snapshot.library.filter((item) => item.categoryIds.includes(categoryId)).length;
}

function assignmentStatus(statuses: string[], multiple = true): string {
    if (statuses.includes('error')) return '错误';
    const pausedCount = statuses.filter((status) => status === 'paused').length;
    if (pausedCount === statuses.length) return multiple ? '全部暂停' : '已暂停';
    if (pausedCount) return '部分暂停';
    return '运行中';
}

function targetLabel(mode: string): string {
    if (mode === 'clone') return '复制组';
    if (mode === 'span') return '铺展组';
    return '显示目标';
}

function selectCategory(categoryId: string | null) {
    wallStore.activeCategoryId = categoryId;
    categoryMenuOpen.value = false;
    if (route.name !== 'library') router.push('/');
}

function toggleCategoryMenu() {
    categoryMenuOpen.value = !categoryMenuOpen.value;
}

function openCategoryMenuFromKeyboard(last: boolean) {
    categoryMenuOpen.value = true;
    nextTick(() => focusMenuEdge(categoryMenu.value, last));
}

function closeCategoryMenu(returnFocus = false) {
    categoryMenuOpen.value = false;
    if (returnFocus) nextTick(() => categoryMenuTrigger.value?.focus());
}

function handleCategoryMenuKeydown(event: KeyboardEvent) {
    const menu = event.currentTarget;
    if (!(menu instanceof HTMLElement)) return;
    const items = enabledMenuItems(menu);
    if (!items.length) return;
    const currentIndex = items.indexOf(document.activeElement as HTMLButtonElement);
    let nextIndex: number | null = null;
    if (event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % items.length;
    else if (event.key === 'ArrowUp') nextIndex = (currentIndex - 1 + items.length) % items.length;
    else if (event.key === 'Home') nextIndex = 0;
    else if (event.key === 'End') nextIndex = items.length - 1;
    else if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        closeCategoryMenu(true);
        return;
    }
    if (nextIndex === null) return;
    event.preventDefault();
    items[nextIndex].focus();
}

function enabledMenuItems(container: HTMLElement): HTMLButtonElement[] {
    return Array.from(container.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
}

function focusMenuEdge(menu: HTMLElement | null, last: boolean) {
    if (!menu) return;
    const items = enabledMenuItems(menu);
    items[last ? items.length - 1 : 0]?.focus();
}

function openCategoryDialog(
    kind: 'create' | 'rename' | 'delete',
    trigger?: EventTarget | null,
    mediaIds: string[] = [],
) {
    const triggerElement = trigger instanceof HTMLElement ? trigger : categoryMenuTrigger.value;
    const triggerInsideMenu = triggerElement?.closest('.category-action-menu');
    categoryReturnFocus = triggerInsideMenu ? categoryMenuTrigger.value : triggerElement;
    categoryDialog.value = kind;
    categoryAssignmentIds.value = kind === 'create' ? [...new Set(mediaIds)] : [];
    categoryName.value = kind === 'rename' ? (activeCategory.value?.name ?? '') : '';
    categoryError.value = '';
    appError.value = '';
    categoryMenuOpen.value = false;
    nextTick(() => {
        if (kind === 'delete') categoryCancel.value?.focus();
        else categoryInput.value?.focus();
    });
}

function closeCategoryDialog() {
    if (categoryBusy.value) return;
    categoryDialog.value = null;
    categoryAssignmentIds.value = [];
    nextTick(() => categoryReturnFocus?.focus());
}

async function submitCategoryDialog() {
    if (categoryBusy.value) return;
    categoryError.value = '';
    if (categoryDialog.value !== 'delete' && !categoryName.value.trim()) {
        categoryError.value = '请输入分类名称';
        return;
    }
    if (categoryDialog.value !== 'delete' && Array.from(categoryName.value.trim()).length > 40) {
        categoryError.value = '分类名称最多 40 个字符';
        return;
    }
    categoryBusy.value = true;
    nextTick(() => categoryDialogElement.value?.focus());
    try {
        if (categoryDialog.value === 'create') {
            const previousIds = new Set(wallStore.snapshot.categories.map((category) => category.id));
            const assignmentIds = [...categoryAssignmentIds.value];
            const snapshot = await createCategory(categoryName.value);
            if (assignmentIds.length) {
                const createdCategory = snapshot.categories.find((category) => !previousIds.has(category.id));
                if (!createdCategory) throw new Error('无法识别新创建的分类');
                try {
                    await setCategoryMembership(assignmentIds, createdCategory.id, true);
                } catch {
                    categoryDialog.value = null;
                    categoryAssignmentIds.value = [];
                    appError.value = '分类已创建，但添加到当前壁纸失败，请重试';
                    nextTick(() => categoryReturnFocus?.focus());
                    return;
                }
            }
        } else if (categoryDialog.value === 'rename' && activeCategory.value) {
            await renameCategory(activeCategory.value.id, categoryName.value);
        } else if (categoryDialog.value === 'delete' && activeCategory.value) {
            await deleteCategory(activeCategory.value.id);
        }
        categoryDialog.value = null;
        categoryAssignmentIds.value = [];
        nextTick(() => categoryReturnFocus?.focus());
    } catch (error) {
        categoryError.value = error instanceof Error ? error.message : String(error);
    } finally {
        categoryBusy.value = false;
        if (categoryDialog.value) {
            nextTick(() => {
                if (categoryDialog.value === 'delete') categoryCancel.value?.focus();
                else categoryInput.value?.focus();
            });
        }
    }
}

function trapDialogFocus(event: KeyboardEvent) {
    if (event.key !== 'Tab') return;
    const dialog = event.currentTarget;
    if (!(dialog instanceof HTMLElement)) return;
    const selector = 'button:not(:disabled), input:not(:disabled), select:not(:disabled), [tabindex="0"]';
    const items = Array.from(dialog.querySelectorAll<HTMLElement>(selector));
    if (!items.length) {
        event.preventDefault();
        dialog.focus();
        return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
    }
}

function handleDocumentKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    if (categoryDialog.value) closeCategoryDialog();
    else if (categoryMenuOpen.value) closeCategoryMenu(true);
}

function handleDocumentPointerDown(event: PointerEvent) {
    if (!categoryMenuOpen.value) return;
    const target = event.target;
    if (!(target instanceof Node)) return;
    if (categoryMenuTrigger.value?.contains(target) || categoryMenu.value?.contains(target)) return;
    closeCategoryMenu(true);
}
</script>

<template>
    <div class="app-window" @contextmenu="preventBrowserContextMenu">
        <header class="titlebar" data-tauri-drag-region :inert="categoryDialog ? true : undefined">
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
        <div class="body-shell" :inert="categoryDialog ? true : undefined">
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
                        <button
                            ref="categoryMenuTrigger"
                            aria-label="管理分类"
                            aria-haspopup="menu"
                            :aria-expanded="categoryMenuOpen"
                            @click="toggleCategoryMenu"
                            @keydown.down.prevent="openCategoryMenuFromKeyboard(false)"
                            @keydown.up.prevent="openCategoryMenuFromKeyboard(true)"
                        >
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
                        <button
                            class="category-add"
                            aria-label="添加分类"
                            @click="openCategoryDialog('create', $event.currentTarget)"
                        >
                            <WallIcon name="add" :size="16" /><span>添加分类</span>
                        </button>
                    </div>
                    <div
                        v-if="categoryMenuOpen"
                        ref="categoryMenu"
                        class="category-action-menu"
                        role="menu"
                        @keydown="handleCategoryMenuKeydown"
                    >
                        <button
                            data-category-action="create"
                            role="menuitem"
                            @click="openCategoryDialog('create', $event.currentTarget)"
                        >
                            <WallIcon name="add" :size="18" />创建分类
                        </button>
                        <button
                            data-category-action="rename"
                            role="menuitem"
                            :disabled="!activeCategory"
                            @click="openCategoryDialog('rename', $event.currentTarget)"
                        >
                            <WallIcon name="settings" :size="18" />重命名分类
                        </button>
                        <button
                            class="danger-action"
                            data-category-action="delete"
                            role="menuitem"
                            :disabled="!activeCategory"
                            @click="openCategoryDialog('delete', $event.currentTarget)"
                        >
                            <WallIcon name="trash" :size="18" />删除分类
                        </button>
                    </div>
                </template>
                <div class="sidebar-spacer" />
                <div class="sidebar-status"><i :class="statusIndicator" />{{ statusText }}</div>
            </aside>
            <main class="main-content"><RouterView /></main>
        </div>
        <div v-if="wallStore.snapshot.playback.lastError" class="toast error-toast">
            {{ wallStore.snapshot.playback.lastError }}
        </div>
        <div v-if="wallStore.notice" class="toast success-toast" role="status" aria-live="polite">
            {{ wallStore.notice }}
        </div>
        <div v-if="appError" class="toast error-toast" role="alert" aria-live="assertive">{{ appError }}</div>
        <div v-if="categoryDialog" class="modal-scrim">
            <div
                ref="categoryDialogElement"
                class="dialog category-dialog"
                role="dialog"
                aria-modal="true"
                aria-labelledby="category-dialog-title"
                tabindex="-1"
                @keydown="trapDialogFocus"
            >
                <form :aria-busy="categoryBusy" @submit.prevent="submitCategoryDialog">
                    <h2 id="category-dialog-title">{{ categoryDialogTitle }}</h2>
                    <p v-if="categoryDialog === 'delete'">
                        删除“{{ activeCategory?.name }}”后，壁纸文件和媒体库条目都会保留。
                    </p>
                    <template v-else>
                        <p v-if="categoryDialog === 'create' && categoryAssignmentIds.length">
                            创建后会自动添加到{{ categoryAssignmentIds.length === 1 ? '当前壁纸' : '当前批量选择' }}。
                        </p>
                        <label>
                            分类名称
                            <input ref="categoryInput" v-model="categoryName" :disabled="categoryBusy" autofocus />
                        </label>
                    </template>
                    <p v-if="categoryError" class="inline-error">{{ categoryError }}</p>
                    <div class="dialog-actions">
                        <button
                            ref="categoryCancel"
                            type="button"
                            class="secondary"
                            data-category-cancel
                            :disabled="categoryBusy"
                            @click="closeCategoryDialog"
                        >
                            取消
                        </button>
                        <button
                            type="submit"
                            :class="categoryDialog === 'delete' ? 'danger' : 'primary'"
                            :disabled="categoryBusy"
                        >
                            {{ categoryDialogConfirmLabel }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    </div>
</template>
