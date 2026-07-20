<!-- 展示单个壁纸的本地预览、播放控制和文件恢复操作。 -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { open } from '@tauri-apps/plugin-dialog';
import {
    mediaUrl,
    openMediaFolder,
    play,
    relocateMedia,
    removeMedia,
    setCategoryMembership,
    setWallpaperSettings,
    stopMedia,
    toggleMediaPause,
} from '../api';
import { wallStore } from '../store';
import WallIcon from '../components/WallIcon.vue';
import type { AppSettings, WallpaperSettings } from '../types';

const route = useRoute();
const router = useRouter();
const errorMessage = ref('');
const categoryEditorOpen = ref(false);
const removeDialogOpen = ref(false);
const removing = ref(false);
const actionBusy = ref(false);
const settingsBusy = ref(false);
const categoryBusy = ref(false);
const previewVideo = ref<HTMLVideoElement | null>(null);
const previewPlaying = ref(false);
const categoryTrigger = ref<HTMLButtonElement | null>(null);
const categoryMenu = ref<HTMLElement | null>(null);
const removeTrigger = ref<HTMLButtonElement | null>(null);
const removeCancel = ref<HTMLButtonElement | null>(null);
const removalDialog = ref<HTMLElement | null>(null);
const scaleModes = [
    { value: 'cover', label: '填充' },
    { value: 'contain', label: '适应' },
    { value: 'stretch', label: '拉伸' },
] as const;
const item = computed(() => wallStore.snapshot.library.find((entry) => entry.id === route.params.id) ?? null);
const activeAssignments = computed(() =>
    (wallStore.snapshot.playback.displayAssignments ?? []).filter(
        (assignment) => assignment.wallpaperId === item.value?.id,
    ),
);
const legacyActive = computed(
    () => activeAssignments.value.length === 0 && wallStore.snapshot.playback.activeId === item.value?.id,
);
const active = computed(() => activeAssignments.value.length > 0 || legacyActive.value);
const paused = computed(() =>
    activeAssignments.value.length > 0
        ? activeAssignments.value.every((assignment) => assignment.status === 'paused')
        : legacyActive.value && wallStore.snapshot.playback.status === 'paused',
);
const muted = computed(() =>
    activeAssignments.value.length > 0
        ? activeAssignments.value.every((assignment) => assignment.muted)
        : wallStore.snapshot.playback.muted,
);
const targetLabels = computed(() =>
    activeAssignments.value.map((assignment) => {
        const mode = { independent: '独立', clone: '复制', span: '铺展' }[assignment.mode];
        const displays = assignment.displayIds.map(
            (id) => wallStore.snapshot.displays?.find((display) => display.id === id)?.name ?? id,
        );
        return `${mode} · ${displays.join(' + ')}`;
    }),
);
const overrides = computed(() => item.value?.settings ?? {});
const hasOverrides = computed(() => Object.keys(overrides.value).length > 0);
const effectiveSettings = computed(() => {
    const global = wallStore.snapshot.settings;
    const local = overrides.value;
    return {
        scaleMode: local.scaleMode ?? global.scaleMode,
        aspectRatio: local.aspectRatio ?? global.aspectRatio,
        antiAliasing: local.antiAliasing ?? global.antiAliasing,
        frameRate: local.frameRate ?? global.frameRate,
        hardwareDecoding: local.hardwareDecoding ?? global.hardwareDecoding,
        muted: local.muted ?? global.defaultMuted,
        volume: local.volume ?? global.volume,
    };
});
const itemCategories = computed(() =>
    wallStore.snapshot.categories.filter((category) => item.value?.categoryIds.includes(category.id)),
);

onMounted(() => {
    document.addEventListener('keydown', handleDocumentKeydown);
    document.addEventListener('pointerdown', handleDocumentPointerDown);
});

onUnmounted(() => {
    document.removeEventListener('keydown', handleDocumentKeydown);
    document.removeEventListener('pointerdown', handleDocumentPointerDown);
});

async function action(task: () => Promise<unknown>) {
    if (actionBusy.value) return;
    errorMessage.value = '';
    actionBusy.value = true;
    try {
        await task();
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        actionBusy.value = false;
    }
}

async function relocate() {
    if (!item.value) return;
    await action(async () => {
        const path = await open({ multiple: false, directory: false });
        if (typeof path === 'string') await relocateMedia(item.value!.id, path);
    });
}

