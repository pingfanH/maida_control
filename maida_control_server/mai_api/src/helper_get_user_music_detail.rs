use anyhow::Result;
use serde_json::{self, json};
use log::info;
use sea_orm::prelude::*;
use crate::config::{COMBO_ID_TO_NAME, SYNC_ID_TO_NAME, TEST_UID};
use crate::database::{achievements, user};
use crate::mai_api::music_data::{get_music_data, get_music_title};
use super::{api_sbga, get_user_music_api};


pub async fn get_user_music_detail(user_id: i32, next_index: i32, max_count: i32) -> Result<serde_json::Value> {
    let data = json!({
        "userId": user_id,
        "nextIndex": next_index,
        "maxCount": max_count
    });
    let response = get_user_music_api(data, user_id.to_string()).await?;
    Ok(serde_json::from_str(&response)?)
}

pub async fn get_user_full_music_detail(user_id: i32) -> Result<Vec<serde_json::Value>> {
    let mut current_user_music_detail_list = Vec::new();
    let mut next_index = 0;

    loop {
        let user_music_response = get_user_music_detail(user_id, next_index, 50).await?;

        next_index = user_music_response["nextIndex"].as_i64().map(|x| x as i32).expect("REASON");
        println!("NextIndex: {:?}", next_index);

        if user_music_response["userMusicList"].as_array().map_or(true, |x| x.is_empty()) {
            break;
        }

        if let Some(music_list) = user_music_response["userMusicList"].as_array() {
            for current_music in music_list {
                if let Some(detail_list) = current_music["userMusicDetailList"].as_array() {
                    for detail in detail_list {
                        if detail["playCount"].as_i64().unwrap_or(0) > 0 {
                            current_user_music_detail_list.push(detail.clone());
                        }
                    }
                }
            }
        }
        if next_index==0 { break }
    }

    Ok(current_user_music_detail_list)
}

use sea_orm::{sea_query, DatabaseConnection, EntityTrait, Set, TryInsert};
use sea_orm::sea_query::OnConflict;
use serde_json::Value;
use crate::database::prelude::User;
use crate::mai_api::utils::single_ra;

pub async fn upsert_user_music_detail(
    user_qq: i64,
    songs: Vec<Value>,
    db: &DatabaseConnection,
) -> Result<()> {

    let user = User::find().filter(user::Column::Qq.eq(user_qq)).one(&*db).await?;
    if user.is_none() { return Err(anyhow::anyhow!("user not found")) }
    let user = user.unwrap();
    let mut achievements=Vec::new();
    for song in songs {

        let song_id = song.get("musicId").unwrap().as_i64().unwrap() as i32;
        if let Some(_song) = get_music_data(song_id){
            println!("{song:?}");
            // 创建新的 ActiveModel 对象
            let achieve = song["achievement"].as_i64().unwrap_or(0) as i32;
            let mut level = song["level"].as_i64().unwrap_or(0) as i32;
            if level==10 { level=0 }
            let constant = _song.ds[level as usize];
            let achievement = achievements::ActiveModel {
                id: sea_orm::NotSet, // id 为空表示这是插入操作
                song_id: sea_orm::Set(song_id),
                achievements: sea_orm::Set(achieve.clone()),
                uid: sea_orm::Set(user.id),
                dx_score: sea_orm::Set(song["deluxscoreMax"].as_i64().unwrap_or(0) as i32),
                level_index: sea_orm::Set(level),
                fc: sea_orm::Set(COMBO_ID_TO_NAME[song["comboStatus"].as_i64().unwrap() as usize].to_string()),
                fs: sea_orm::Set(SYNC_ID_TO_NAME[song["syncStatus"].as_i64().unwrap() as usize].to_string()),
                ra:Set(single_ra(achieve,constant)),
                constant: Set(constant),
            };
            achievements.push(achievement);
        }else{
            println!("unknow song {song_id:?}");
        }

    }


    let conflict = OnConflict::columns([
        achievements::Column::LevelIndex,
        achievements::Column::SongId,
        achievements::Column::Uid,
    ])
        .update_columns(vec![
            achievements::Column::Achievements,
            achievements::Column::DxScore,
            achievements::Column::Fc,
            achievements::Column::Fs,
        ])
        .to_owned();

    let result = achievements::Entity::insert_many(achievements)
        .on_conflict(conflict)
        .do_nothing()
        .exec(db)
        .await?;
    println!("{result:?}");
    Ok(())
}


pub async fn parse_user_full_music_detail(user_full_music_detail_list: Vec<serde_json::Value>) -> Result<Vec<serde_json::Value>> {
    let mut music_detail_list = Vec::new();

    for detail in user_full_music_detail_list {
        println!("{detail}");
        music_detail_list.push(json!({
            "id": detail["musicId"].as_i64().unwrap_or(0) as i32,
            "歌名": get_music_title(detail["musicId"].as_i64().unwrap_or(0) as i32).unwrap_or("？？？".to_string()),
            "难度": detail["level"],
            "分数": detail["achievement"].as_f64().unwrap_or(0.0) / 10000.0,
            "DX分数": detail["deluxscoreMax"]
        }));
    }

    Ok(music_detail_list)
}

#[cfg(test)]
mod tests {
    use kovi::tokio;
    use crate::mai_api::music_data::load_music_data;
    use super::*;

    #[tokio::test]
    async fn test_get_user_music_detail() -> Result<()> {
        load_music_data().ok();
        let user_full_music_detail_list = get_user_full_music_detail(TEST_UID).await?;
        println!("{:?}", user_full_music_detail_list.len());
        let parsed_detail = parse_user_full_music_detail(user_full_music_detail_list).await?;
        println!("{:?}", parsed_detail);
        Ok(())
    }
}