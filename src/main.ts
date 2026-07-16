/** 创建 Wall 的 Vue 应用并加载全局设计变量。 */
import { createApp } from 'vue';
import App from './App.vue';
import router from './router';
import './styles.css';

createApp(App).use(router).mount('#app');
