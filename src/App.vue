<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { theme } from "antdv-next";
import zhCN from "antdv-next/locale/zh_CN";
import LoginPanel from "./components/LoginPanel.vue";
import UserPanel from "./components/UserPanel.vue";
import DanmakuList from "./components/DanmakuList.vue";
import GameStage from "./components/GameStage.vue";
import {
    getBattleState,
    getSession,
    logoutSession,
    reconnectDanmaku,
    spawnTestWave,
    startBattle,
    stopBattle,
} from "./api";
import type {
    BattleSnapshot,
    DanmakuMessage,
    DanmakuStatus,
    RoomStats,
    UserProfile,
} from "./types";

const MAX_MESSAGE_COUNT = 400;

const currentUser = ref<UserProfile | null>(null);
const messages = ref<DanmakuMessage[]>([]);
const connectionState = ref("disconnected");
const connectionDetail = ref("未连接");
const watchedCount = ref(0);
const bootstrapping = ref(true);
const drawerOpen = ref(false);
const battleSnapshot = ref<BattleSnapshot | null>(null);
const battleBusy = ref(false);

const antdTheme = {
    algorithm: theme.darkAlgorithm,
    token: {
        colorPrimary: "#d4822a",
        colorBgContainer: "#2b211b",
        colorBgElevated: "#352820",
        borderRadius: 8,
        fontFamily: "inherit",
    },
};

const listeners: UnlistenFn[] = [];

const drawerTitle = computed(() => {
    if (bootstrapping.value) {
        return "加载中";
    }
    if (!currentUser.value) {
        return "扫码登录";
    }
    return "直播间 / 弹幕";
});

const battleRunning = computed(() => battleSnapshot.value?.status === "running");

const openDrawer = () => {
    drawerOpen.value = true;
};

const handleLoginSuccess = (user: UserProfile) => {
    currentUser.value = user;
    messages.value = [];
    watchedCount.value = 0;
    connectionState.value = "connecting";
    connectionDetail.value = "正在连接弹幕服务器";
};

const handleLogout = async () => {
    await logoutSession();
    currentUser.value = null;
    messages.value = [];
    watchedCount.value = 0;
    connectionState.value = "disconnected";
    connectionDetail.value = "未连接";
    drawerOpen.value = true;
};

const handleReconnect = async () => {
    connectionState.value = "connecting";
    connectionDetail.value = "正在重新连接";
    await reconnectDanmaku();
};

const handleStartBattle = async () => {
    battleBusy.value = true;
    try {
        battleSnapshot.value = await startBattle();
    } finally {
        battleBusy.value = false;
    }
};

const handleStopBattle = async () => {
    battleBusy.value = true;
    try {
        battleSnapshot.value = await stopBattle();
    } finally {
        battleBusy.value = false;
    }
};

const handleSpawnTestWave = async () => {
    battleSnapshot.value = await spawnTestWave();
};

watch(bootstrapping, (loading) => {
    if (!loading && !currentUser.value) {
        drawerOpen.value = true;
    }
});

onMounted(async () => {
    listeners.push(
        await listen<DanmakuMessage>("danmaku-message", (event) => {
            messages.value.push(event.payload);
            if (messages.value.length > MAX_MESSAGE_COUNT) {
                messages.value.splice(0, messages.value.length - MAX_MESSAGE_COUNT);
            }
        }),
    );
    listeners.push(
        await listen<DanmakuStatus>("danmaku-status", (event) => {
            connectionState.value = event.payload.state;
            connectionDetail.value = event.payload.detail;
        }),
    );
    listeners.push(
        await listen<RoomStats>("room-stats", (event) => {
            watchedCount.value = event.payload.watched;
        }),
    );
    listeners.push(
        await listen<BattleSnapshot>("game-state", (event) => {
            battleSnapshot.value = event.payload;
        }),
    );

    try {
        const session = await getSession();
        if (session) {
            currentUser.value = session;
            connectionState.value = "connecting";
            connectionDetail.value = "正在连接弹幕服务器";
        }
        battleSnapshot.value = await getBattleState();
    } finally {
        bootstrapping.value = false;
    }
});

onUnmounted(() => {
    listeners.forEach((unlisten) => {
        unlisten();
    });
});
</script>

<template>
    <a-config-provider :theme="antdTheme" :locale="zhCN">
        <div class="app-shell">
            <main class="app-shell__game">
                <GameStage :snapshot="battleSnapshot" />
                <div class="app-shell__controls">
                    <a-space>
                        <a-button
                            type="primary"
                            :loading="battleBusy"
                            :disabled="battleRunning"
                            @click="handleStartBattle"
                        >
                            开始对局
                        </a-button>
                        <a-button
                            :disabled="!battleRunning"
                            :loading="battleBusy"
                            @click="handleStopBattle"
                        >
                            结束对局
                        </a-button>
                        <a-button :disabled="!battleRunning" @click="handleSpawnTestWave">
                            测试刷兵
                        </a-button>
                    </a-space>
                    <p class="app-shell__tip">
                        弹幕进场：带「红/左」归红方，「蓝/右」归蓝方，否则按 UID 奇偶分边
                    </p>
                </div>
            </main>

            <a-float-button
                class="app-shell__trigger"
                type="primary"
                :badge="{ count: messages.length, overflowCount: 99 }"
                @click="openDrawer"
            >
                <template #tooltip>直播间 / 弹幕</template>
                <template #icon>
                    <span class="app-shell__trigger-icon">弹</span>
                </template>
            </a-float-button>

            <a-drawer
                v-model:open="drawerOpen"
                :title="drawerTitle"
                placement="right"
                :width="360"
                :destroy-on-hidden="false"
                root-class="live-drawer"
            >
                <div class="live-drawer__body">
                    <div v-if="bootstrapping" class="live-drawer__loading">正在恢复登录态...</div>
                    <LoginPanel v-else-if="!currentUser" @success="handleLoginSuccess" />
                    <template v-else>
                        <UserPanel
                            :user="currentUser"
                            :connection-state="connectionState"
                            :connection-detail="connectionDetail"
                            :watched-count="watchedCount"
                            @logout="handleLogout"
                            @reconnect="handleReconnect"
                        />
                        <DanmakuList :messages="messages" />
                    </template>
                </div>
            </a-drawer>
        </div>
    </a-config-provider>
</template>

<style lang="less" scoped>
.app-shell {
    position: relative;
    height: 100%;
    background:
        radial-gradient(circle at top left, rgba(212, 130, 42, 0.16), transparent 36%),
        linear-gradient(180deg, #1c1511 0%, #120e0c 100%);

    &__game {
        position: relative;
        height: 100%;
    }

    &__controls {
        position: absolute;
        left: 16px;
        bottom: 16px;
        z-index: 2;
        padding: 12px 14px;
        border-radius: 12px;
        background: rgba(18, 14, 12, 0.78);
        border: 1px solid rgba(246, 231, 200, 0.12);
    }

    &__tip {
        margin: 10px 0 0;
        color: #a89478;
        font-size: 12px;
        max-width: 420px;
        line-height: 1.5;
    }

    &__trigger-icon {
        font-size: 14px;
        font-weight: 700;
        color: #fff;
    }
}
</style>

<style lang="less">
.live-drawer {
    .ant-drawer-body {
        padding: 0;
        height: 100%;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    &__body {
        display: flex;
        flex-direction: column;
        height: 100%;
        min-height: 0;
        overflow: hidden;
    }

    &__loading {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        color: #cbb89a;
        padding: 24px;
        text-align: center;
    }
}
</style>
