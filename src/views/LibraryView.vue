<!-- 展示本地壁纸库、筛选、导入和卡片快捷播放。 -->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { open } from '@tauri-apps/plugin-dialog';
import { importMedia, mediaUrl, play, setCategoryMembership, setDisplayLayout } from '../api';
import { wallStore } from '../store';
import WallIcon from '../components/WallIcon.vue';
import type { DisplayMode, WallpaperItem } from '../types';

const router = useRouter();
const importing = ref(false);
const errorMessage = ref('');
const batchMenu = ref<'add' | 'remove' | null>(null);
const displayPanelOpen = ref(false);
const draftDisplayMode = ref<DisplayMode>('independent');
const draftDisplayIds = ref<string[]>([]);
const library = computed(() => wallStore.filteredLibrary);
const selectedItems = computed(() =>
    wallStore.snapshot.library.filter((item) => wallStore.selectedMediaIds.includes(item.id)),
);
const displays = computed(() => wallStore.snapshot.displays ?? []);
const displayTargetLabel = computed(() => {
    const selected = displays.value.filter((display) =>
        wallStore.snapshot.settings.selectedDisplayIds.includes(display.id),
    );
    if (wallStore.snapshot.settings.displayMode === 'clone') return `复制 · ${selected.length} 块屏幕`;
    if (wallStore.snapshot.settings.displayMode === 'span') return `铺展 · ${selected.length} 块屏幕`;
    return selected[0]?.name ?? displays.value.find((display) => display.primary)?.name ?? '选择显示器';
});
const validDraftDisplays = computed(() =>
    draftDisplayMode.value === 'independent' ? draftDisplayIds.value.length === 1 : draftDisplayIds.value.length >= 2,
);

async function chooseMedia() {
    errorMessage.value = '';
    const selected = await open({
        multiple: true,
        directory: false,
        filters: [
            { name: '视频', extensions: ['mp4', 'mkv', 'webm', 'mov', 'avi'] },
            { name: '图片', extensions: ['jpg', 'jpeg', 'png', 'webp', 'bmp', 'gif'] },
        ],
    });
    const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
    if (!paths.length) return;
    importing.value = true;
    try {
        await importMedia(paths);
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        importing.value = false;
    }
}

async function quickPlay(item: WallpaperItem) {
    errorMessage.value = '';
    try {
        await play(item.id);
    } catch (error) {
        errorMessage.value = readError(error);
    }
}

function openDetails(item: WallpaperItem) {
    router.push(`/wallpaper/${item.id}`);
}

function selectCard(item: WallpaperItem) {
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
    errorMessage.value = '';
    try {
        await setCategoryMembership([...wallStore.selectedMediaIds], categoryId, assigned);
        batchMenu.value = null;
    } catch (error) {
        errorMessage.value = readError(error);
    }
}

function exitBatchMode() {
    batchMenu.value = null;
    wallStore.exitBatchMode();
}

function openDisplayPanel() {
    draftDisplayMode.value = wallStore.snapshot.settings.displayMode;
    draftDisplayIds.value = [...wallStore.snapshot.settings.selectedDisplayIds];
    if (!draftDisplayIds.value.length) {
        const primary = displays.value.find((display) => display.primary) ?? displays.value[0];
        if (primary) draftDisplayIds.value = [primary.id];
    }
    displayPanelOpen.value = !displayPanelOpen.value;
}

function chooseDisplayMode(mode: DisplayMode) {
    if (mode !== 'independent' && displays.value.filter((display) => display.connected).length < 2) return;
    draftDisplayMode.value = mode;
    if (mode === 'independent') {
        draftDisplayIds.value = [draftDisplayIds.value[0] ?? displays.value[0]?.id].filter(Boolean) as string[];
    } else if (draftDisplayIds.value.length < 2) {
        draftDisplayIds.value = displays.value.filter((display) => display.connected).map((display) => display.id);
    }
}

function toggleDisplay(displayId: string) {
    if (draftDisplayMode.value === 'independent') {
        draftDisplayIds.value = [displayId];
        return;
    }
    draftDisplayIds.value = draftDisplayIds.value.includes(displayId)
        ? draftDisplayIds.value.filter((id) => id !== displayId)
        : [...draftDisplayIds.value, displayId];
}

async function applyDisplayLayout() {
    if (!validDraftDisplays.value) return;
    await actionDisplay(() => setDisplayLayout(draftDisplayMode.value, [...draftDisplayIds.value]));
    displayPanelOpen.value = false;
}

async function actionDisplay(task: () => Promise<unknown>) {
    errorMessage.value = '';
    try {
        await task();
    } catch (error) {
        errorMessage.value = readError(error);
    }
}

function readError(error: unknown): string {
    if (typeof error === 'object' && error && 'message' in error) return String(error.message);
    return String(error);
}
</script>

