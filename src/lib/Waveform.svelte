<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy } from "svelte";

  interface Props {
    active: boolean;
  }

  let { active }: Props = $props();

  const BAR_COUNT = 72;
  const SAMPLE_COUNT = 288; // ~4 source samples per bar for variation
  const PER_BAR = SAMPLE_COUNT / BAR_COUNT;

  let canvas: HTMLCanvasElement;
  let latest: Float32Array = new Float32Array(BAR_COUNT);
  let displayed: Float32Array = new Float32Array(BAR_COUNT);
  let rafId: number | null = null;
  let fetching = false;

  async function fetchLevels() {
    if (fetching) return;
    fetching = true;
    try {
      const raw: number[] = await invoke("get_audio_levels", {
        count: SAMPLE_COUNT,
      });
      // Aggregate SAMPLE_COUNT source samples into BAR_COUNT bars by peak
      const out = new Float32Array(BAR_COUNT);
      for (let i = 0; i < BAR_COUNT; i++) {
        const start = Math.floor(i * PER_BAR);
        const end = Math.floor((i + 1) * PER_BAR);
        let peak = 0;
        for (let j = start; j < end; j++) {
          const v = raw[j] ?? 0;
          if (v > peak) peak = v;
        }
        out[i] = peak;
      }
      latest = out;
    } catch {
      // ignore
    } finally {
      fetching = false;
    }
  }

  function tick() {
    // Smoothly approach target values (spring-like)
    for (let i = 0; i < BAR_COUNT; i++) {
      const target = latest[i];
      const current = displayed[i];
      // Rise fast, decay slower for natural feel
      const rate = target > current ? 0.55 : 0.18;
      displayed[i] = current + (target - current) * rate;
    }
    draw();
    if (active) {
      rafId = requestAnimationFrame(tick);
    }
  }

  function draw() {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;

    if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
      canvas.width = w * dpr;
      canvas.height = h * dpr;
    }

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    const gap = 2;
    const barWidth = Math.max(1.5, (w - gap * (BAR_COUNT - 1)) / BAR_COUNT);
    const centerY = h / 2;
    const maxBarHeight = h * 0.95;

    const styles = getComputedStyle(document.documentElement);
    const accent = styles.getPropertyValue("--accent").trim() || "#12a594";
    const accentSoft = styles.getPropertyValue("--teal-7").trim() || "#49bda5";
    const idle = styles.getPropertyValue("--gray-6").trim() || "#dbdbd7";

    for (let i = 0; i < BAR_COUNT; i++) {
      const value = displayed[i] ?? 0;
      // Perceptual scaling + minimum visible height
      const scaled = Math.min(1, Math.pow(value * 2.2, 0.65));
      const barHeight = Math.max(2, scaled * maxBarHeight);

      // Fade older bars (left side)
      const age = 1 - i / BAR_COUNT;
      const opacity = 0.35 + age * 0.65;

      const x = i * (barWidth + gap);
      const y = centerY - barHeight / 2;

      if (value < 0.005) {
        ctx.fillStyle = idle;
        ctx.globalAlpha = 0.35;
      } else {
        ctx.fillStyle = age > 0.55 ? accent : accentSoft;
        ctx.globalAlpha = opacity;
      }

      const r = Math.min(barWidth / 2, 1.5);
      roundRect(ctx, x, y, barWidth, barHeight, r);
      ctx.fill();
    }

    ctx.globalAlpha = 1;
  }

  function roundRect(
    ctx: CanvasRenderingContext2D,
    x: number,
    y: number,
    w: number,
    h: number,
    r: number
  ) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.arcTo(x + w, y, x + w, y + h, r);
    ctx.arcTo(x + w, y + h, x, y + h, r);
    ctx.arcTo(x, y + h, x, y, r);
    ctx.arcTo(x, y, x + w, y, r);
    ctx.closePath();
  }

  let pollInterval: ReturnType<typeof setInterval> | null = null;

  $effect(() => {
    if (active) {
      latest = new Float32Array(BAR_COUNT);
      displayed = new Float32Array(BAR_COUNT);
      pollInterval = setInterval(fetchLevels, 40);
      rafId = requestAnimationFrame(tick);
    } else {
      if (pollInterval) {
        clearInterval(pollInterval);
        pollInterval = null;
      }
      // Let bars decay naturally
      latest = new Float32Array(BAR_COUNT);
      const fadeEnd = setInterval(() => {
        let anyAlive = false;
        for (let i = 0; i < BAR_COUNT; i++) {
          displayed[i] *= 0.82;
          if (displayed[i] > 0.01) anyAlive = true;
        }
        draw();
        if (!anyAlive) {
          clearInterval(fadeEnd);
          displayed = new Float32Array(BAR_COUNT);
          draw();
        }
      }, 30);
      if (rafId !== null) {
        cancelAnimationFrame(rafId);
        rafId = null;
      }
    }
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
    if (rafId !== null) cancelAnimationFrame(rafId);
  });
</script>

<canvas bind:this={canvas} class="waveform"></canvas>

<style>
  .waveform {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
