<!-- 展示本地壁纸库、筛选、导入和卡片快捷播放。 -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { open } from '@tauri-apps/plugin-dialog';
import {
    importMedia,
    mediaUrl,
    play,
    removeMediaBatch,
    removeMissingMedia,
    scanLibrary,
    setCategoryMembership,
    setDisplayLayout,
} from '../api';
import { wallStore } from '../store';
import WallIcon from '../components/WallIcon.vue';
import type { DisplayMode, WallpaperItem } from '../types';

const router = useRouter();
const importing = ref(false);
const errorMessage = ref('');
const batchMenu = ref<'add' | 'remove' | null>(null);
const displayPanelOpen = ref(false);
const managementMenuOpen = ref(false);
const cleanupDialogOpen = ref(false);
const batchRemoveDialogOpen = ref(false);
const managementBusy = ref(false);
const removalBusy = ref(false);
const displayBusy = ref(false);
const displayError = ref('');
const quickPlayId = ref<string | null>(null);
const batchCategoryBusy = ref(false);
const draftDisplayMode = ref<DisplayMode>('independent');
const draftDisplayIds = ref<string[]>([]);
const managementTrigger = ref<HTMLButtonElement | null>(null);
const managementMenu = ref<HTMLElement | null>(null);
const managementWrap = ref<HTMLElement | null>(null);
const displayTrigger = ref<HTMLButtonElement | null>(null);
const displayWrap = ref<HTMLElement | null>(null);
const batchActions = ref<HTMLElement | null>(null);
const batchAddTrigger = ref<HTMLButtonElement | null>(null);
const batchRemoveCategoryTrigger = ref<HTMLButtonElement | null>(null);
const batchRemoveTrigger = ref<HTMLButtonElement | null>(null);
const importTrigger = ref<HTMLButtonElement | null>(null);
const cleanupCancel = ref<HTMLButtonElement | null>(null);
const batchRemovalCancel = ref<HTMLButtonElement | null>(null);
const importDialog = ref<HTMLElement | null>(null);
const cleanupDialog = ref<HTMLElement | null>(null);
const batchRemovalDialog = ref<HTMLElement | null>(null);
let removalReturnFocus: HTMLElement | null = null;
const library = computed(() => wallStore.filteredLibrary);
const selectedItems = computed(() =>
    wallStore.snapshot.library.filter((item) => wallStore.selectedMediaIds.includes(item.id)),
);
const missingItems = computed(() => wallStore.snapshot.library.filter((item) => item.missing));
const missingActiveCount = computed(() => missingItems.value.filter((item) => wallStore.isMediaActive(item.id)).length);
const selectedActiveCount = computed(
    () => selectedItems.value.filter((item) => wallStore.isMediaActive(item.id)).length,
);
const displays = computed(() => wallStore.snapshot.displays ?? []);
const modalOpen = computed(() => importing.value || cleanupDialogOpen.value || batchRemoveDialogOpen.value);
const displayTargetLabel = computed(() => {
    const selected = displays.value.filter((display) =>
        wallStore.snapshot.settings.selectedDisplayIds.includes(display.id),
    );
    if (wallStore.snapshot.settings.displayMode === 'clone') return `复制 · ${selected.length} 块屏幕`;
    if (wallStore.snapshot.settings.displayMode === 'span') return `铺展 · ${selected.length} 块屏幕`;
    return selected[0]?.name ?? displays.value.find((display) => display.primary)?.name ?? '选择显示器';
});
const validDraftDisplays = computed(() => {
    const ids = draftDisplayIds.value;
    const requiredCount = draftDisplayMode.value === 'independent' ? 1 : 2;
    if (ids.length < requiredCount || new Set(ids).size !== ids.length) return false;
    return ids.every((id) => displays.value.some((display) => display.id === id && display.connected));
});

onMounted(() => {
    document.addEventListener('keydown', handleDocumentKeydown);
    document.addEventListener('pointerdown', handleDocumentPointerDown);
});

