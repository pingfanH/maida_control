<script setup lang="ts">
import { useRoute } from 'vue-router'
import router from "@/lib/router";
import {api} from "@/lib/request";
import {ref} from "vue";
import {getOwnHomeData} from "@/lib/api";

var user_id=ref();
var open_game_id=ref();
var session_id=ref();
var open_user_id=ref();
var ownHomeData=ref();
const route = useRoute()
// 存储到cookie
if(route.query.user_id&&route.query.session_id&&route.query.open_user_id){
  localStorage.setItem('user_id', route.query.user_id);
  localStorage.setItem('session_id', route.query.session_id);
  localStorage.setItem('open_user_id', route.query.open_user_id);
}
if(!localStorage.getItem('user_id')){
  router.push({ path: '/' });
}else{
  user_id.value=localStorage.getItem('user_id');
  session_id.value=localStorage.getItem('session_id');
  open_user_id.value=localStorage.getItem('open_user_id');
}

getOwnHomeData().then(res=>{
  console.log(res)
  ownHomeData.value=res["data"];
});

</script>

<template>
  <h1>MaiDaControl Demo</h1>
  <div>用户名:{{ownHomeData.userName}}</div>
  <div>Rating:{{ownHomeData.playerRating}}</div>
  <div>Ver.{{ownHomeData.lastRomVersion}}</div>
</template>

<style scoped>

</style>