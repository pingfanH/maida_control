<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

const configUrl = computed(() => {
  if (typeof window === "undefined") {
    return "/clashconf/maida.yaml";
  }
  return `${window.location.origin}/clashconf/maida.yaml`;
});

const installLink = computed(
  () => `clash://install-config?url=${encodeURIComponent(configUrl.value)}`
);
const openLink = computed(
  () => `clash://open?url=${encodeURIComponent(configUrl.value)}`
);

const configText = ref<string>("");
const copyInfo = ref<string | null>(null);
const copyFallback = ref<string | null>(null);
const pageUrl = ref<string>("");
const mainUrl = ref<string>("");

const fetchConfig = async () => {
  try {
    const res = await fetch(configUrl.value, { cache: "no-store" });
    configText.value = await res.text();
  } catch {
    configText.value = "";
  }
};

const copyText = async (text: string, label: string) => {
  copyInfo.value = null;
  copyFallback.value = null;
  try {
    await navigator.clipboard.writeText(text);
    copyInfo.value = `${label} 已复制`;
  } catch {
    copyInfo.value = `${label} 复制失败`;
    copyFallback.value = text;
  }
};

onMounted(() => {
  mainUrl.value = typeof window !== "undefined" ? window.location.origin : "";
  pageUrl.value = typeof window !== "undefined" ? window.location.href : "";
  fetchConfig();
});
</script>

<template>
  <div class="page">
    <h1>Maida-Control Demo</h1>
    <p  class="desc">舞萌第三方国服dxnet</p>
    <h1 >配置教程</h1>
    <p class="desc">1.一键导入配置或复制链接分享。<br/>(请在浏览器打开)</p>
    <div class="btn-row">
      <a class="btn primary" :href="installLink">一键导入（install-config）</a>
      <a class="btn" :href="openLink">一键导入（open）</a>
    </div>
    <div class="btn-row">
      <button class="btn" @click="copyText(configUrl, '配置链接')">复制配置链接</button>
      <button class="btn" @click="copyText(pageUrl, '页面链接')">复制页面链接</button>
    </div>
     <p class="desc">2.在微信打开链接  <button class="btn" @click="copyText(mainUrl, '主页链接')">复制主页链接</button></p>
    <div v-if="copyInfo" class="info">{{ copyInfo }}</div>
    <div v-if="copyFallback" class="fallback">
      复制失败，可长按复制：
      <span class="fallback-link">{{ copyFallback }}</span>
    </div>
    <div class="panel">
      <div class="panel-title">配置内容</div>
      <pre class="config">{{ configText || "加载失败或为空" }}</pre>
    </div>
  </div>
</template>

<style scoped>
.page {
  max-width: 820px;
  margin: 0 auto;
  padding: 24px 16px 40px;
}
.desc {
  color: #6b7280;
  margin-top: 6px;
}
.btn-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 12px;
}
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid #d1d5db;
  background: #fff;
  color: #111827;
  text-decoration: none;
  cursor: pointer;
}
.btn.primary {
  background: #0f172a;
  color: #fff;
  border-color: #0f172a;
}
.info {
  margin-top: 10px;
  font-size: 12px;
  color: #1f2937;
}
.fallback {
  margin-top: 8px;
  font-size: 12px;
  color: #111827;
  word-break: break-all;
}
.fallback-link {
  color: #2563eb;
}
.panel {
  margin-top: 16px;
  border: 1px solid #e5e7eb;
  border-radius: 10px;
  padding: 12px;
  background: #fff;
}
.panel-title {
  font-size: 12px;
  color: #6b7280;
  margin-bottom: 8px;
}
.config {
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 12px;
  line-height: 1.5;
}
</style>