onUnmounted(() => {
    document.removeEventListener('keydown', handleDocumentKeydown);
    document.removeEventListener('pointerdown', handleDocumentPointerDown);
});

async function chooseMedia() {
    if (importing.value || displayBusy.value) return;
    errorMessage.value = '';
    importing.value = true;
    nextTick(() => importDialog.value?.focus());
    try {
        const selected = await open({
            multiple: true,
            directory: false,
            filters: [
                { name: '视频', extensions: ['mp4', 'mkv', 'webm', 'mov', 'avi'] },
                { name: '图片', extensions: ['jpg', 'jpeg', 'png', 'webp', 'bmp', 'gif'] },
            ],
        });
        const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
        if (paths.length) {
            const previousCount = wallStore.snapshot.library.length;
            const snapshot = await importMedia(paths);
            const importedCount = Math.max(0, snapshot.library.length - previousCount);
            wallStore.showNotice(importedCount ? `已导入 ${importedCount} 项壁纸` : '所选壁纸已在库中');
        }
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        importing.value = false;
        nextTick(() => importTrigger.value?.focus());
    }
}

async function quickPlay(item: WallpaperItem) {
    if (quickPlayId.value) return;
    errorMessage.value = '';
    quickPlayId.value = item.id;
    try {
        await play(item.id);
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        quickPlayId.value = null;
    }
}

function openDetails(item: WallpaperItem) {
    router.push(`/wallpaper/${item.id}`);
}

function selectCard(item: WallpaperItem) {
    if (wallStore.batchMode && batchCategoryBusy.value) return;
    if (wallStore.batchMode) wallStore.toggleMediaSelection(item.id);
    else openDetails(item);
}

function canChangeCategory(categoryId: string, assigned: boolean): boolean {
    if (!selectedItems.value.length) return false;
    return assigned
        ? !selectedItems.value.every((item) => item.categoryIds.includes(categoryId))
        : selectedItems.value.some((item) => item.categoryIds.includes(categoryId));
}

async function changeCategory(categoryId: string, assigned: boolean) {
    if (batchCategoryBusy.value) return;
    const menu = batchMenu.value;
    const mediaIds = [...wallStore.selectedMediaIds];
    errorMessage.value = '';
    batchCategoryBusy.value = true;
    try {
        await setCategoryMembership(mediaIds, categoryId, assigned);
        batchMenu.value = null;
        const trigger = menu === 'add' ? batchAddTrigger.value : batchRemoveCategoryTrigger.value;
        nextTick(() => trigger?.focus());
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        batchCategoryBusy.value = false;
    }
}

function exitBatchMode() {
    if (batchCategoryBusy.value) return;
    batchMenu.value = null;
    batchRemoveDialogOpen.value = false;
    wallStore.exitBatchMode();
}

function enterBatchMode() {
    managementMenuOpen.value = false;
    wallStore.enterBatchMode();
}

function toggleManagementMenu() {
    if (displayBusy.value) return;
    displayPanelOpen.value = false;
    managementMenuOpen.value = !managementMenuOpen.value;
}

function openManagementMenuFromKeyboard(last: boolean) {
    if (displayBusy.value) return;
    displayPanelOpen.value = false;
    managementMenuOpen.value = true;
    nextTick(() => focusMenuEdge(managementMenu.value, last));
}

function closeManagementMenu(returnFocus = false) {
    managementMenuOpen.value = false;
    if (returnFocus) nextTick(() => managementTrigger.value?.focus());
}

function handleManagementMenuKeydown(event: KeyboardEvent) {
    handleMenuKeydown(event, () => closeManagementMenu(true));
}

function openBatchRemovalDialog() {
    batchMenu.value = null;
    errorMessage.value = '';
    removalReturnFocus = batchRemoveTrigger.value;
    batchRemoveDialogOpen.value = true;
    nextTick(() => batchRemovalCancel.value?.focus());
}