function duration(seconds: number | null, kind: 'video' | 'image'): string {
    if (seconds === null) return kind === 'video' ? '时长将在播放后读取' : '静态图片';
    return `${Math.floor(seconds / 60)
        .toString()
        .padStart(2, '0')}:${Math.floor(seconds % 60)
        .toString()
        .padStart(2, '0')}`;
}

function readError(error: unknown): string {
    if (typeof error === 'object' && error && 'message' in error) return String(error.message);
    return String(error);
}

async function changeOverride<K extends keyof WallpaperSettings>(key: K, value: WallpaperSettings[K]) {
    if (!item.value || settingsBusy.value) return;
    settingsBusy.value = true;
    errorMessage.value = '';
    try {
        await setWallpaperSettings(item.value.id, { ...overrides.value, [key]: value });
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        settingsBusy.value = false;
    }
}

async function restoreGlobalSettings() {
    if (!item.value || settingsBusy.value) return;
    settingsBusy.value = true;
    errorMessage.value = '';
    try {
        await setWallpaperSettings(item.value.id, {});
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        settingsBusy.value = false;
    }
}

function moveScaleMode(event: KeyboardEvent, direction: -1 | 1) {
    if (settingsBusy.value) return;
    const current = event.currentTarget;
    if (!(current instanceof HTMLButtonElement) || !current.parentElement) return;
    const buttons = Array.from(current.parentElement.querySelectorAll<HTMLButtonElement>('button'));
    const currentIndex = buttons.indexOf(current);
    if (currentIndex < 0) return;
    const next = buttons[(currentIndex + direction + buttons.length) % buttons.length];
    const mode = next.dataset.scaleMode as WallpaperSettings['scaleMode'] | undefined;
    if (!mode) return;
    void changeOverride('scaleMode', mode);
    next.focus();
}

async function toggleCategory(categoryId: string) {
    if (!item.value || categoryBusy.value) return;
    const assigned = !item.value.categoryIds.includes(categoryId);
    categoryBusy.value = true;
    errorMessage.value = '';
    try {
        await setCategoryMembership([item.value.id], categoryId, assigned);
        categoryEditorOpen.value = false;
        nextTick(() => categoryTrigger.value?.focus());
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        categoryBusy.value = false;
    }
}

async function toggleCurrentTargets() {
    if (!item.value) return;
    await action(() => toggleMediaPause(item.value!.id));
}

async function stopCurrentTargets() {
    if (!item.value) return;
    await action(() => stopMedia(item.value!.id));
}

async function confirmRemoval() {
    if (!item.value || removing.value) return;
    const mediaId = item.value.id;
    const mediaName = item.value.name;
    errorMessage.value = '';
    removing.value = true;
    nextTick(() => removalDialog.value?.focus());
    try {
        await removeMedia(mediaId);
        removeDialogOpen.value = false;
        wallStore.showNotice(`已从壁纸库移除 ${mediaName}`);
        wallStore.activeCategoryId = null;
        await router.push('/');
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        removing.value = false;
        if (removeDialogOpen.value) nextTick(() => removeCancel.value?.focus());
    }
}

function openRemovalDialog() {
    if (actionBusy.value || settingsBusy.value || categoryBusy.value) return;
    errorMessage.value = '';
    removeDialogOpen.value = true;
    nextTick(() => removeCancel.value?.focus());
}

function closeRemovalDialog() {
    if (removing.value) return;
    removeDialogOpen.value = false;
    errorMessage.value = '';
    nextTick(() => removeTrigger.value?.focus());
}

function toggleCategoryEditor() {
    if (categoryBusy.value) return;
    categoryEditorOpen.value = !categoryEditorOpen.value;
}

function openCategoryEditorFromKeyboard(last: boolean) {
    if (categoryBusy.value) return;
    categoryEditorOpen.value = true;
    nextTick(() => focusMenuEdge(last));
}

function closeCategoryEditor(returnFocus = false) {
    if (categoryBusy.value) return;
    categoryEditorOpen.value = false;
    if (returnFocus) nextTick(() => categoryTrigger.value?.focus());
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
        closeCategoryEditor(true);
        return;
    }
    if (nextIndex === null) return;
    event.preventDefault();
    items[nextIndex].focus();
}

