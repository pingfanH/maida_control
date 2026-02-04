<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { getFavorites, getLocalFavorites, getMusicList, setLocalFavorites, syncFavorites } from "@/lib/api";

type FavoriteItem = {
  category: string;
  title: string;
  artist: string;
  musicType: string;
  image: string;
};

type MusicItem = {
  id: number;
  title: string;
  artist: string;
  genre: string;
  type: string;
  aliases?: string | null;
};

const loading = ref(true);
const error = ref<string | null>(null);
const items = ref<FavoriteItem[]>([]);

const musicLoading = ref(true);
const musicError = ref<string | null>(null);
const musicItems = ref<MusicItem[]>([]);
const localSelected = ref<Set<number>>(new Set());
const search = ref("");
const saving = ref(false);
const syncing = ref(false);
const localInfo = ref<string | null>(null);

const refreshFavorites = async () => {
  try {
    const res = await getFavorites();
    items.value = res.data?.items || [];
    error.value = null;
  } catch (e: any) {
    error.value = e?.message || "请求失败";
  } finally {
    loading.value = false;
  }
};

refreshFavorites();

Promise.all([getMusicList(), getLocalFavorites()])
  .then(([musicRes, favRes]) => {
    musicItems.value = musicRes.data?.items || [];
    const favIds = new Set<number>((favRes.data?.items || []) as number[]);
    localSelected.value = favIds;
  })
  .catch((e) => {
    musicError.value = e?.message || "加载失败";
  })
  .finally(() => {
    musicLoading.value = false;
  });

const list = computed(() => items.value);
const localCount = computed(() => localSelected.value.size);
const favoriteIdSet = computed(() => {
  const ids = new Set<number>();
  for (const fav of list.value) {
    const found = musicItems.value.find(
      (m) => m.title === fav.title && m.artist === fav.artist
    );
    if (found) {
      ids.add(found.id);
    }
  }
  return ids;
});
const displayMusic = computed(() => {
  const key = search.value.trim().toLowerCase();
  const filtered = key
    ? musicItems.value.filter((item) =>
        item.title.toLowerCase().includes(key) ||
        item.artist.toLowerCase().includes(key) ||
        (item.aliases || "").toLowerCase().includes(key)
      )
    : musicItems.value;
  const favs = filtered.filter((item) => favoriteIdSet.value.has(item.id));
  const rest = filtered.filter((item) => !favoriteIdSet.value.has(item.id));
  return [...favs, ...rest];
});
const pageSize = 24;
const currentPage = ref(1);
const totalPages = computed(() =>
  Math.max(1, Math.ceil(displayMusic.value.length / pageSize))
);
const pagedMusic = computed(() => {
  const start = (currentPage.value - 1) * pageSize;
  return displayMusic.value.slice(start, start + pageSize);
});
const prevPage = () => {
  currentPage.value = Math.max(1, currentPage.value - 1);
};
const nextPage = () => {
  currentPage.value = Math.min(totalPages.value, currentPage.value + 1);
};
watch(displayMusic, () => {
  currentPage.value = 1;
});
const aliasHint = (item: MusicItem) => {
  const key = search.value.trim().toLowerCase();
  if (!key || !item.aliases) return "";
  const matched = item.aliases
    .split(",")
    .map((alias) => alias.trim())
    .filter((alias) => alias && alias.toLowerCase().includes(key));
  return matched.join(" / ");
};

const coverUrl = (id: number) => {
  const padded = String(id).padStart(5, "0");
  return `https://www.diving-fish.com/covers/${padded}.png`;
};

const toggleLocal = (id: number) => {
  localInfo.value = null;
  const next = new Set(localSelected.value);
  if (next.has(id)) {
    next.delete(id);
    localSelected.value = next;
    return;
  }
  if (next.size >= 50) {
    localInfo.value = "收藏上限 50 首";
    return;
  }
  next.add(id);
  localSelected.value = next;
};

const saveLocal = async () => {
  saving.value = true;
  localInfo.value = null;
  try {
    const ids = Array.from(localSelected.value);
    await setLocalFavorites(ids);
    localInfo.value = "本地收藏已保存";
  } catch (e: any) {
    localInfo.value = e?.message || "保存失败";
  } finally {
    saving.value = false;
  }
};