async function scanForCleanup() {
    if (managementBusy.value) return;
    managementMenuOpen.value = false;
    errorMessage.value = '';
    managementBusy.value = true;
    try {
        await scanLibrary();
        if (!missingItems.value.length) {
            wallStore.showNotice('没有发现失效项');
            return;
        }
        removalReturnFocus = managementTrigger.value;
        cleanupDialogOpen.value = true;
        nextTick(() => cleanupCancel.value?.focus());
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        managementBusy.value = false;
    }
}

async function confirmCleanup() {
    if (removalBusy.value) return;
    const beforeCount = wallStore.snapshot.library.length;
    removalBusy.value = true;
    nextTick(() => cleanupDialog.value?.focus());
    errorMessage.value = '';
    try {
        await removeMissingMedia();
        const count = beforeCount - wallStore.snapshot.library.length;
        cleanupDialogOpen.value = false;
        nextTick(() => managementTrigger.value?.focus());
        wallStore.showNotice(count ? `已清理 ${count} 个失效项` : '没有发现失效项');
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        removalBusy.value = false;
        if (cleanupDialogOpen.value) nextTick(() => cleanupCancel.value?.focus());
    }
}

async function confirmBatchRemoval() {
    if (removalBusy.value) return;
    const mediaIds = [...wallStore.selectedMediaIds];
    if (!mediaIds.length) return;
    removalBusy.value = true;
    nextTick(() => batchRemovalDialog.value?.focus());
    errorMessage.value = '';
    try {
        await removeMediaBatch(mediaIds);
        batchRemoveDialogOpen.value = false;
        wallStore.exitBatchMode();
        nextTick(() => managementTrigger.value?.focus());
        wallStore.showNotice(`已从壁纸库移除 ${mediaIds.length} 项`);
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        removalBusy.value = false;
        if (batchRemoveDialogOpen.value) nextTick(() => batchRemovalCancel.value?.focus());
    }
}

function openDisplayPanel() {
    if (displayBusy.value) return;
    managementMenuOpen.value = false;
    displayError.value = '';
    draftDisplayMode.value = wallStore.snapshot.settings.displayMode;
    draftDisplayIds.value = [...wallStore.snapshot.settings.selectedDisplayIds];
    if (!draftDisplayIds.value.length) {
        const primary = displays.value.find((display) => display.primary) ?? displays.value[0];
        if (primary) draftDisplayIds.value = [primary.id];
    }
    displayPanelOpen.value = !displayPanelOpen.value;
}

function closeDisplayPanel(returnFocus = false) {
    if (displayBusy.value) return;
    displayPanelOpen.value = false;
    displayError.value = '';
    if (returnFocus) nextTick(() => displayTrigger.value?.focus());
}

function chooseDisplayMode(mode: DisplayMode) {
    if (displayBusy.value) return;
    if (mode !== 'independent' && displays.value.filter((display) => display.connected).length < 2) return;
    draftDisplayMode.value = mode;
    if (mode === 'independent') {
        draftDisplayIds.value = [draftDisplayIds.value[0] ?? displays.value[0]?.id].filter(Boolean) as string[];
    } else if (draftDisplayIds.value.length < 2) {
        draftDisplayIds.value = displays.value.filter((display) => display.connected).map((display) => display.id);
    }
}

function toggleDisplay(displayId: string) {
    if (displayBusy.value) return;
    if (draftDisplayMode.value === 'independent') {
        draftDisplayIds.value = [displayId];
        return;
    }
    draftDisplayIds.value = draftDisplayIds.value.includes(displayId)
        ? draftDisplayIds.value.filter((id) => id !== displayId)
        : [...draftDisplayIds.value, displayId];
}

async function applyDisplayLayout() {
    if (!validDraftDisplays.value || displayBusy.value) return;
    displayError.value = '';
    displayBusy.value = true;
    try {
        await setDisplayLayout(draftDisplayMode.value, [...draftDisplayIds.value]);
        displayPanelOpen.value = false;
        nextTick(() => displayTrigger.value?.focus());
    } catch (error) {
        displayError.value = readError(error);
    } finally {
        displayBusy.value = false;
    }
}