<template>
    <section class="page library-page">
        <div class="page-heading">
            <h1>{{ wallStore.batchMode ? '批量管理' : '壁纸库' }}</h1>
            <div v-if="wallStore.batchMode" class="heading-actions batch-actions">
                <span>已选 {{ wallStore.selectedMediaIds.length }} 项</span>
                <div class="batch-action-wrap">
                    <button
                        class="primary"
                        data-batch-action="add"
                        :disabled="!wallStore.selectedMediaIds.length"
                        @click="batchMenu = batchMenu === 'add' ? null : 'add'"
                    >
                        添加到分类
                    </button>
                    <div v-if="batchMenu === 'add'" class="batch-category-menu">
                        <button
                            v-for="category in wallStore.snapshot.categories"
                            :key="category.id"
                            :aria-label="`添加到${category.name}`"
                            :disabled="!canChangeCategory(category.id, true)"
                            @click="changeCategory(category.id, true)"
                        >
                            <span>{{ category.name }}</span>
                            <small v-if="!canChangeCategory(category.id, true)">已包含</small>
                        </button>
                    </div>
                </div>
                <div class="batch-action-wrap">
                    <button
                        class="secondary"
                        data-batch-action="remove"
                        :disabled="!wallStore.selectedMediaIds.length"
                        @click="batchMenu = batchMenu === 'remove' ? null : 'remove'"
                    >
                        从分类移除
                    </button>
                    <div v-if="batchMenu === 'remove'" class="batch-category-menu remove-menu">
                        <button
                            v-for="category in wallStore.snapshot.categories"
                            :key="category.id"
                            :aria-label="`从${category.name}移除`"
                            :disabled="!canChangeCategory(category.id, false)"
                            @click="changeCategory(category.id, false)"
                        >
                            <span>{{ category.name }}</span>
                            <small v-if="!canChangeCategory(category.id, false)">未包含</small>
                        </button>
                    </div>
                </div>
                <button class="secondary" @click="exitBatchMode">取消</button>
            </div>
            <div v-else class="heading-actions">
                <label class="search-field">
                    <WallIcon name="search" :size="16" />
                    <input v-model="wallStore.search" placeholder="搜索本地壁纸" />
                </label>
                <div class="display-target-wrap">
                    <button class="secondary display-target-button" aria-label="选择显示器" @click="openDisplayPanel">
                        <WallIcon name="monitor" :size="18" /><span>{{ displayTargetLabel }}</span>
                        <WallIcon name="chevron-down" :size="16" />
                    </button>
                    <div v-if="displayPanelOpen" class="display-selector-panel">
                        <div class="display-mode-selector">
                            <button
                                v-for="mode in ['independent', 'clone', 'span'] as DisplayMode[]"
                                :key="mode"
                                :data-display-mode="mode"
                                :class="{ active: draftDisplayMode === mode }"
                                :disabled="
                                    mode !== 'independent' && displays.filter((display) => display.connected).length < 2
                                "
                                @click="chooseDisplayMode(mode)"
                            >
                                {{ mode === 'independent' ? '独立' : mode === 'clone' ? '复制' : '铺展' }}
                            </button>
                        </div>
                        <div class="monitor-list">
                            <button
                                v-for="display in displays"
                                :key="display.id"
                                class="monitor-card"
                                :class="{ selected: draftDisplayIds.includes(display.id), offline: !display.connected }"
                                :disabled="!display.connected"
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
                        <p v-if="displays.length < 2">连接至少两块屏幕后可使用复制和铺展。</p>
                        <div class="display-panel-actions">
                            <button class="secondary" @click="displayPanelOpen = false">取消</button>
                            <button
                                class="primary"
                                data-display-action="apply"
                                :disabled="!validDraftDisplays"
                                @click="applyDisplayLayout"
                            >
                                应用
                            </button>
                        </div>
                    </div>
                </div>
                <button class="primary button-medium" @click="chooseMedia">导入壁纸</button>
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
                    active: wallStore.snapshot.playback.activeId === item.id,
                    missing: item.missing,
                    selected: wallStore.selectedMediaIds.includes(item.id),
                }"
                :aria-selected="wallStore.batchMode ? wallStore.selectedMediaIds.includes(item.id) : undefined"
                tabindex="0"
                @click="selectCard(item)"
                @keydown.enter="selectCard(item)"
                @keydown.space.prevent="selectCard(item)"
                @dblclick.stop="!wallStore.batchMode && quickPlay(item)"
            >
                <div class="card-preview" :class="`preview-${item.kind}`">
                    <img v-if="item.kind === 'image' && mediaUrl(item.path)" :src="mediaUrl(item.path)" alt="" />
                    <video
                        v-else-if="item.kind === 'video' && mediaUrl(item.path)"
                        :src="mediaUrl(item.path)"
                        muted
                        preload="metadata"
                    />
                    <span v-if="wallStore.batchMode" class="card-checkbox">
                        <WallIcon v-if="wallStore.selectedMediaIds.includes(item.id)" name="check" :size="14" />
                    </span>
                </div>
                <div class="card-copy">
                    <strong>{{ item.name }}</strong
                    ><small>
                        {{ item.kind.toUpperCase() }}<template v-if="item.missing"> · 文件丢失</template
                        ><template v-else-if="wallStore.snapshot.playback.activeId === item.id"> · 正在运行</template
                        ><template v-else-if="item.width"> · {{ item.width }} × {{ item.height }}</template>
                    </small>
                </div>
            </article>
        </div>

        <p v-if="errorMessage" class="inline-error">{{ errorMessage }}</p>
        <div v-if="importing" class="modal-scrim">
            <div class="dialog">
                <h2>正在导入壁纸</h2>
                <p>正在读取本地媒体信息，请稍候…</p>
                <div class="progress"><i /></div>
            </div>
        </div>
    </section>
</template>
