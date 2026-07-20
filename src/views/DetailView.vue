<!-- 展示单个壁纸的本地预览、播放控制和文件恢复操作。 -->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { open } from '@tauri-apps/plugin-dialog';
import {
    mediaUrl,
    openMediaFolder,
    play,
    relocateMedia,
    setCategoryMembership,
    setWallpaperSettings,
    stop,
    stopTarget,
    togglePause,
    toggleTargetPause,
} from '../api';
import { wallStore } from '../store';
import WallIcon from '../components/WallIcon.vue';
import type { AppSettings, WallpaperSettings } from '../types';

const route = useRoute();
const router = useRouter();
const errorMessage = ref('');
const categoryEditorOpen = ref(false);
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

async function action(task: () => Promise<unknown>) {
    errorMessage.value = '';
    try {
        await task();
    } catch (error) {
        errorMessage.value = readError(error);
    }
}

async function relocate() {
    if (!item.value) return;
    const path = await open({ multiple: false, directory: false });
    if (typeof path === 'string') await action(() => relocateMedia(item.value!.id, path));
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

function changeOverride<K extends keyof WallpaperSettings>(key: K, value: WallpaperSettings[K]) {
    if (!item.value) return Promise.resolve();
    return action(() => setWallpaperSettings(item.value!.id, { ...overrides.value, [key]: value }));
}

function restoreGlobalSettings() {
    if (!item.value) return Promise.resolve();
    return action(() => setWallpaperSettings(item.value!.id, {}));
}

function toggleCategory(categoryId: string) {
    if (!item.value) return Promise.resolve();
    const assigned = !item.value.categoryIds.includes(categoryId);
    return action(() => setCategoryMembership([item.value!.id], categoryId, assigned));
}

async function toggleCurrentTargets() {
    if (activeAssignments.value.length === 0) {
        await action(togglePause);
        return;
    }
    const shouldPause = !paused.value;
    const targets = activeAssignments.value.filter((assignment) => (assignment.status === 'paused') !== shouldPause);
    await action(async () => {
        for (const assignment of targets) await toggleTargetPause(assignment.targetId);
    });
}

async function stopCurrentTargets() {
    if (activeAssignments.value.length === 0) {
        await action(stop);
        return;
    }
    const targets = [...activeAssignments.value];
    await action(async () => {
        for (const assignment of targets) await stopTarget(assignment.targetId);
    });
}
</script>

<template>
    <section v-if="item" class="page detail-page">
        <div class="page-heading">
            <button class="back-button" title="返回壁纸库" aria-label="返回壁纸库" @click="router.push('/')">
                <WallIcon name="back" :size="20" />
            </button>
            <h1>{{ item.name }}</h1>
            <button v-if="!item.missing" class="primary button-medium" @click="action(() => play(item!.id))">
                设为壁纸
            </button>
        </div>
        <div class="detail-grid">
            <div class="media-preview">
                <img v-if="item.kind === 'image' && mediaUrl(item.path)" :src="mediaUrl(item.path)" :alt="item.name" />
                <video v-else-if="item.kind === 'video' && mediaUrl(item.path)" :src="mediaUrl(item.path)" muted loop />
                <div v-else class="preview-fallback">{{ item.kind === 'video' ? 'VIDEO' : 'IMAGE' }}</div>
            </div>
            <aside class="detail-card">
                <h2>{{ item.name }}</h2>
                <span class="format-label">{{ item.kind.toUpperCase() }} · {{ item.format }}</span>
                <div class="wallpaper-categories">
                    <span v-for="category in itemCategories" :key="category.id" class="category-tag">
                        {{ category.name }}
                    </span>
                    <button
                        class="category-edit-button"
                        aria-label="编辑分类"
                        @click="categoryEditorOpen = !categoryEditorOpen"
                    >
                        <WallIcon name="settings" :size="14" />编辑分类
                    </button>
                    <div v-if="categoryEditorOpen" class="wallpaper-category-menu">
                        <button
                            v-for="category in wallStore.snapshot.categories"
                            :key="category.id"
                            :aria-label="
                                item.categoryIds.includes(category.id)
                                    ? `从${category.name}移除`
                                    : `添加到${category.name}`
                            "
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
                              ? paused
                                  ? '已暂停'
                                  : muted
                                    ? '正在运行 · 已静音'
                                    : '正在运行'
                              : '未运行'
                    }}
                </div>
                <div class="detail-actions">
                    <button v-if="item.missing" class="primary" @click="relocate">定位新文件</button>
                    <button class="secondary" :disabled="!active" @click="toggleCurrentTargets">
                        {{ paused ? '继续' : '暂停' }}
                    </button>
                    <button class="danger" :disabled="!active" @click="stopCurrentTargets">停止</button>
                    <button class="secondary" @click="action(() => openMediaFolder(item!.id))">打开文件位置</button>
                </div>
            </aside>
        </div>
        <div class="detail-settings">
            <div class="detail-settings-header">
                <strong>壁纸设置</strong>
                <span class="inheritance-pill" :class="{ overridden: hasOverrides }">
                    {{ hasOverrides ? '已单独设置' : '使用全局设置' }}
                </span>
                <button v-if="hasOverrides" class="secondary" @click="restoreGlobalSettings">恢复全局设置</button>
            </div>
            <div class="detail-setting-field">
                <label>缩放方式</label>
                <div class="segmented">
                    <button
                        v-for="mode in scaleModes"
                        :key="mode.value"
                        :class="{ active: effectiveSettings.scaleMode === mode.value }"
                        @click="changeOverride('scaleMode', mode.value)"
                    >
                        {{ mode.label }}
                    </button>
                </div>
            </div>
            <div class="detail-setting-field">
                <label>画幅</label>
                <select
                    data-setting="aspect-ratio"
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
            <div v-if="item.kind === 'video'" class="detail-setting-field inline-field">
                <label>硬件解码</label>
                <button
                    class="toggle"
                    role="switch"
                    :aria-checked="effectiveSettings.hardwareDecoding"
                    :class="{ on: effectiveSettings.hardwareDecoding }"
                    @click="changeOverride('hardwareDecoding', !effectiveSettings.hardwareDecoding)"
                >
                    <i />
                </button>
            </div>
            <div v-if="item.kind === 'video'" class="detail-setting-field inline-field">
                <label>静音</label>
                <button
                    class="toggle mute-button"
                    role="switch"
                    :aria-checked="effectiveSettings.muted"
                    :class="{ on: effectiveSettings.muted }"
                    @click="changeOverride('muted', !effectiveSettings.muted)"
                >
                    <i />
                </button>
            </div>
            <div v-if="item.kind === 'video'" class="detail-setting-field volume-field">
                <label>音量 · {{ effectiveSettings.volume }}%</label>
                <input
                    type="range"
                    min="0"
                    max="100"
                    :value="effectiveSettings.volume"
                    :style="{ '--range-progress': `${effectiveSettings.volume}%` }"
                    @change="changeOverride('volume', Number(($event.target as HTMLInputElement).value))"
                />
            </div>
        </div>
        <p v-if="errorMessage" class="inline-error">
            {{ errorMessage }}
            <button v-if="errorMessage.includes('mpv')" @click="router.push('/settings/about')">查看说明</button>
        </p>
    </section>
    <section v-else class="page empty-state">
        <h1>找不到这张壁纸</h1>
        <p>它可能已经从媒体库移除。</p>
        <button class="primary" @click="router.push('/')">返回壁纸库</button>
    </section>
</template>
