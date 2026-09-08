<script setup lang="ts">
import type { UserProfile } from "../types";

const props = defineProps<{
    user: UserProfile;
    connectionState: string;
    connectionDetail: string;
    watchedCount: number;
}>();

const emit = defineEmits<{
    logout: [];
    reconnect: [];
}>();

const liveLabel = () => {
    return props.user.liveStatus === 1 ? "直播中" : "未开播";
};

const roomLabel = () => {
    if (!props.user.roomId) {
        return "没有直播间";
    }
    if (props.user.shortId && props.user.shortId !== props.user.roomId) {
        return `${props.user.shortId} / ${props.user.roomId}`;
    }
    return String(props.user.roomId);
};

const connectionColor = () => {
    if (props.connectionState === "connected") {
        return "success";
    }
    if (props.connectionState === "connecting") {
        return "processing";
    }
    if (props.connectionState === "error") {
        return "error";
    }
    return "default";
};
</script>

<template>
    <div class="user-panel">
        <div class="user-panel__top">
            <a-avatar :src="user.face" :size="40">
                {{ user.uname.slice(0, 1) }}
            </a-avatar>
            <div class="user-panel__meta">
                <h2 class="user-panel__name">{{ user.uname }}</h2>
                <p class="user-panel__sub">
                    <a-tag :color="user.liveStatus === 1 ? 'red' : 'default'">{{ liveLabel() }}</a-tag>
                    <span>看过 {{ watchedCount }}</span>
                </p>
            </div>
        </div>
        <div class="user-panel__info">
            <p>
                <span>直播间</span>
                <strong>{{ roomLabel() }}</strong>
            </p>
            <p v-if="user.title">
                <span>标题</span>
                <strong>{{ user.title }}</strong>
            </p>
            <p>
                <span>弹幕</span>
                <a-tag :color="connectionColor()">{{ connectionDetail }}</a-tag>
            </p>
        </div>
        <a-space size="small" class="user-panel__actions">
            <a-button size="small" type="primary" :disabled="!user.roomId" @click="emit('reconnect')">
                重连
            </a-button>
            <a-button size="small" @click="emit('logout')">退出</a-button>
        </a-space>
    </div>
</template>

<style lang="less" scoped>
.user-panel {
    flex-shrink: 0;
    padding: 12px 12px 10px;
    border-bottom: 1px solid rgba(246, 231, 200, 0.12);

    &__top {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-bottom: 10px;
    }

    &__meta {
        min-width: 0;
        flex: 1;
    }

    &__name {
        margin: 0 0 4px;
        font-size: 15px;
        color: #f6e7c8;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    &__sub {
        display: flex;
        align-items: center;
        gap: 8px;
        margin: 0;
        color: #a89478;
        font-size: 12px;
    }

    &__info {
        p {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 8px;
            margin: 0 0 6px;
            color: #e8d5b0;
            font-size: 12px;

            span {
                flex-shrink: 0;
                color: #a89478;
            }

            strong {
                font-weight: 600;
                text-align: right;
                word-break: break-all;
            }
        }
    }

    &__actions {
        margin-top: 4px;
    }
}
</style>
