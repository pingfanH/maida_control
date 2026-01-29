<script setup>
import { useRoute } from 'vue-router'
import router from "@/lib/router.js";

if (localStorage.getItem("user_id")){
  router.push({ path: '/home' });
}

const redirectBase = encodeURIComponent(window.location.origin);
location.href = `/mc/oauth/authorize/maimai-dx?redirect_base=${redirectBase}`;
</script>

<template>


</template>

<style scoped>

</style>
