import { api, dxNet } from "@/lib/request";

export function getFavorites(){
//     dxNet("FavoriteUpdateMusicHtmlCache").then(()=>{
  
// });
    return dxNet("Favorites")
}
export function getOwnHomeData(){
    return dxNet("OwnHomeData")
}
export function getSession(){
    return dxNet("Session")
}
export function getMusicList(){
    return dxNet("MusicList")
}
export function getLocalFavorites(){
    return dxNet("FavoriteList")
}
export function setLocalFavorites(songIds: number[]){
    return api.post('/api/favorites', { song_ids: songIds })
}
export function syncFavorites(){
    return api.post('/api/favorites/sync', {})
}
