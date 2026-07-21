/** 集中定义详情页与设置页共用的播放选择项，避免相同文案和值发生漂移。 */

/** Wall Select 接受的标量值。 */
export type WallSelectValue = string | number;

/** Wall Select 的单个可选项。 */
export interface WallSelectOption {
    value: WallSelectValue;
    label: string;
}

/** 可用的画幅选项。 */
export const aspectRatioOptions = [
    { value: 'original', label: '原始' },
    { value: 'screen', label: '屏幕' },
    { value: 'ratio16x9', label: '16:9' },
    { value: 'ratio16x10', label: '16:10' },
    { value: 'ratio21x9', label: '21:9' },
    { value: 'ratio32x9', label: '32:9' },
    { value: 'ratio4x3', label: '4:3' },
    { value: 'ratio1x1', label: '1:1' },
    { value: 'ratio9x16', label: '9:16' },
] as const satisfies readonly WallSelectOption[];

/** 可用的抗锯齿选项。 */
export const antiAliasingOptions = [
    { value: 'off', label: '关闭' },
    { value: 'balanced', label: '均衡' },
    { value: 'high', label: '高质量' },
] as const satisfies readonly WallSelectOption[];

/** 可用的视频帧率选项。 */
export const frameRateOptions = [
    { value: 0, label: '源帧率' },
    { value: 24, label: '24 FPS' },
    { value: 30, label: '30 FPS' },
    { value: 60, label: '60 FPS' },
] as const satisfies readonly WallSelectOption[];

/** v1 固定提供的界面语言。 */
export const languageOptions = [{ value: 'zh-CN', label: '简体中文' }] as const satisfies readonly WallSelectOption[];
