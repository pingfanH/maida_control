import {dxNet} from "@/lib/request";

export function getFavorites(){
    return dxNet("Favorites")
}
export function getOwnHomeData(){
    return dxNet("OwnHomeData")
}

