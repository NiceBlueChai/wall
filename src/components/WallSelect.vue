<!-- 复用 Figma 03 Components 的 Select 与 Menu Item，并提供统一的键盘选择行为。 -->
<script setup lang="ts">
import { computed, nextTick, ref, useId } from 'vue';
import type { WallSelectOption, WallSelectValue } from '../playbackOptions';
import WallIcon from './WallIcon.vue';

defineOptions({ inheritAttrs: false });

const props = withDefaults(
    defineProps<{
        modelValue: WallSelectValue;
        options: readonly WallSelectOption[];
        disabled?: boolean;
        label: string;
    }>(),
    { disabled: false },
);
const emit = defineEmits<{
    'update:modelValue': [value: WallSelectValue];
    change: [value: WallSelectValue];
}>();
const root = ref<HTMLElement | null>(null);
const trigger = ref<HTMLButtonElement | null>(null);
const optionButtons = ref<HTMLButtonElement[]>([]);
const open = ref(false);
const listboxId = `wall-select-${useId().replace(/[^a-z0-9_-]/gi, '')}`;
const selectedOption = computed(
    () => props.options.find((option) => option.value === props.modelValue) ?? props.options[0],
);

function focusOption(index: number) {
    const count = optionButtons.value.length;
    if (!count) return;
    optionButtons.value[(index + count) % count]?.focus();
}

async function show(direction: -1 | 0 | 1 = 0) {
    if (props.disabled || !props.options.length) return;
    open.value = true;
    await nextTick();
    const selectedIndex = Math.max(
        0,
        props.options.findIndex((option) => option.value === props.modelValue),
    );
    focusOption(selectedIndex + direction);
}

function hide(returnFocus = false) {
    open.value = false;
    if (returnFocus) void nextTick(() => trigger.value?.focus());
}

function toggle() {
    if (open.value) hide();
    else void show();
}

function choose(value: WallSelectValue) {
    emit('update:modelValue', value);
    emit('change', value);
    hide(true);
}

function handleTriggerKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        void show(event.key === 'ArrowDown' ? 0 : -1);
    } else if (event.key === 'Escape' && open.value) {
        event.preventDefault();
        hide(true);
    }
}

function handleOptionKeydown(event: KeyboardEvent, index: number) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        focusOption(index + (event.key === 'ArrowDown' ? 1 : -1));
    } else if (event.key === 'Home' || event.key === 'End') {
        event.preventDefault();
        focusOption(event.key === 'Home' ? 0 : optionButtons.value.length - 1);
    } else if (event.key === 'Escape') {
        event.preventDefault();
        hide(true);
    }
}

function handleFocusout(event: FocusEvent) {
    const next = event.relatedTarget;
    if (!(next instanceof Node) || !root.value?.contains(next)) hide();
}
</script>

<template>
    <div ref="root" v-bind="$attrs" class="wall-select" :class="{ open, disabled }" @focusout="handleFocusout">
        <button
            ref="trigger"
            type="button"
            class="wall-select-trigger"
            role="combobox"
            aria-haspopup="listbox"
            :aria-label="label"
            :aria-controls="listboxId"
            :aria-expanded="open"
            :disabled="disabled"
            @click="toggle"
            @keydown="handleTriggerKeydown"
        >
            <span>{{ selectedOption?.label }}</span>
            <WallIcon name="chevron-down" :size="16" />
        </button>
        <div v-if="open" :id="listboxId" class="wall-select-menu" role="listbox" :aria-label="label">
            <button
                v-for="(option, index) in options"
                :key="option.value"
                ref="optionButtons"
                type="button"
                class="wall-select-option"
                role="option"
                :data-value="option.value"
                :aria-selected="option.value === modelValue"
                @click="choose(option.value)"
                @keydown="handleOptionKeydown($event, index)"
            >
                <span>{{ option.label }}</span>
                <WallIcon v-if="option.value === modelValue" name="check" :size="14" />
            </button>
        </div>
    </div>
</template>
