pub mod vo;

use reqwest::header::COOKIE;
use reqwest::{Error, Response};
use reqwest::redirect::Policy;
use crate::BASE_API;
use crate::mobile_handle::vo::UserData;

pub fn get_favorites(){
    let url = BASE_API.to_owned() +"home/userOption/favorite/musicList";
    reqwest::get(url);

}

pub async fn get_records(user_data: UserData){
    let url = BASE_API.to_owned() +"record";
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build().unwrap();
    let method = reqwest::Method::GET;

    let mut req_builder = client.request(method, &url)
        .header(COOKIE,format!("_t={};userId={}",user_data.session_id,user_data.open_user_id))
        ;
   match req_builder.send().await  {
       Ok(res) => {
           println!("{:#?}", res);
       }
       Err(e) => {}
   };


}