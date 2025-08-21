<script setup>
import { useRoute } from 'vue-router'
import router from "@/lib/router.js";

if (localStorage.getItem("user_id")){
  router.push({ path: '/home' });
}

location.href="http://tgk-wcaime.wahlap.com/wc_auth/oauth/authorize/maimai-dx";
</script>

<template>


</template>

<style scoped>

</style>