function enabledMenuItems(container: HTMLElement): HTMLButtonElement[] {
    return Array.from(container.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
}

function focusMenuEdge(last: boolean) {
    if (!categoryMenu.value) return;
    const items = enabledMenuItems(categoryMenu.value);
    items[last ? items.length - 1 : 0]?.focus();
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
    if (removeDialogOpen.value) closeRemovalDialog();
    else if (categoryEditorOpen.value) closeCategoryEditor(true);
}

function handleDocumentPointerDown(event: PointerEvent) {
    if (!categoryEditorOpen.value || categoryBusy.value) return;
    const target = event.target;
    if (!(target instanceof Node)) return;
    if (categoryTrigger.value?.contains(target) || categoryMenu.value?.contains(target)) return;
    closeCategoryEditor(true);
}

function playPreview() {
    errorMessage.value = '';
    previewVideo.value?.play().catch((error) => {
        errorMessage.value = readError(error);
    });
}
</script>

<template>
    <section v-if="item" class="page detail-page">
        <div class="page-modal-background" :inert="removeDialogOpen ? true : undefined">
            <div class="page-heading">
                <button
                    class="back-button"
                    title="返回壁纸库"
                    aria-label="返回壁纸库"
                    :disabled="actionBusy || settingsBusy || categoryBusy || removing"
                    @click="router.push('/')"
                >
                    <WallIcon name="back" :size="20" />
                </button>
                <h1>{{ item.name }}</h1>
                <div class="detail-heading-actions">
                    <button
                        ref="removeTrigger"
                        class="danger-quiet button-medium"
                        data-detail-action="remove"
                        :disabled="actionBusy || settingsBusy || categoryBusy"
                        @click="openRemovalDialog"
                    >
                        <WallIcon name="trash" :size="16" />从库中移除
                    </button>
                    <button
                        v-if="!item.missing"
                        class="primary button-medium"
                        data-detail-action="play"
                        :disabled="actionBusy"
                        @click="action(() => play(item!.id))"
                    >
                        {{ actionBusy ? '正在设置…' : '设为壁纸' }}
                    </button>
                </div>
            </div>
            <div class="detail-grid">
                <div class="media-preview">
                    <img
                        v-if="item.kind === 'image' && mediaUrl(item.path)"
                        :src="mediaUrl(item.path)"
                        :alt="item.name"
                    />
                    <video
                        v-else-if="item.kind === 'video' && mediaUrl(item.path)"
                        ref="previewVideo"
                        :src="mediaUrl(item.path)"
                        :aria-label="`${item.name} 视频预览`"
                        controls
                        muted
                        loop
                        playsinline
                        preload="metadata"
                        @play="previewPlaying = true"
                        @pause="previewPlaying = false"
                        @ended="previewPlaying = false"
                        @loadedmetadata="previewPlaying = false"
                    />
                    <div v-else class="preview-fallback">{{ item.kind === 'video' ? 'VIDEO' : 'IMAGE' }}</div>
                    <button
                        v-if="item.kind === 'video' && mediaUrl(item.path) && !previewPlaying"
                        class="preview-play-button"
                        data-preview-action="play"
                        aria-label="播放视频预览"
                        @click="playPreview"
                    >
                        <WallIcon name="play" :size="26" />
                    </button>
                </div>
                <aside class="detail-card">
                    <h2>{{ item.name }}</h2>
                    <span class="format-label">{{ item.kind.toUpperCase() }} · {{ item.format }}</span>
                    <div class="wallpaper-categories">
                        <span v-for="category in itemCategories" :key="category.id" class="category-tag">
                            {{ category.name }}
                        </span>
                        <button
                            ref="categoryTrigger"
                            class="category-edit-button"
                            aria-label="编辑分类"
                            aria-haspopup="menu"
                            :aria-expanded="categoryEditorOpen"
                            :disabled="categoryBusy"
                            @click="toggleCategoryEditor"
                            @keydown.down.prevent="openCategoryEditorFromKeyboard(false)"
                            @keydown.up.prevent="openCategoryEditorFromKeyboard(true)"
                        >
                            <WallIcon name="settings" :size="14" />编辑分类
                        </button>
                        <div
                            v-if="categoryEditorOpen"
                            ref="categoryMenu"
                            class="wallpaper-category-menu"
                            role="menu"
                            @keydown="handleCategoryMenuKeydown"
                        >
                            <button
                                v-for="category in wallStore.snapshot.categories"
                                :key="category.id"
                                :aria-label="
                                    item.categoryIds.includes(category.id)
                                        ? `从${category.name}移除`
                                        : `添加到${category.name}`
                                "
                                role="menuitem"
                                :disabled="categoryBusy"
                                @click="toggleCategory(category.id)"
                            >
                                <span>{{ category.name }}</span>
                                <WallIcon v-if="item.categoryIds.includes(category.id)" name="check" :size="14" />
                            </button>
                        </div>
                    </div>
                    <p>
                        {{ item.width ? `${item.width} × ${item.height}` : '尺寸将在播放后读取' }}<br />{{
                            duration(item.durationSeconds, item.kind)
                        }}<br /><span class="path-copy">{{ item.path }}</span>
                    </p>
                    <div v-if="targetLabels.length" class="detail-targets">
                        <span>当前目标</span>
                        <strong v-for="target in targetLabels" :key="target">{{ target }}</strong>
                    </div>
                    <div class="playback-state">
                        <i :class="{ error: item.missing }" />{{
                            item.missing
                                ? '文件丢失'
                                : active
                                  ? item.kind === 'image'
                                      ? '正在显示'
                                      : paused
                                        ? '已暂停'
                                        : muted
                                          ? '正在运行 · 已静音'
                                          : '正在运行'
                                  : '未运行'
                        }}
                    </div>
                    <div class="detail-actions">
                        <button v-if="item.missing" class="primary" :disabled="actionBusy" @click="relocate">
                            {{ actionBusy ? '正在定位…' : '定位新文件' }}
                        </button>
                        <button
                            v-if="item.kind === 'video'"
                            class="secondary"
                            :disabled="actionBusy || !active"
                            @click="toggleCurrentTargets"
                        >
                            {{ paused ? '继续' : '暂停' }}
                        </button>
                        <button class="danger" :disabled="actionBusy || !active" @click="stopCurrentTargets">
                            停止
                        </button>
                        <button
                            class="secondary"
                            :disabled="actionBusy"
                            @click="action(() => openMediaFolder(item!.id))"
                        >
                            打开文件位置
                        </button>
                    </div>
                </aside>
            </div>
            <div class="detail-settings" :aria-busy="settingsBusy">
                <div class="detail-settings-header">
                    <strong>播放覆盖设置</strong>
                    <span class="inheritance-pill" :class="{ overridden: hasOverrides }">
                        {{ hasOverrides ? '已单独设置' : '使用全局设置' }}
                    </span>
                    <button
                        v-if="hasOverrides"
                        class="secondary"
                        :disabled="settingsBusy"
                        @click="restoreGlobalSettings"
                    >
                        恢复全局设置
                    </button>
                </div>
                <div class="detail-settings-primary">
                    <div class="detail-setting-field">
                        <label>缩放方式</label>
                        <div class="segmented">
                            <button
                                v-for="mode in scaleModes"
                                :key="mode.value"
                                :data-scale-mode="mode.value"
                                :disabled="settingsBusy"
                                :class="{ active: effectiveSettings.scaleMode === mode.value }"
                                :aria-pressed="effectiveSettings.scaleMode === mode.value"
                                @click="changeOverride('scaleMode', mode.value)"
                                @keydown.left.prevent="moveScaleMode($event, -1)"
                                @keydown.right.prevent="moveScaleMode($event, 1)"
                            >
                                {{ mode.label }}
                            </button>
                        </div>
                    </div>
                    <div class="detail-setting-field">
                        <label>画幅</label>
                        <select
                            data-setting="aspect-ratio"
                            :disabled="settingsBusy"
                            :value="effectiveSettings.aspectRatio"
                            @change="
                                changeOverride(
                                    'aspectRatio',
                                    ($event.target as HTMLSelectElement).value as AppSettings['aspectRatio'],
                                )
                            "
                        >
                            <option value="original">原始</option>
                            <option value="screen">屏幕</option>
                            <option value="ratio16x9">16:9</option>
                            <option value="ratio16x10">16:10</option>
                            <option value="ratio21x9">21:9</option>
                            <option value="ratio32x9">32:9</option>
                            <option value="ratio4x3">4:3</option>
                            <option value="ratio1x1">1:1</option>
                            <option value="ratio9x16">9:16</option>
                        </select>
                    </div>
                    <div class="detail-setting-field">
                        <label>抗锯齿</label>
                        <select
                            data-setting="anti-aliasing"
                            :disabled="settingsBusy"
                            :value="effectiveSettings.antiAliasing"
                            @change="
                                changeOverride(
                                    'antiAliasing',
                                    ($event.target as HTMLSelectElement).value as AppSettings['antiAliasing'],
                                )
                            "
                        >
                            <option value="off">关闭</option>
                            <option value="balanced">均衡</option>
                            <option value="high">高质量</option>
                        </select>
                    </div>
                    <div v-if="item.kind === 'video'" class="detail-setting-field">
                        <label>帧率</label>
                        <select
                            data-setting="frame-rate"
                            :disabled="settingsBusy"
                            :value="effectiveSettings.frameRate"
                            @change="
                                changeOverride(
                                    'frameRate',
                                    Number(($event.target as HTMLSelectElement).value) as AppSettings['frameRate'],
                                )
                            "
                        >
                            <option :value="0">源帧率</option>
                            <option :value="24">24 FPS</option>
                            <option :value="30">30 FPS</option>
                            <option :value="60">60 FPS</option>
                        </select>
                    </div>
                </div>
                <div v-if="item.kind === 'video'" class="detail-settings-video">
                    <div class="detail-setting-field inline-field">
                        <label>硬件解码</label>
                        <button
                            class="toggle"
                            role="switch"
                            :disabled="settingsBusy"
                            :aria-checked="effectiveSettings.hardwareDecoding"
                            :class="{ on: effectiveSettings.hardwareDecoding }"
                            @click="changeOverride('hardwareDecoding', !effectiveSettings.hardwareDecoding)"
                        >
                            <i />
                        </button>
                    </div>
                    <div class="detail-setting-field inline-field">
                        <label>静音</label>
                        <button
                            class="toggle mute-button"
                            role="switch"
                            :disabled="settingsBusy"
                            :aria-checked="effectiveSettings.muted"
                            :class="{ on: effectiveSettings.muted }"
                            @click="changeOverride('muted', !effectiveSettings.muted)"
                        >
                            <i />
                        </button>
                    </div>
                    <div class="detail-setting-field volume-field">
                        <label>音量 · {{ effectiveSettings.volume }}%</label>
                        <input
                            type="range"
                            min="0"
                            max="100"
                            :disabled="settingsBusy"
                            :value="effectiveSettings.volume"
                            :style="{ '--range-progress': `${effectiveSettings.volume}%` }"
                            @change="changeOverride('volume', Number(($event.target as HTMLInputElement).value))"
                        />
                    </div>
                </div>
                <p v-else class="image-settings-note">图片壁纸不显示帧率、硬件解码和声音设置</p>
            </div>
            <p v-if="errorMessage" class="inline-error" role="alert">
                {{ errorMessage }}
                <button v-if="errorMessage.includes('mpv')" @click="router.push('/settings/about')">查看说明</button>
            </p>
        </div>
        <div v-if="removeDialogOpen" class="modal-scrim">
            <div
                ref="removalDialog"
                class="dialog removal-dialog"
                data-removal-kind="single"
                role="alertdialog"
                aria-modal="true"
                aria-labelledby="single-removal-dialog-title"
                :aria-busy="removing"
                tabindex="-1"
                @keydown="trapDialogFocus"
            >
                <button class="dialog-close" aria-label="关闭移除确认" :disabled="removing" @click="closeRemovalDialog">
                    <WallIcon name="close" :size="18" />
                </button>
                <h2 id="single-removal-dialog-title">从壁纸库移除 {{ item.name }}？</h2>
                <p>只移除 Wall 的记录，不会删除或修改原文件。</p>
                <div v-if="active" class="removal-warning">
                    <WallIcon name="warning" :size="18" />
                    <span>{{ item.name }} 正在使用；继续后将停止相关屏幕。</span>
                </div>
                <p v-if="errorMessage" class="inline-error" role="alert">{{ errorMessage }}</p>
                <div class="dialog-actions">
                    <button
                        ref="removeCancel"
                        class="secondary"
                        data-removal-cancel
                        autofocus
                        :disabled="removing"
                        @click="closeRemovalDialog"
                    >
                        取消
                    </button>
                    <button class="danger" data-removal-confirm :disabled="removing" @click="confirmRemoval">
                        移除
                    </button>
                </div>
            </div>
        </div>
    </section>
    <section v-else class="page empty-state">
        <h1>找不到这张壁纸</h1>
        <p>它可能已经从媒体库移除。</p>
        <button class="primary" @click="router.push('/')">返回壁纸库</button>
    </section>
</template>
