<script setup lang="ts">
import router from "@/lib/router";
import {ref, onMounted} from "vue";
import {getOwnHomeData, getSession,syncFavorites} from "@/lib/api";

var ownHomeData=ref({});
const loading = ref(true);
onMounted(async () => {
  try {
    const sessionRes = await getSession();
    if (!sessionRes?.data?.exists) {
      router.push({ path: '/' });
      return;
    }
  } catch (e) {
    router.push({ path: '/' });
    return;
  }
  try {
    const res = await getOwnHomeData();
    console.log(res);
    if (res?.data?.redirect) {
      window.location.href = res.data.redirect;
      return;
    }
    ownHomeData.value = res["data"];
  } finally {
    loading.value = false;
  }
});

</script>

<template>
  <div v-if="loading" class="loading-mask">
    <div class="loading-card">
      <div class="spinner"></div>
      <div class="loading-text">加载中...</div>
    </div>
  </div>
  <h1>MaiDaControl Demo</h1>
  <div>用户名:{{ownHomeData.userName}}</div>
  <div>Rating:{{ownHomeData.playerRating}}</div>
  <div>Ver.{{ownHomeData.lastRomVersion}}</div>
  <div style="margin-top: 12px;">
    <a href="/favorites">查看收藏列表</a>
  </div>
  <div style="margin-top: 8px;">
    <a href="/clash">Clash 一键导入/分享</a>
  </div>
</template>

<style scoped>
.loading-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.loading-card {
  background: #ffffff;
  padding: 18px 24px;
  border-radius: 10px;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
  display: flex;
  align-items: center;
  gap: 12px;
}

.spinner {
  width: 22px;
  height: 22px;
  border: 3px solid #e0e0e0;
  border-top-color: #1f6feb;
  border-radius: 50%;
  animation: spin 0.9s linear infinite;
}

.loading-text {
  font-size: 14px;
  color: #333333;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
