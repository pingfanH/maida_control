pub mod helper_get_user_thing;

use serde_json::json;
pub use helper_get_user_thing::*;
pub mod title_server;
pub mod config;
pub mod aes_pkcs7;
pub mod get_preview;
pub mod aimedb;
pub mod helper_get_user_music_detail;
pub mod utils;
pub mod music_data;

use anyhow::Result;
pub use title_server::*;
use tokio;
use log::{debug, info, warn};
use aes_pkcs7::AesPkcs7;
// macro_rules! generate_api_call {
//     ($( $api_name:ident ),*) => {
//         $(
//             pub async fn $api_name(data: serde_json::Value) -> Result<String> {
//                 api_sbga(data, stringify!($api_name), &config::TEST_UID.to_string()).await
//             }
//         )*
//     };
// }
macro_rules! generate_api_call {
    ($( $api_name:ident )*) => {
        $(
            // 将 snake_case 转为 camelCase
            pub async fn $api_name(data: serde_json::Value,user_agent_extra_data:String) -> Result<String> {
                let camel_case_name = snake_to_pascal(stringify!($api_name));
                api_sbga(data, &camel_case_name, user_agent_extra_data).await
            }
        )*
    };
}

fn snake_to_pascal(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true; // 首字母要大写

    for c in s.chars() {
        if c == '_' {
            capitalize = true;
            continue;
        }
        if capitalize {
            result.push(c.to_uppercase().next().unwrap());
            capitalize = false;
        } else {
            result.push(c);
        }
    }

    result
}

generate_api_call! {
    get_user_card_api
    get_user_character_api
    get_user_charge_api
    get_user_course_api
    get_user_data_api
    get_user_extend_api
    get_user_favorite_api
    get_user_friend_season_ranking_api
    get_user_ghost_api
    get_user_item_api
    get_game_charge_api
    get_user_login_bonus_api
    get_game_event_api
    get_user_map_api
    get_game_ng_music_id_api
    get_user_music_api
    get_game_ranking_api
    get_game_setting_api
    get_user_option_api
    get_user_portrait_api
    get_game_tournament_info_api
    user_logout_api
    get_user_preview_api
    get_transfer_friend_api
    get_user_rating_api
    get_user_activity_api
    get_user_recommend_rate_music_api
    get_user_recommend_select_music_api
    get_user_region_api
    get_user_score_ranking_api
    upload_user_photo_api
    upload_user_playlog_api
    upload_user_portrait_api
    upsert_client_bookkeeping_api
    upsert_client_setting_api
    upsert_client_testmode_api
    upsert_client_upload_api
    upsert_user_all_api
    upsert_user_chargelog_api
    user_login_api
    ping
    get_user_favorite_item_api
    get_game_ng_word_list_api
}

#[tokio::test]
async fn test() ->Result<()>{

    let data = json!({
        "userId":config::TEST_UID
    });

    let response = get_user_preview_api(data,config::TEST_UID.to_string()).await;

    // println!("{:?}",response)
    match response {
        Ok(response_data) => {
            println!("success:{}",response_data)
        },
        Err(e) => {
            println!("Request failed: {:?}", e);
        }
    }
    Ok(())
}
