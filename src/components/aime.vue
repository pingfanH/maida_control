<script setup>
import router from "@/lib/router.js";
import { getOwnHomeData, getSession } from "@/lib/api";

const redirectToOauth = () => {
  location.href = `http://tgk-wcaime.wahlap.com/wc_auth/oauth/authorize/maimai-dx`;
};

getSession()
  .then((res) => {
    if (!res?.data?.exists) {
      redirectToOauth();
      return;
    }
    getOwnHomeData()
      .then((homeRes) => {
        if (homeRes?.data?.redirect) {
          redirectToOauth();
          return;
        }
        router.push({ path: '/home' });
      })
      .catch(() => {
        redirectToOauth();
      });
  })
  .catch(() => {
    redirectToOauth();
  });
</script>

<template>


</template>

<style scoped>

</style>
