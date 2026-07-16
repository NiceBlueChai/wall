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
    removeMedia,
    setMuted,
    setScaleMode,
    setVolume,
    stop,
    togglePause,
} from '../api';
import { wallStore } from '../store';
import type { ScaleMode } from '../types';

const route = useRoute();
const router = useRouter();
const errorMessage = ref('');
const item = computed(() => wallStore.snapshot.library.find((entry) => entry.id === route.params.id) ?? null);
const active = computed(() => wallStore.snapshot.playback.activeId === item.value?.id);
const paused = computed(() => active.value && wallStore.snapshot.playback.status === 'paused');

async function action(task: () => Promise<unknown>) {
    errorMessage.value = '';
    try {
        await task();
    } catch (error) {
        errorMessage.value = readError(error);
    }
}

async function removeCurrent() {
    if (!item.value || !window.confirm(`从壁纸库移除“${item.value.name}”？原文件不会被删除。`)) return;
    await action(() => removeMedia(item.value!.id));
    if (!errorMessage.value) router.push('/');
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
</script>

<template>
    <section v-if="item" class="page detail-page">
        <div class="page-heading">
            <button class="back-button" @click="router.push('/')">‹</button>
            <h1>{{ item.name }}</h1>
            <button class="danger-quiet" @click="removeCurrent">从库中移除</button>
        </div>
        <div class="detail-grid">
            <div class="media-preview">
                <img v-if="item.kind === 'image' && mediaUrl(item.path)" :src="mediaUrl(item.path)" :alt="item.name" />
                <video
                    v-else-if="item.kind === 'video' && mediaUrl(item.path)"
                    :src="mediaUrl(item.path)"
                    muted
                    controls
                    loop
                />
                <div v-else class="preview-fallback">{{ item.kind === 'video' ? 'VIDEO' : 'IMAGE' }}</div>
            </div>
            <aside class="detail-card">
                <h2>{{ item.name }}</h2>
                <span class="format-label">{{ item.kind.toUpperCase() }} · {{ item.format }}</span>
                <p>
                    {{ item.width ? `${item.width} × ${item.height}` : '尺寸将在播放后读取' }}<br />{{
                        duration(item.durationSeconds, item.kind)
                    }}<br /><span class="path-copy">{{ item.path }}</span>
                </p>
                <div class="playback-state">
                    <i :class="{ error: item.missing }" />{{
                        item.missing
                            ? '文件丢失'
                            : active
                              ? paused
                                  ? '已暂停'
                                  : wallStore.snapshot.playback.muted
                                    ? '正在运行 · 已静音'
                                    : '正在运行'
                              : '未运行'
                    }}
                </div>
                <div class="detail-actions">
                    <button v-if="item.missing" class="primary" @click="relocate">定位新文件</button>
                    <button v-else class="primary" @click="action(() => play(item!.id))">设为壁纸</button>
                    <button class="secondary" :disabled="!active" @click="action(togglePause)">
                        {{ paused ? '继续' : '暂停' }}
                    </button>
                    <button class="secondary" :disabled="!active" @click="action(stop)">停止</button>
                    <button class="secondary" @click="action(() => openMediaFolder(item!.id))">打开文件夹</button>
                </div>
            </aside>
        </div>
        <div class="detail-settings">
            <div>
                <label>缩放方式</label>
                <div class="segmented">
                    <button
                        v-for="mode in ['cover', 'contain', 'stretch'] as ScaleMode[]"
                        :key="mode"
                        :class="{ active: wallStore.snapshot.settings.scaleMode === mode }"
                        @click="action(() => setScaleMode(mode))"
                    >
                        {{ mode[0].toUpperCase() + mode.slice(1) }}
                    </button>
                </div>
            </div>
            <div>
                <label>音量 · {{ wallStore.snapshot.playback.volume }}%</label
                ><input
                    type="range"
                    min="0"
                    max="100"
                    :value="wallStore.snapshot.playback.volume"
                    @change="action(() => setVolume(Number(($event.target as HTMLInputElement).value)))"
                />
            </div>
            <button
                class="secondary mute-button"
                :disabled="!active"
                @click="action(() => setMuted(!wallStore.snapshot.playback.muted))"
            >
                {{ wallStore.snapshot.playback.muted ? '取消静音' : '静音' }}
            </button>
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
