export type QrStatus = "active" | "expired" | "loading" | "scanned";

export interface QrGenerateResult {
    url: string;
    qrcodeKey: string;
}

export interface QrPollResult {
    status: "waiting" | "scanned" | "expired" | "success" | "error";
    message: string;
    user: UserProfile | null;
}

export interface UserProfile {
    uid: number;
    uname: string;
    face: string;
    roomId: number;
    shortId: number;
    title: string;
    liveStatus: number;
}

export interface DanmakuMessage {
    id: number;
    cmd: string;
    uid: number | null;
    username: string | null;
    content: string;
    timestamp: number;
}

export interface DanmakuStatus {
    state: "connecting" | "connected" | "disconnected" | "error" | string;
    detail: string;
}

export interface RoomStats {
    watched: number;
}

export type Team = "red" | "blue";

export type BattleStatus = "idle" | "running" | "redWin" | "blueWin";

export interface BaseView {
    team: Team;
    hp: number;
    maxHp: number;
    x: number;
    y: number;
}

export interface UnitView {
    id: number;
    team: Team;
    x: number;
    y: number;
    hp: number;
    maxHp: number;
    label: string;
}

export interface BattleSnapshot {
    status: BattleStatus;
    tick: number;
    mapWidth: number;
    mapHeight: number;
    redBase: BaseView;
    blueBase: BaseView;
    units: UnitView[];
    redSpawned: number;
    blueSpawned: number;
}
