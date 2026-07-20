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
const settingsBusy = ref(false);
const externalBusy = ref(false);
const section = computed(() => String(route.params.section || 'general'));
const settings = computed(() => wallStore.snapshot.settings);
const tabs = [
    ['general', '常规'],
    ['playback', '播放'],
    ['performance', '性能'],
    ['about', '关于'],
] as const;
const scaleModes = [
    { value: 'cover', label: '填充' },
    { value: 'contain', label: '适应' },
    { value: 'stretch', label: '拉伸' },
] as const;

async function change<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    if (settingsBusy.value) return;
    errorMessage.value = '';
    settingsBusy.value = true;
    try {
        await updateSettings({ ...settings.value, [key]: value });
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        settingsBusy.value = false;
    }
}

async function run(action: () => Promise<unknown>) {
    if (externalBusy.value) return;
    errorMessage.value = '';
    externalBusy.value = true;
    try {
        await action();
    } catch (error) {
        errorMessage.value = readError(error);
    } finally {
        externalBusy.value = false;
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
    const mode = next.dataset.scaleMode as AppSettings['scaleMode'] | undefined;
    if (!mode) return;
    void change('scaleMode', mode);
    next.focus();
}

function readError(error: unknown): string {
    if (typeof error === 'object' && error && 'message' in error) return String(error.message);
    return String(error);
}
</script>

<template>
    <section class="page settings-page" :aria-busy="settingsBusy || externalBusy">
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
                    :disabled="settingsBusy"
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
                    :disabled="settingsBusy"
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
                    :disabled="settingsBusy"
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
            <h2>画面</h2>
            <div class="setting-row">
                <div><strong>缩放方式</strong><small>默认等比例填满并居中裁切</small></div>
                <div class="segmented compact">
                    <button
                        v-for="mode in scaleModes"
                        :key="mode.value"
                        :data-scale-mode="mode.value"
                        :disabled="settingsBusy"
                        :class="{ active: settings.scaleMode === mode.value }"
                        :aria-pressed="settings.scaleMode === mode.value"
                        @click="change('scaleMode', mode.value)"
                        @keydown.left.prevent="moveScaleMode($event, -1)"
                        @keydown.right.prevent="moveScaleMode($event, 1)"
                    >
                        {{ mode.label }}
                    </button>
                </div>
            </div>
            <div class="setting-row">
                <div><strong>画幅</strong><small>覆盖视频的逻辑宽高比</small></div>
                <select
                    data-setting="aspect-ratio"
                    :disabled="settingsBusy"
                    :value="settings.aspectRatio"
                    @change="
                        change('aspectRatio', ($event.target as HTMLSelectElement).value as AppSettings['aspectRatio'])
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
            <div class="setting-row">
                <div><strong>抗锯齿</strong><small>更高质量会增加 GPU 占用</small></div>
                <select
                    data-setting="anti-aliasing"
                    :disabled="settingsBusy"
                    :value="settings.antiAliasing"
                    @change="
                        change(
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
            <div class="setting-row">
                <div><strong>帧率上限</strong><small>降低数值可以减少 GPU 占用</small></div>
                <select
                    data-setting="frame-rate"
                    :disabled="settingsBusy"
                    :value="settings.frameRate"
                    @change="
                        change(
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
            <div class="setting-row">
                <div><strong>硬件解码</strong><small>优先使用 GPU 解码视频</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :disabled="settingsBusy"
                    :aria-checked="settings.hardwareDecoding"
                    :class="{ on: settings.hardwareDecoding }"
                    @click="change('hardwareDecoding', !settings.hardwareDecoding)"
                >
                    <i />
                </button>
            </div>
            <h2 class="settings-section-title">声音</h2>
            <div class="setting-row">
                <div><strong>默认静音</strong><small>新导入的视频默认不播放声音</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :disabled="settingsBusy"
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
                    :disabled="settingsBusy"
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
                <div><strong>最大化应用时</strong><small>前台应用最大化并遮蔽桌面时暂停壁纸</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :disabled="settingsBusy"
                    :aria-checked="settings.pauseOnMaximized"
                    :class="{ on: settings.pauseOnMaximized }"
                    @click="change('pauseOnMaximized', !settings.pauseOnMaximized)"
                >
                    <i />
                </button>
            </div>
            <div class="setting-row">
                <div><strong>全屏应用运行时</strong><small>游戏或全屏视频运行时暂停壁纸</small></div>
                <button
                    class="toggle"
                    role="switch"
                    :disabled="settingsBusy"
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
                    :disabled="settingsBusy"
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
                    :disabled="settingsBusy"
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
            <dl class="about-contact">
                <dt>作者：</dt>
                <dd>NiceBlueChai</dd>
                <dt>联系邮箱：</dt>
                <dd>bluechai@qq.com</dd>
            </dl>
            <div class="button-row">
                <button class="secondary button-medium" :disabled="externalBusy" @click="run(openLicense)">
                    查看开源许可证</button
                ><button class="secondary button-medium" :disabled="externalBusy" @click="run(openProjectHomepage)">
                    打开项目主页
                </button>
            </div>
            <small class="offline-notice">点击后使用系统默认浏览器打开；Wall 自身不请求网络。</small>
        </div>
        <p v-if="errorMessage" class="inline-error" role="alert">{{ errorMessage }}</p>
    </section>
</template>
