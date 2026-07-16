<!-- 按职责呈现常规、播放、性能和关于四个设置页。 -->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { RouterLink, useRoute } from 'vue-router';
import { openLicense, openProjectHomepage, updateSettings } from '../api';
import { wallStore } from '../store';
import WallIcon from '../components/WallIcon.vue';
import type { AppSettings } from '../types';

const route = useRoute();
const errorMessage = ref('');
const section = computed(() => String(route.params.section || 'general'));
const settings = computed(() => wallStore.snapshot.settings);
const tabs = [
    ['general', '常规'],
    ['playback', '播放'],
    ['performance', '性能'],
    ['about', '关于'],
] as const;

async function change<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    errorMessage.value = '';
    try {
        await updateSettings({ ...settings.value, [key]: value });
    } catch (error) {
        errorMessage.value = readError(error);
    }
}

async function run(action: () => Promise<unknown>) {
    errorMessage.value = '';
    try {
        await action();
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
    <section class="page settings-page">
        <h1>设置</h1>
        <nav class="tabs settings-tabs" aria-label="设置分类">
            <RouterLink
                v-for="tab in tabs"
                :key="tab[0]"
                :to="`/settings/${tab[0]}`"
                :class="{ active: section === tab[0] }"
            >
                {{ tab[1] }}
            </RouterLink>
        </nav>

        <div v-if="section === 'general'" class="settings-panel">
            <h2>启动与窗口</h2>
            <div class="setting-row">
                <div><strong>开机启动</strong><small>登录 Windows 后自动启动 Wall</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :aria-checked="settings.autoStart"
                    :class="{ on: settings.autoStart }"
                    @click="change('autoStart', !settings.autoStart)"
                >
                    <i />
                </button>
            </div>
            <div class="setting-row">
                <div><strong>关闭时最小化到托盘</strong><small>关闭主窗口不会停止当前壁纸</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :aria-checked="settings.closeToTray"
                    :class="{ on: settings.closeToTray }"
                    @click="change('closeToTray', !settings.closeToTray)"
                >
                    <i />
                </button>
            </div>
            <div class="setting-row">
                <div><strong>恢复上次壁纸</strong><small>应用启动后恢复上一次运行的壁纸</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :aria-checked="settings.restoreLastWallpaper"
                    :class="{ on: settings.restoreLastWallpaper }"
                    @click="change('restoreLastWallpaper', !settings.restoreLastWallpaper)"
                >
                    <i />
                </button>
            </div>
            <div class="setting-row">
                <div><strong>界面语言</strong><small>v1 仅提供简体中文</small></div>
                <select disabled>
                    <option>简体中文</option>
                </select>
            </div>
        </div>

        <div v-else-if="section === 'playback'" class="settings-panel playback-panel">
            <h2>播放</h2>
            <div class="setting-row">
                <div><strong>缩放方式</strong><small>默认等比例填满并居中裁切</small></div>
                <div class="segmented compact">
                    <button
                        v-for="mode in ['cover', 'contain', 'stretch'] as const"
                        :key="mode"
                        :class="{ active: settings.scaleMode === mode }"
                        @click="change('scaleMode', mode)"
                    >
                        {{ mode[0].toUpperCase() + mode.slice(1) }}
                    </button>
                </div>
            </div>
            <div class="setting-row">
                <div><strong>帧率上限</strong><small>降低数值可以减少 GPU 占用</small></div>
                <select
                    :value="settings.frameRate"
                    @change="change('frameRate', Number(($event.target as HTMLSelectElement).value) as 30 | 60)"
                >
                    <option :value="30">30 FPS</option>
                    <option :value="60">60 FPS</option>
                </select>
            </div>
            <div class="setting-row">
                <div><strong>硬件解码</strong><small>优先使用 GPU 解码视频</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :aria-checked="settings.hardwareDecoding"
                    :class="{ on: settings.hardwareDecoding }"
                    @click="change('hardwareDecoding', !settings.hardwareDecoding)"
                >
                    <i />
                </button>
            </div>
            <div class="setting-row">
                <div><strong>默认静音</strong><small>新导入的视频默认不播放声音</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :aria-checked="settings.defaultMuted"
                    :class="{ on: settings.defaultMuted }"
                    @click="change('defaultMuted', !settings.defaultMuted)"
                >
                    <i />
                </button>
            </div>
            <div class="setting-row">
                <div>
                    <strong>壁纸音量</strong><small>当前音量 {{ settings.volume }}%</small>
                </div>
                <input
                    type="range"
                    min="0"
                    max="100"
                    :value="settings.volume"
                    :style="{ '--range-progress': `${settings.volume}%` }"
                    @change="change('volume', Number(($event.target as HTMLInputElement).value))"
                />
            </div>
        </div>

        <div v-else-if="section === 'performance'" class="settings-panel performance-panel">
            <h2>自动暂停</h2>
            <p class="panel-description">在以下状态下自动暂停壁纸</p>
            <div class="setting-row">
                <div><strong>全屏应用运行时</strong><small>游戏或全屏视频运行时暂停壁纸</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :aria-checked="settings.pauseOnFullscreen"
                    :class="{ on: settings.pauseOnFullscreen }"
                    @click="change('pauseOnFullscreen', !settings.pauseOnFullscreen)"
                >
                    <i />
                </button>
            </div>
            <div class="setting-row">
                <div><strong>使用电池时</strong><small>未连接电源时减少耗电</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :aria-checked="settings.pauseOnBattery"
                    :class="{ on: settings.pauseOnBattery }"
                    @click="change('pauseOnBattery', !settings.pauseOnBattery)"
                >
                    <i />
                </button>
            </div>
            <div class="setting-row">
                <div><strong>显示器休眠时</strong><small>屏幕关闭后立即暂停播放</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :aria-checked="settings.pauseOnDisplaySleep"
                    :class="{ on: settings.pauseOnDisplaySleep }"
                    @click="change('pauseOnDisplaySleep', !settings.pauseOnDisplaySleep)"
                >
                    <i />
                </button>
            </div>
        </div>

        <div v-else class="settings-panel about-panel">
            <h2>关于 Wall</h2>
            <div class="about-brand">
                <WallIcon name="app" :size="48" />
                <div>
                    <h3>Wall</h3>
                    <b>v1.0.0</b><span>Windows 10/11 x64</span><small>免费开源 · 完全离线</small>
                </div>
            </div>
            <p>本地视频与图片动态壁纸工具</p>
            <div class="button-row">
                <button class="secondary button-medium" @click="run(openLicense)">查看开源许可证</button
                ><button class="secondary button-medium" disabled @click="run(openProjectHomepage)">
                    打开项目主页
                </button>
            </div>
            <small class="offline-notice">项目主页尚未配置；Wall 自身不请求网络。</small>
        </div>
        <p v-if="errorMessage" class="inline-error">{{ errorMessage }}</p>
    </section>
</template>
