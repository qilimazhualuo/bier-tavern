<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import type { DanmakuMessage } from "../types";

const props = defineProps<{
    messages: DanmakuMessage[];
}>();

const feedRef = ref<HTMLElement | null>(null);
const stickToBottom = ref(true);

const onFeedScroll = () => {
    const element = feedRef.value;
    if (!element) {
        return;
    }
    const distance = element.scrollHeight - element.scrollTop - element.clientHeight;
    stickToBottom.value = distance < 56;
};

const cmdMeta = (command: string) => {
    const mapping: Record<string, { label: string; color: string }> = {
        DANMU_MSG: { label: "弹幕", color: "gold" },
        SEND_GIFT: { label: "礼物", color: "orange" },
        COMBO_SEND: { label: "连击", color: "orange" },
        SUPER_CHAT_MESSAGE: { label: "SC", color: "red" },
        SUPER_CHAT_MESSAGE_JPN: { label: "SC", color: "red" },
        INTERACT_WORD: { label: "互动", color: "blue" },
        GUARD_BUY: { label: "舰长", color: "purple" },
        ENTRY_EFFECT: { label: "进场", color: "cyan" },
        LIVE: { label: "开播", color: "red" },
        WARNING: { label: "警告", color: "volcano" },
        CUT_OFF: { label: "切断", color: "magenta" },
        ROOM_BLOCK_MSG: { label: "禁言", color: "default" },
    };
    return mapping[command] ?? { label: command, color: "default" };
};

watch(
    () => props.messages.length,
    async () => {
        await nextTick();
        const element = feedRef.value;
        if (stickToBottom.value && element) {
            element.scrollTop = element.scrollHeight;
        }
    },
);
</script>

<template>
    <div class="danmaku-list">
        <header class="danmaku-list__header">
            <h2>弹幕列表</h2>
            <span>{{ messages.length }} 条</span>
        </header>
        <div ref="feedRef" class="danmaku-list__feed" @scroll="onFeedScroll">
            <a-empty v-if="messages.length === 0" description="还没有弹幕，安静得像你的代码评审" />
            <div
                v-for="item in messages"
                :key="item.id"
                class="danmaku-list__item"
                :data-cmd="item.cmd"
            >
                <a-tag :color="cmdMeta(item.cmd).color">{{ cmdMeta(item.cmd).label }}</a-tag>
                <div class="danmaku-list__body">
                    <strong v-if="item.username">{{ item.username }}</strong>
                    <span>{{ item.content }}</span>
                </div>
            </div>
        </div>
    </div>
</template>

<style lang="less" scoped>
.danmaku-list {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;

    &__header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 10px 12px 8px;
        border-bottom: 1px solid rgba(246, 231, 200, 0.12);

        h2 {
            margin: 0;
            font-size: 14px;
            color: #f6e7c8;
        }

        span {
            color: #a89478;
            font-size: 11px;
        }
    }

    &__feed {
        flex: 1;
        overflow: auto;
        padding: 8px 10px 14px;
    }

    &__item {
        display: flex;
        align-items: flex-start;
        gap: 6px;
        padding: 6px 4px;
        border-radius: 6px;

        &:hover {
            background: rgba(255, 255, 255, 0.04);
        }
    }

    &__body {
        min-width: 0;
        color: #f3e9d7;
        line-height: 1.45;
        font-size: 12px;

        strong {
            margin-right: 6px;
            color: #f0c36a;
        }
    }
}
</style>
