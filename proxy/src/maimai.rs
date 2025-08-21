use std::io::ErrorKind::NotFound;
use httparse::Header;
use reqwest::redirect::Policy;
use reqwest::Url;
use anyhow::{anyhow, Error, Result};

use serde::Deserialize;

#[derive(Debug, serde::Deserialize)]
pub struct LoginResponse {
    #[serde(rename = "errorID")]
    pub error_id: u32,

    #[serde(rename = "openGameID")]
    pub open_game_id: String,

    #[serde(rename = "userID")]
    pub user_id: u64,

    #[serde(rename = "sessionId")]
    pub session_id: u64,

    #[serde(rename = "userPlayFlag")]
    pub user_play_flag: bool,

    #[serde(rename = "newUserIdFlag")]
    pub new_user_id_flag: bool,

    #[serde(rename = "openGameIDFlag")]
    pub open_game_id_flag: bool,
}


pub async fn maimai_handle<'headers, 'buf>(full_url:String, headers: &'headers mut [Header<'buf>])->Result<LoginResponse>{

        println!("\n✅ 成功捕获请求，准备使用 reqwest 转发 ✅");
        println!("URL: {}", full_url);

        let client = reqwest::Client::builder()
            .redirect(Policy::none()) // <-- 关键！禁止自动重定向
            .build()?;
        let method = reqwest::Method::GET;

        let mut req_builder = client.request(method, &full_url);
        println!("--> 正在转发其余 headers:");
        for header in headers.iter() {
            if !header.name.eq_ignore_ascii_case("Host") && !header.name.eq_ignore_ascii_case("Proxy-Connection") {
                println!("    {}: {}", header.name, std::str::from_utf8(header.value).unwrap_or("<invalid utf8>"));
                req_builder = req_builder.header(header.name, header.value);
            }
        }

        match req_builder.send().await {
            Ok(response) => {
                println!("\n✅ 成功从目标服务器获取data响应 ✅");
                println!("URL: {}", full_url);
                println!("<-- 响应状态: {}", response.status());

                let location = response.headers().get("location");
                if let Some(_location)=location{
                    println!("location:{}",_location.to_str()?.to_string());
                    let res = get_user_data_handle(_location.to_str()?.to_string()).await?;
                    Ok(res)
                }else {
                    Err(anyhow!("在响应中未找到 Location 头部"))
                }

            },
            Err(e) => Err(Error::from(e)),
        }
}
pub async fn get_open_url(url:&String)->Result<String>{
    let client = reqwest::Client::builder()
        .redirect(Policy::none()) // <-- 关键！禁止自动重定向
        .build()
        .unwrap();
    let method = reqwest::Method::GET;

    let mut req_builder = client.request(method, url);
    match req_builder.send().await {
        Ok(res)=>{
            println!("\n✅ 成功从目标服务器获取响应 ✅");
            println!("<-- 响应状态: {}", res.status());
            let location = res.headers().get("location");
            if let Some(_location)=location{
                let _location = _location.to_str()?.to_string();
                let _location = _location.replace("https%3A%2F%2Ftgk-wcaime.wahlap.com","http%3A%2F%2Ftgk-wcaime.wahlap.com");
                println!("location:{}",_location);

                Ok(_location)
            }else {
                Err(anyhow!("在响应中未找到 Location 头部"))
            }
        },
        Err(e) => Err(Error::from(e)),
    }

}

pub async fn get_user_data_handle(url:String)->Result<LoginResponse>{
    let client = reqwest::Client::builder()
        .redirect(Policy::none()) // <-- 关键！禁止自动重定向
        .build()
        .unwrap();
    let method = reqwest::Method::GET;

    let mut req_builder = client.request(method, &url);
    match req_builder.send().await {
        Ok(res)=>{
            println!("\n✅ 成功从目标服务器获取响应 ✅");
            println!("<-- 响应状态: {}", res.status());
            let text = res.text().await?;
            let text = {let texts:Vec<&str>=text.split("login=").collect();texts[1]};
            let json_part = text
                .trim_end()                // 去掉 \n \r 空格
                .strip_suffix('"')         // 去掉最后一个 "
                .unwrap_or(text)
                .trim();                   // 再保险修剪一次

            println!("json_part:{json_part}");
            let parsed: LoginResponse = serde_json::from_str(&json_part)?;
            Ok(parsed)
        },
        Err(e) => Err(Error::from(e)),
    }

}
