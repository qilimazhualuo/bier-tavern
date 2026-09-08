<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { Application, Container, Graphics, Text } from "pixi.js";
import type { BattleSnapshot, BattleStatus } from "../types";

const props = defineProps<{
    snapshot: BattleSnapshot | null;
}>();

const hostRef = ref<HTMLElement | null>(null);
const appRef = shallowRef<Application | null>(null);
const worldRef = shallowRef<Container | null>(null);
const ready = ref(false);

const statusText = computed(() => {
    const status = props.snapshot?.status ?? "idle";
    const mapping: Record<BattleStatus, string> = {
        idle: "等待开局",
        running: "激战中",
        redWin: "红方胜利",
        blueWin: "蓝方胜利",
    };
    return mapping[status];
});

const resizeStage = () => {
    const host = hostRef.value;
    const app = appRef.value;
    const world = worldRef.value;
    const snapshot = props.snapshot;
    if (!host || !app || !world || !snapshot) {
        return;
    }
    const width = host.clientWidth;
    const height = host.clientHeight;
    app.renderer.resize(width, height);
    const scale = Math.min(width / snapshot.mapWidth, height / snapshot.mapHeight);
    world.scale.set(scale);
    world.position.set(
        (width - snapshot.mapWidth * scale) / 2,
        (height - snapshot.mapHeight * scale) / 2,
    );
};

const drawSnapshot = (snapshot: BattleSnapshot) => {
    const world = worldRef.value;
    if (!world) {
        return;
    }
    world.removeChildren();

    const ground = new Graphics()
        .rect(0, 0, snapshot.mapWidth, snapshot.mapHeight)
        .fill({ color: 0x1a1410 });
    world.addChild(ground);

    const lane = new Graphics()
        .rect(80, snapshot.mapHeight / 2 - 48, snapshot.mapWidth - 160, 96)
        .fill({ color: 0x2a2118, alpha: 0.9 });
    world.addChild(lane);

    const drawBase = (base: BattleSnapshot["redBase"], color: number) => {
        const ring = new Graphics()
            .circle(base.x, base.y, 54)
            .fill({ color, alpha: 0.28 })
            .circle(base.x, base.y, 36)
            .fill({ color });
        world.addChild(ring);

        const hpRatio = Math.max(0, base.hp / base.maxHp);
        const bar = new Graphics()
            .rect(base.x - 40, base.y - 78, 80, 10)
            .fill({ color: 0x000000, alpha: 0.45 })
            .rect(base.x - 40, base.y - 78, 80 * hpRatio, 10)
            .fill({ color: 0xf2d28b });
        world.addChild(bar);

        const label = new Text({
            text: `${Math.ceil(base.hp)}`,
            style: {
                fill: 0xfff4dd,
                fontSize: 16,
                fontWeight: "700",
            },
        });
        label.anchor.set(0.5);
        label.position.set(base.x, base.y - 96);
        world.addChild(label);
    };

    drawBase(snapshot.redBase, 0xd64545);
    drawBase(snapshot.blueBase, 0x3b82f6);

    for (const unit of snapshot.units) {
        const color = unit.team === "red" ? 0xff6b6b : 0x60a5fa;
        const body = new Graphics().circle(unit.x, unit.y, 12).fill({ color });
        world.addChild(body);

        const hpRatio = Math.max(0, unit.hp / unit.maxHp);
        const bar = new Graphics()
            .rect(unit.x - 12, unit.y - 22, 24, 4)
            .fill({ color: 0x000000, alpha: 0.4 })
            .rect(unit.x - 12, unit.y - 22, 24 * hpRatio, 4)
            .fill({ color: 0xf6e7c8 });
        world.addChild(bar);

        if (unit.label) {
            const name = new Text({
                text: unit.label.slice(0, 4),
                style: {
                    fill: 0xfff8ea,
                    fontSize: 11,
                },
            });
            name.anchor.set(0.5);
            name.position.set(unit.x, unit.y + 20);
            world.addChild(name);
        }
    }

    resizeStage();
};

onMounted(async () => {
    if (!hostRef.value) {
        return;
    }
    const app = new Application();
    await app.init({
        backgroundAlpha: 0,
        antialias: true,
        resizeTo: hostRef.value,
        preference: "webgl",
    });
    hostRef.value.appendChild(app.canvas);
    const world = new Container();
    app.stage.addChild(world);
    appRef.value = app;
    worldRef.value = world;
    ready.value = true;
    if (props.snapshot) {
        drawSnapshot(props.snapshot);
    }
    window.addEventListener("resize", resizeStage);
});

onUnmounted(() => {
    window.removeEventListener("resize", resizeStage);
    const app = appRef.value;
    if (app) {
        app.destroy(true);
    }
    appRef.value = null;
    worldRef.value = null;
});

watch(
    () => props.snapshot,
    (snapshot) => {
        if (!ready.value || !snapshot) {
            return;
        }
        drawSnapshot(snapshot);
    },
    { deep: false },
);
</script>

<template>
    <div class="game-stage">
        <div ref="hostRef" class="game-stage__canvas"></div>
        <div class="game-stage__hud">
            <div class="game-stage__status">{{ statusText }}</div>
            <div v-if="snapshot" class="game-stage__scores">
                <span class="is-red">红 {{ snapshot.redSpawned }}</span>
                <span class="is-blue">蓝 {{ snapshot.blueSpawned }}</span>
            </div>
        </div>
    </div>
</template>

<style lang="less" scoped>
.game-stage {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;

    &__canvas {
        width: 100%;
        height: 100%;
    }

    &__hud {
        position: absolute;
        top: 16px;
        left: 16px;
        right: 16px;
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        pointer-events: none;
    }

    &__status {
        padding: 8px 12px;
        border-radius: 8px;
        background: rgba(18, 14, 12, 0.72);
        color: #f6e7c8;
        font-size: 14px;
        font-weight: 600;
    }

    &__scores {
        display: flex;
        gap: 10px;

        span {
            padding: 8px 12px;
            border-radius: 8px;
            background: rgba(18, 14, 12, 0.72);
            font-size: 13px;
            font-weight: 600;
        }

        .is-red {
            color: #ff8b7a;
        }

        .is-blue {
            color: #7eb6ff;
        }
    }
}
</style>
