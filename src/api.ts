import { invoke } from "@tauri-apps/api/core";
import type {
    BattleSnapshot,
    QrGenerateResult,
    QrPollResult,
    UserProfile,
} from "./types";

export const generateQrcode = () => {
    return invoke<QrGenerateResult>("generate_qrcode");
};

export const pollQrcode = (qrcodeKey: string) => {
    return invoke<QrPollResult>("poll_qrcode", { qrcodeKey });
};

export const getSession = () => {
    return invoke<UserProfile | null>("get_session");
};

export const logoutSession = () => {
    return invoke<void>("logout");
};

export const reconnectDanmaku = () => {
    return invoke<void>("reconnect_danmaku");
};

export const startBattle = () => {
    return invoke<BattleSnapshot>("start_battle");
};

export const stopBattle = () => {
    return invoke<BattleSnapshot>("stop_battle");
};

export const getBattleState = () => {
    return invoke<BattleSnapshot>("get_battle_state");
};

export const spawnTestWave = () => {
    return invoke<BattleSnapshot>("spawn_test_wave");
};
