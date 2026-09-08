<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { generateQrcode, pollQrcode } from "../api";
import type { QrStatus, UserProfile } from "../types";

const emit = defineEmits<{
    success: [user: UserProfile];
}>();

const qrUrl = ref("https://www.bilibili.com");
const qrKey = ref("");
const qrStatus = ref<QrStatus>("loading");
const hintText = ref("正在生成二维码...");
const errorText = ref("");

let pollTimer: ReturnType<typeof setInterval> | null = null;

const stopPoll = () => {
    if (pollTimer !== null) {
        clearInterval(pollTimer);
        pollTimer = null;
    }
};

const refreshQrcode = async () => {
    stopPoll();
    qrStatus.value = "loading";
    hintText.value = "正在生成二维码...";
    errorText.value = "";
    try {
        const payload = await generateQrcode();
        qrUrl.value = payload.url;
        qrKey.value = payload.qrcodeKey;
        qrStatus.value = "active";
        hintText.value = "打开哔哩哔哩 App 扫码登录";
        pollTimer = setInterval(async () => {
            if (!qrKey.value) {
                return;
            }
            try {
                const result = await pollQrcode(qrKey.value);
                if (result.status === "scanned") {
                    qrStatus.value = "scanned";
                    hintText.value = result.message;
                    return;
                }
                if (result.status === "expired") {
                    qrStatus.value = "expired";
                    hintText.value = result.message;
                    stopPoll();
                    return;
                }
                if (result.status === "success" && result.user) {
                    stopPoll();
                    hintText.value = "登录成功";
                    emit("success", result.user);
                    return;
                }
                if (result.status === "error") {
                    errorText.value = result.message;
                }
            } catch (error) {
                errorText.value = error instanceof Error ? error.message : String(error);
            }
        }, 1500);
    } catch (error) {
        qrStatus.value = "expired";
        errorText.value = error instanceof Error ? error.message : String(error);
        hintText.value = "二维码生成失败，点刷新重来";
    }
};

onMounted(() => {
    void refreshQrcode();
});

onUnmounted(() => {
    stopPoll();
});
</script>

<template>
    <div class="login-panel">
        <h1 class="login-panel__title">比尔泰维勒之争</h1>
        <p class="login-panel__subtitle">扫码登录后，本小姐会去连你的直播间弹幕</p>
        <div class="login-panel__qr">
            <a-qrcode
                :value="qrUrl"
                :size="148"
                :status="qrStatus"
                color="#000000"
                bg-color="#ffffff"
                error-level="M"
                @refresh="refreshQrcode"
            />
        </div>
        <p class="login-panel__hint">{{ hintText }}</p>
        <p v-if="errorText" class="login-panel__error">{{ errorText }}</p>
        <a-button type="primary" ghost @click="refreshQrcode">刷新二维码</a-button>
    </div>
</template>

<style lang="less" scoped>
.login-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 16px 12px;
    text-align: center;
    overflow: auto;

    &__title {
        margin: 0 0 6px;
        font-size: 18px;
        font-weight: 700;
        letter-spacing: 1px;
        color: #f6e7c8;
    }

    &__subtitle {
        margin: 0 0 16px;
        color: #cbb89a;
        font-size: 12px;
        line-height: 1.5;
    }

    &__qr {
        padding: 8px;
        background: #fff8ec;
        border-radius: 10px;
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.28);
    }

    &__hint {
        margin: 14px 0 8px;
        color: #e8d5b0;
        font-size: 12px;
    }

    &__error {
        margin: 0 0 10px;
        color: #ff8b7a;
        font-size: 11px;
        max-width: 240px;
    }
}
</style>
