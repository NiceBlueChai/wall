<!-- 展示本地壁纸库、筛选、导入和卡片快捷播放。 -->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { open } from '@tauri-apps/plugin-dialog';
import { importMedia, mediaUrl, play } from '../api';
import { wallStore } from '../store';
import type { WallpaperItem } from '../types';

const router = useRouter();
const importing = ref(false);
const errorMessage = ref('');
const library = computed(() => wallStore.filteredLibrary);

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

function readError(error: unknown): string {
    if (typeof error === 'object' && error && 'message' in error) return String(error.message);
    return String(error);
}
</script>

<template>
    <section class="page library-page">
        <div class="page-heading">
            <h1>壁纸库</h1>
            <div class="heading-actions">
                <label class="search-field"
                    ><span>⌕</span><input v-model="wallStore.search" placeholder="搜索壁纸"
                /></label>
                <button class="primary" @click="chooseMedia">＋ 导入壁纸</button>
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

        <div v-if="!library.length && !wallStore.snapshot.library.length" class="empty-state">
            <div class="empty-icon">▧</div>
            <h2>导入第一张壁纸</h2>
            <p>支持本地视频和图片，文件始终保留在原位置。</p>
            <button class="primary" @click="chooseMedia">导入壁纸</button>
        </div>
        <div v-else-if="!library.length" class="empty-state compact-empty">
            <h2>没有匹配的壁纸</h2>
            <p>试试其他关键词或媒体类型。</p>
        </div>
        <div v-else class="wallpaper-grid">
            <article
                v-for="item in library"
                :key="item.id"
                class="wallpaper-card"
                :class="{ active: wallStore.snapshot.playback.activeId === item.id, missing: item.missing }"
                tabindex="0"
                @click="openDetails(item)"
                @keydown.enter="openDetails(item)"
                @dblclick.stop="quickPlay(item)"
            >
                <div class="card-preview" :class="`preview-${item.kind}`">
                    <img v-if="item.kind === 'image' && mediaUrl(item.path)" :src="mediaUrl(item.path)" alt="" />
                    <video
                        v-else-if="item.kind === 'video' && mediaUrl(item.path)"
                        :src="mediaUrl(item.path)"
                        muted
                        preload="metadata"
                    />
                    <span class="media-badge">{{ item.kind === 'video' ? 'VIDEO' : 'IMAGE' }}</span>
                    <span v-if="item.missing" class="missing-badge">文件丢失</span>
                </div>
                <div class="card-copy">
                    <strong>{{ item.name }}</strong
                    ><small
                        >{{ item.format
                        }}<template v-if="item.width"> · {{ item.width }} × {{ item.height }}</template></small
                    >
                </div>
                <i v-if="wallStore.snapshot.playback.activeId === item.id" class="running-dot" />
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