function moveDisplayMode(event: KeyboardEvent, direction: -1 | 1) {
    if (displayBusy.value) return;
    const current = event.currentTarget;
    if (!(current instanceof HTMLButtonElement) || !current.parentElement) return;
    const buttons = Array.from(current.parentElement.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
    const currentIndex = buttons.indexOf(current);
    if (currentIndex < 0 || buttons.length < 2) return;
    const next = buttons[(currentIndex + direction + buttons.length) % buttons.length];
    const mode = next.dataset.displayMode as DisplayMode | undefined;
    if (!mode) return;
    chooseDisplayMode(mode);
    next.focus();
}

function setBatchMenu(kind: 'add' | 'remove') {
    if (batchCategoryBusy.value) return;
    batchMenu.value = batchMenu.value === kind ? null : kind;
}

function openBatchMenuFromKeyboard(kind: 'add' | 'remove', last: boolean) {
    if (batchCategoryBusy.value) return;
    batchMenu.value = kind;
    nextTick(() => {
        const selector = `[data-batch-menu="${kind}"]`;
        focusMenuEdge(document.querySelector<HTMLElement>(selector), last);
    });
}

function handleBatchMenuKeydown(event: KeyboardEvent) {
    handleMenuKeydown(event, () => {
        const trigger = batchMenu.value === 'add' ? batchAddTrigger.value : batchRemoveCategoryTrigger.value;
        batchMenu.value = null;
        nextTick(() => trigger?.focus());
    });
}

function handleMenuKeydown(event: KeyboardEvent, close: () => void) {
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
        close();
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

function closeCleanupDialog() {
    if (removalBusy.value) return;
    cleanupDialogOpen.value = false;
    errorMessage.value = '';
    nextTick(() => removalReturnFocus?.focus());
}

function closeBatchRemovalDialog() {
    if (removalBusy.value) return;
    batchRemoveDialogOpen.value = false;
    errorMessage.value = '';
    nextTick(() => removalReturnFocus?.focus());
}

function trapDialogFocus(event: KeyboardEvent) {
    if (event.key !== 'Tab') return;
    const dialog = event.currentTarget;
    if (!(dialog instanceof HTMLElement)) return;
    const items = Array.from(
        dialog.querySelectorAll<HTMLElement>(
            'button:not(:disabled), input:not(:disabled), select:not(:disabled), [tabindex="0"]',
        ),
    );
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
    if (cleanupDialogOpen.value) closeCleanupDialog();
    else if (batchRemoveDialogOpen.value) closeBatchRemovalDialog();
    else if (batchMenu.value && !batchCategoryBusy.value) {
        const trigger = batchMenu.value === 'add' ? batchAddTrigger.value : batchRemoveCategoryTrigger.value;
        batchMenu.value = null;
        nextTick(() => trigger?.focus());
    } else if (managementMenuOpen.value) closeManagementMenu(true);
    else if (displayPanelOpen.value) closeDisplayPanel(true);
}

function handleDocumentPointerDown(event: PointerEvent) {
    const target = event.target;
    if (!(target instanceof Node)) return;
    if (managementMenuOpen.value && !managementWrap.value?.contains(target)) closeManagementMenu(true);
    if (displayPanelOpen.value && !displayWrap.value?.contains(target) && !displayBusy.value) {
        closeDisplayPanel(true);
    }
    if (batchMenu.value && !batchCategoryBusy.value && !batchActions.value?.contains(target)) {
        const trigger = batchMenu.value === 'add' ? batchAddTrigger.value : batchRemoveCategoryTrigger.value;
        batchMenu.value = null;
        nextTick(() => trigger?.focus());
    }
}

function readError(error: unknown): string {
    if (typeof error === 'object' && error && 'message' in error) return String(error.message);
    return String(error);
}
</script>

<template>
    <section class="page library-page">
        <div class="page-modal-background" :inert="modalOpen ? true : undefined">
            <div class="page-heading">
                <h1>{{ wallStore.batchMode ? '批量管理' : '壁纸库' }}</h1>
                <div v-if="wallStore.batchMode" ref="batchActions" class="heading-actions batch-actions">
                    <span>已选 {{ wallStore.selectedMediaIds.length }} 项</span>
                    <div class="batch-action-wrap">
                        <button
                            ref="batchAddTrigger"
                            class="primary"
                            data-batch-action="add"
                            aria-haspopup="menu"
                            :aria-expanded="batchMenu === 'add'"
                            :disabled="batchCategoryBusy || !wallStore.selectedMediaIds.length"
                            @click="setBatchMenu('add')"
                            @keydown.down.prevent="openBatchMenuFromKeyboard('add', false)"
                            @keydown.up.prevent="openBatchMenuFromKeyboard('add', true)"
                        >
                            添加到分类
                        </button>
                        <div
                            v-if="batchMenu === 'add'"
                            class="batch-category-menu"
                            data-batch-menu="add"
                            role="menu"
                            @keydown="handleBatchMenuKeydown"
                        >
                            <button
                                v-for="category in wallStore.snapshot.categories"
                                :key="category.id"
                                :aria-label="`添加到${category.name}`"
                                role="menuitem"
                                :disabled="batchCategoryBusy || !canChangeCategory(category.id, true)"
                                @click="changeCategory(category.id, true)"
                            >
                                <span>{{ category.name }}</span>
                                <small v-if="!canChangeCategory(category.id, true)">已包含</small>
                            </button>
                        </div>
                    </div>
                    <div class="batch-action-wrap">
                        <button
                            ref="batchRemoveCategoryTrigger"
                            class="secondary"
                            data-batch-action="remove"
                            aria-haspopup="menu"
                            :aria-expanded="batchMenu === 'remove'"
                            :disabled="batchCategoryBusy || !wallStore.selectedMediaIds.length"
                            @click="setBatchMenu('remove')"
                            @keydown.down.prevent="openBatchMenuFromKeyboard('remove', false)"
                            @keydown.up.prevent="openBatchMenuFromKeyboard('remove', true)"
                        >
                            从分类移除
                        </button>
                        <div
                            v-if="batchMenu === 'remove'"
                            class="batch-category-menu remove-menu"
                            data-batch-menu="remove"
                            role="menu"
                            @keydown="handleBatchMenuKeydown"
                        >
                            <button
                                v-for="category in wallStore.snapshot.categories"
                                :key="category.id"
                                :aria-label="`从${category.name}移除`"
                                role="menuitem"
                                :disabled="batchCategoryBusy || !canChangeCategory(category.id, false)"
                                @click="changeCategory(category.id, false)"
                            >
                                <span>{{ category.name }}</span>
                                <small v-if="!canChangeCategory(category.id, false)">未包含</small>
                            </button>
                        </div>
                    </div>
                    <button
                        ref="batchRemoveTrigger"
                        class="batch-remove-action"
                        data-batch-action="remove-library"
                        :disabled="batchCategoryBusy || !wallStore.selectedMediaIds.length"
                        @click="openBatchRemovalDialog"
                    >
                        <WallIcon name="trash" :size="16" />移出库
                    </button>
                    <button
                        class="secondary"
                        data-batch-action="cancel"
                        :disabled="batchCategoryBusy"
                        @click="exitBatchMode"
                    >
                        取消
                    </button>
                </div>
                <div v-else class="heading-actions">
                    <label class="search-field">
                        <WallIcon name="search" :size="16" />
                        <input v-model="wallStore.search" placeholder="搜索本地壁纸" />
                    </label>
                    <div ref="displayWrap" class="display-target-wrap">
                        <button
                            ref="displayTrigger"
                            class="secondary display-target-button"
                            aria-label="选择显示器"
                            aria-haspopup="dialog"
                            aria-controls="display-selector-panel"
                            :aria-expanded="displayPanelOpen"
                            @click="openDisplayPanel"
                        >
                            <WallIcon name="monitor" :size="18" /><span>{{ displayTargetLabel }}</span>
                            <WallIcon name="chevron-down" :size="16" />
                        </button>
                        <div
                            v-if="displayPanelOpen"
                            id="display-selector-panel"
                            class="display-selector-panel"
                            role="dialog"
                            aria-label="显示目标"
                            :aria-busy="displayBusy"
                        >
                            <div class="display-panel-heading">
                                <h2>显示目标</h2>
                                <button
                                    class="dialog-close"
                                    aria-label="关闭显示目标"
                                    :disabled="displayBusy"
                                    @click="closeDisplayPanel(true)"
                                >
                                    <WallIcon name="close" :size="18" />
                                </button>
                            </div>
                            <div class="display-mode-selector">
                                <button
                                    v-for="mode in ['independent', 'clone', 'span'] as DisplayMode[]"
                                    :key="mode"
                                    :data-display-mode="mode"
                                    :class="{ active: draftDisplayMode === mode }"
                                    :aria-pressed="draftDisplayMode === mode"
                                    :disabled="
                                        displayBusy ||
                                        (mode !== 'independent' &&
                                            displays.filter((display) => display.connected).length < 2)
                                    "
                                    @click="chooseDisplayMode(mode)"
                                    @keydown.left.prevent="moveDisplayMode($event, -1)"
                                    @keydown.right.prevent="moveDisplayMode($event, 1)"
                                >
                                    {{ mode === 'independent' ? '独立' : mode === 'clone' ? '复制' : '铺展' }}
                                </button>
                            </div>
                            <div class="monitor-list">
                                <button
                                    v-for="display in displays"
                                    :key="display.id"
                                    class="monitor-card"
                                    :class="{
                                        selected: draftDisplayIds.includes(display.id),
                                        offline: !display.connected,
                                    }"
                                    :disabled="displayBusy || !display.connected"
                                    @click="toggleDisplay(display.id)"
                                >
                                    <WallIcon name="monitor" :size="24" />
                                    <span
                                        ><strong>{{ display.name }}</strong
                                        ><small>{{ display.width }} × {{ display.height }}</small></span
                                    >
                                    <em v-if="display.primary">主屏</em>
                                    <WallIcon v-if="draftDisplayIds.includes(display.id)" name="check" :size="16" />
                                </button>
                            </div>
                            <p v-if="displays.filter((display) => display.connected).length < 2">
                                连接至少两块在线屏幕后可使用复制和铺展。
                            </p>
                            <p v-if="displayError" class="inline-error" role="alert">{{ displayError }}</p>
                            <div class="display-panel-actions">
                                <button
                                    class="secondary"
                                    data-display-action="cancel"
                                    :disabled="displayBusy"
                                    @click="closeDisplayPanel(true)"
                                >
                                    取消
                                </button>
                                <button
                                    class="primary"
                                    data-display-action="apply"
                                    :disabled="displayBusy || !validDraftDisplays"
                                    @click="applyDisplayLayout"
                                >
                                    {{ displayBusy ? '正在应用…' : '应用' }}
                                </button>
                            </div>
                        </div>
                    </div>
                    <div ref="managementWrap" class="library-management-wrap">
                        <button
                            ref="managementTrigger"
                            class="secondary button-medium library-management-button"
                            aria-label="管理壁纸库"
                            aria-haspopup="menu"
                            :aria-expanded="managementMenuOpen"
                            :aria-busy="managementBusy"
                            :disabled="managementBusy || displayBusy"
                            @click="toggleManagementMenu"
                            @keydown.down.prevent="openManagementMenuFromKeyboard(false)"
                            @keydown.up.prevent="openManagementMenuFromKeyboard(true)"
                        >
                            {{ managementBusy ? '正在扫描…' : '管理' }}
                            <WallIcon v-if="!managementBusy" name="chevron-down" :size="16" />
                        </button>
                        <div
                            v-if="managementMenuOpen"
                            ref="managementMenu"
                            class="library-management-menu"
                            role="menu"
                            @keydown="handleManagementMenuKeydown"
                        >
                            <button data-library-action="batch" role="menuitem" @click="enterBatchMode">
                                <WallIcon name="check" :size="18" />批量管理
                            </button>
                            <button
                                class="danger-action"
                                data-library-action="cleanup"
                                role="menuitem"
                                @click="scanForCleanup"
                            >
                                <WallIcon name="trash" :size="18" />清理失效项
                            </button>
                        </div>
                    </div>
                    <button
                        ref="importTrigger"
                        class="primary button-medium"
                        data-library-action="import"
                        :disabled="importing || displayBusy"
                        @click="chooseMedia"
                    >
                        {{ importing ? '正在导入…' : '导入壁纸' }}
                    </button>
                </div>
            </div>
            <div class="tabs library-tabs">
                <button
                    v-for="option in [
                        ['all', '全部'],
                        ['video', '视频'],
                        ['image', '图片'],
                    ] as const"
                    :key="option[0]"
                    :class="{ active: wallStore.filter === option[0] }"
                    @click="wallStore.filter = option[0]"
                >
                    {{ option[1] }}
                </button>
            </div>

            <div v-if="!library.length" class="empty-area">
                <div v-if="!wallStore.snapshot.library.length" class="empty-state">
                    <WallIcon name="info" :size="32" />
                    <h2>还没有壁纸</h2>
                    <p>导入本地视频或图片开始使用</p>
                </div>
                <div
                    v-else-if="wallStore.activeCategoryId && !wallStore.search.trim() && wallStore.filter === 'all'"
                    class="empty-state compact-empty"
                >
                    <WallIcon name="info" :size="32" />
                    <h2>此分类还没有壁纸</h2>
                    <p>通过批量管理将壁纸添加到此分类</p>
                </div>
                <div v-else class="empty-state compact-empty">
                    <WallIcon name="search" :size="32" />
                    <h2>没有搜索结果</h2>
                    <p>尝试其他文件名或清除筛选</p>
                </div>
            </div>
            <div v-else class="wallpaper-grid">
                <article
                    v-for="item in library"
                    :key="item.id"
                    class="wallpaper-card"
                    :class="{
                        active: wallStore.isMediaActive(item.id),
                        missing: item.missing,
                        selected: wallStore.selectedMediaIds.includes(item.id),
                    }"
                    :aria-selected="wallStore.batchMode ? wallStore.selectedMediaIds.includes(item.id) : undefined"
                    :aria-disabled="wallStore.batchMode && batchCategoryBusy"
                    tabindex="0"
                    @click="selectCard(item)"
                    @keydown.enter.self="selectCard(item)"
                    @keydown.space.self.prevent="selectCard(item)"
                >
                    <div class="card-preview" :class="`preview-${item.kind}`">
                        <img v-if="item.kind === 'image' && mediaUrl(item.path)" :src="mediaUrl(item.path)" alt="" />
                        <video
                            v-else-if="item.kind === 'video' && mediaUrl(item.path)"
                            :src="mediaUrl(item.path)"
                            muted
                            preload="metadata"
                        />
                        <button
                            v-if="!wallStore.batchMode && !item.missing"
                            type="button"
                            class="card-quick-play"
                            data-card-action="quick-play"
                            :aria-label="`将 ${item.name} 设为壁纸`"
                            title="设为壁纸"
                            :aria-busy="quickPlayId === item.id"
                            :disabled="quickPlayId !== null"
                            @click.stop="quickPlay(item)"
                        >
                            <WallIcon name="play" :size="18" />
                        </button>
                        <span v-if="wallStore.batchMode" class="card-checkbox">
                            <WallIcon v-if="wallStore.selectedMediaIds.includes(item.id)" name="check" :size="14" />
                        </span>
                    </div>
                    <div class="card-copy">
                        <strong>{{ item.name }}</strong
                        ><small>
                            {{ item.kind.toUpperCase() }}<template v-if="item.missing"> · 文件丢失</template
                            ><template v-else-if="wallStore.isMediaActive(item.id)"> · 正在运行</template
                            ><template v-else-if="item.width"> · {{ item.width }} × {{ item.height }}</template>
                        </small>
                    </div>
                </article>
            </div>

            <p v-if="errorMessage" class="inline-error" role="alert">{{ errorMessage }}</p>
        </div>
        <div v-if="importing" class="modal-scrim">
            <div
                ref="importDialog"
                class="dialog"
                role="dialog"
                aria-modal="true"
                aria-label="正在导入壁纸"
                tabindex="-1"
                @keydown="trapDialogFocus"
            >
                <h2>正在导入壁纸</h2>
                <p>正在读取本地媒体信息，请稍候…</p>
                <div class="progress"><i /></div>
            </div>
        </div>
        <div v-if="cleanupDialogOpen" class="modal-scrim">
            <div
                ref="cleanupDialog"
                class="dialog removal-dialog"
                data-removal-kind="cleanup"
                role="alertdialog"
                aria-modal="true"
                aria-labelledby="cleanup-dialog-title"
                :aria-busy="removalBusy"
                tabindex="-1"
                @keydown="trapDialogFocus"
            >
                <button
                    class="dialog-close"
                    aria-label="关闭清理确认"
                    :disabled="removalBusy"
                    @click="closeCleanupDialog"
                >
                    <WallIcon name="close" :size="18" />
                </button>
                <h2 id="cleanup-dialog-title">清理 {{ missingItems.length }} 个失效项？</h2>
                <p>已重新扫描整个壁纸库；源文件不会被删除或修改。</p>
                <div v-if="missingActiveCount" class="removal-warning">
                    <WallIcon name="warning" :size="18" />
                    <span>其中 {{ missingActiveCount }} 项仍分配到屏幕；继续后将停止相关屏幕。</span>
                </div>
                <p v-if="errorMessage" class="inline-error" role="alert">{{ errorMessage }}</p>
                <div class="dialog-actions">
                    <button
                        ref="cleanupCancel"
                        class="secondary"
                        data-removal-cancel
                        autofocus
                        :disabled="removalBusy"
                        @click="closeCleanupDialog"
                    >
                        取消
                    </button>
                    <button class="danger" data-removal-confirm :disabled="removalBusy" @click="confirmCleanup">
                        移除 {{ missingItems.length }} 项
                    </button>
                </div>
            </div>
        </div>
        <div v-if="batchRemoveDialogOpen" class="modal-scrim">
            <div
                ref="batchRemovalDialog"
                class="dialog removal-dialog"
                data-removal-kind="batch"
                role="alertdialog"
                aria-modal="true"
                aria-labelledby="batch-removal-dialog-title"
                :aria-busy="removalBusy"
                tabindex="-1"
                @keydown="trapDialogFocus"
            >
                <button
                    class="dialog-close"
                    aria-label="关闭批量移除确认"
                    :disabled="removalBusy"
                    @click="closeBatchRemovalDialog"
                >
                    <WallIcon name="close" :size="18" />
                </button>
                <h2 id="batch-removal-dialog-title">从壁纸库移除 {{ selectedItems.length }} 项？</h2>
                <p>只移除 Wall 的记录，不会删除或修改任何源文件。</p>
                <div v-if="selectedActiveCount" class="removal-warning">
                    <WallIcon name="warning" :size="18" />
                    <span>其中 {{ selectedActiveCount }} 项仍分配到屏幕；继续后将停止相关屏幕。</span>
                </div>
                <p v-if="errorMessage" class="inline-error" role="alert">{{ errorMessage }}</p>
                <div class="dialog-actions">
                    <button
                        ref="batchRemovalCancel"
                        class="secondary"
                        data-removal-cancel
                        autofocus
                        :disabled="removalBusy"
                        @click="closeBatchRemovalDialog"
                    >
                        取消
                    </button>
                    <button class="danger" data-removal-confirm :disabled="removalBusy" @click="confirmBatchRemoval">
                        移除 {{ selectedItems.length }} 项
                    </button>
                </div>
            </div>
        </div>
    </section>
</template>