const syncRemote = async () => {
  syncing.value = true;
  localInfo.value = null;
  try {
    const ids = Array.from(localSelected.value);
    await setLocalFavorites(ids);
    const res = await syncFavorites();
    if (res.data?.error) {
      throw new Error(res.data.error);
    }
    if (res.data?.updated !== true) {
      throw new Error("同步失败");
    }
    await refreshFavorites();
    const missing = res.data?.missing?.length ? `，未匹配 ${res.data.missing.length} 首` : "";
    localInfo.value = `同步完成${missing}`;
  } catch (e: any) {
    localInfo.value = e?.message || "同步失败";
  } finally {
    syncing.value = false;
  }
};
</script>

<template>
  <h1>收藏列表</h1>
  <div class="local-block">
    <h2>收藏</h2>
    <div class="toolbar">
      <input v-model="search" placeholder="搜索歌名/作者" />
      <div class="count">已选 {{ localCount }}/50</div>
      <!-- <button class="btn" :disabled="saving" @click="saveLocal">
        {{ saving ? "保存中..." : "保存本地收藏" }}
      </button> -->
      <button class="btn primary" :disabled="syncing" @click="syncRemote">
        {{ syncing ? "同步中..." : "保存" }}
      </button>
    </div>
    <div class="info" v-if="localInfo">{{ localInfo }}</div>
    <div v-if="musicLoading">加载中...</div>
    <div v-else-if="musicError">错误：{{ musicError }}</div>
    <div v-else class="music-card-grid">
      <label v-for="item in pagedMusic" :key="item.id" class="music-card">
        <input
          type="checkbox"
          :checked="localSelected.has(item.id) || favoriteIdSet.has(item.id)"
          @change="toggleLocal(item.id)"
        />
        <img :src="coverUrl(item.id)" alt="" class="music-cover" />
        <div class="music-meta">
          <div class="title">
            {{ item.title }}
            <span v-if="aliasHint(item)" class="alias-inline">（{{ aliasHint(item) }}）</span>
          </div>
          <div class="artist">{{ item.artist }}</div>
        </div>
      </label>
    </div>
    <div class="pager">
      <button class="btn" :disabled="currentPage === 1" @click="prevPage">上一页</button>
      <div class="page-info">第 {{ currentPage }} / {{ totalPages }} 页</div>
      <button class="btn" :disabled="currentPage === totalPages" @click="nextPage">下一页</button>
    </div>
  </div>
  <div v-if="loading">加载中...</div>
  <div v-else-if="error">错误：{{ error }}</div>
</template>

<style scoped>
.local-block {
  padding: 8px 0 20px;
  border-bottom: 1px solid #e5e7eb;
  margin-bottom: 16px;
}
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
  flex-wrap: wrap;
}
.toolbar input {
  flex: 1 1 220px;
  padding: 6px 8px;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
}
.count {
  font-size: 12px;
  color: #6b7280;
}
.btn {
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid #d1d5db;
  background: #fff;
  cursor: pointer;
}
.btn.primary {
  background: #0f172a;
  color: #fff;
  border-color: #0f172a;
}
.info {
  font-size: 12px;
  color: #1f2937;
  margin-bottom: 8px;
}
.music-card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
}
.music-card {
  display: grid;
  grid-template-columns: 18px 64px 1fr;
  gap: 10px;
  align-items: center;
  padding: 10px;
  border: 1px solid #e5e7eb;
  border-radius: 10px;
  background: #fff;
}
.music-cover {
  width: 64px;
  height: 64px;
  object-fit: cover;
  border-radius: 6px;
  background: #f3f4f6;
}
.music-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.music-card .title {
  font-weight: 600;
}
.music-card .artist {
  color: #6b7280;
  font-size: 12px;
}
.music-card .alias-inline {
  color: #9ca3af;
  font-size: 12px;
  font-weight: 400;
}
.pager {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
}
.page-info {
  font-size: 12px;
  color: #6b7280;
}
.list {
  padding: 8px 0 24px;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
}
.card {
  display: flex;
  gap: 10px;
  padding: 10px;
  border: 1px solid #e5e7eb;
  border-radius: 10px;
  background: #fff;
  align-items: center;
}
.cover {
  width: 72px;
  height: 72px;
  object-fit: cover;
  border-radius: 6px;
  background: #f3f4f6;
}
.meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.title {
  font-weight: 600;
}
.artist {
  color: #6b7280;
  font-size: 12px;
}
.tag {
  display: inline-block;
  align-self: flex-start;
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 999px;
  background: #eef2ff;
  color: #3730a3;
}
.tag[data-type="dx"] {
  background: #ecfeff;
  color: #155e75;
}
.tag[data-type="standard"] {
  background: #fef3c7;
  color: #92400e;
}
.tag[data-type="both"] {
  background: #ede9fe;
  color: #5b21b6;
}
</style>
