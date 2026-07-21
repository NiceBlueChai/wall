/** 定义详情页与批量管理复用的应用级分类创建入口。 */
import type { InjectionKey } from 'vue';

/** 打开分类创建弹窗，并在创建成功后把分类添加到指定壁纸。 */
export type OpenCategoryCreator = (mediaIds: string[], trigger: HTMLElement) => void;

/** 供路由页面注入应用级分类创建入口。 */
export const openCategoryCreatorKey: InjectionKey<OpenCategoryCreator> = Symbol('openCategoryCreator');
