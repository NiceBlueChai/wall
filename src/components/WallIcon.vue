<!-- 复用 Figma 04 Icons 导出的离线 SVG，并统一尺寸与可访问性。 -->
<script setup lang="ts">
export type WallIconName =
    | 'add'
    | 'app'
    | 'back'
    | 'check'
    | 'chevron-down'
    | 'chevron-right'
    | 'close'
    | 'contain'
    | 'cover'
    | 'external-link'
    | 'filter'
    | 'folder'
    | 'image'
    | 'import'
    | 'info'
    | 'library'
    | 'maximize'
    | 'minimize'
    | 'monitor'
    | 'more'
    | 'mute'
    | 'pause'
    | 'play'
    | 'power'
    | 'restore'
    | 'search'
    | 'settings'
    | 'stop'
    | 'stretch'
    | 'trash'
    | 'video'
    | 'volume'
    | 'warning';

withDefaults(defineProps<{ name: WallIconName; size?: number }>(), { size: 24 });
</script>

<template>
    <img class="wall-icon" :src="`/icons/${name}.svg`" alt="" :width="size" :height="size" />
</template>
