/** 声明 Vite 和 Vue 单文件组件类型。 */
/// <reference types="vite/client" />

declare module '*.vue' {
    import type { DefineComponent } from 'vue';

    const component: DefineComponent<Record<string, never>, Record<string, never>, unknown>;
    export default component;
}
