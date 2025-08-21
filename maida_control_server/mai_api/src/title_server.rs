
use aes;
use block_modes;
use flate2;

use reqwest;
use log;
use env_logger;

use aes::{Aes128, BlockCipher};
use block_modes::{BlockMode, Cbc};
use block_padding::Padding;
use block_modes::block_padding::Pkcs7;
use flate2::read::ZlibDecoder;
use std::io::prelude::*;
use std::io;
use std::time::Duration;
use reqwest::{Client, StatusCode};
use reqwest::header::{HeaderMap, USER_AGENT, CONTENT_TYPE};
use log::{info, debug, warn};
use anyhow::Result;
use env_logger::Env;
use kovi::tokio::time::sleep;
use md5;
use serde_json::Value;
use super::config::*;
use crate::mai_api::aes_pkcs7::{zlib_compress, AesPkcs7};
type Aes128Cbc = Cbc<Aes128, Pkcs7>;

// 舞萌DX 2024
// omg it's leaking
// AES_KEY = "n7bx6:@Fg_:2;5E89Phy7AyIcpxEQ:R@"
// AES_IV = ";;KjR1C3hgB1ovXa"
// OBFUSCATE_PARAM = "BEs2D5vW"

// 2025


pub fn init_logger() {
    // let env = Env::default();
    // 
    // env_logger::init_from_env(env);
}
//
// pub fn aes_encrypt(data: &[u8]) -> Vec<u8> {
//     let cipher = Aes128Cbc::new_from_slices(AES_KEY, AES_IV).unwrap();
//     cipher.encrypt_vec(data)
// }
//
// pub fn aes_decrypt(data: &[u8]) -> Vec<u8> {
//     let cipher = Aes128Cbc::new_from_slices(AES_KEY, AES_IV).unwrap();
//     cipher.decrypt_vec(data).unwrap()
// }
//
// pub fn zlib_decompress(data: &[u8]) -> io::Result<Vec<u8>> {
//     let mut decoder = ZlibDecoder::new(data);
//     let mut decompressed = Vec::new();
//     decoder.read_to_end(&mut decompressed)?;
//     Ok(decompressed)
// }
fn get_sdgb_api_hash(api: &str) -> String {
    let input = format!("{}MaimaiChn{}", api,OBFUSCATE_PARAM);
    let digest = md5::compute(input.as_bytes());
    format!("{:x}", digest)
}


/// 舞萌DX 2025 API 通讯用函数
///
/// # 参数
/// - `data`: 请求数据
/// - `target_api`: 使用的 API
/// - `user_agent_extra_data`: UA 附加信息，机台相关则为狗号（如 A63E01E9564），用户相关则为 UID
/// - `no_log`: 是否不记录日志
/// - `timeout`: 请求超时时间（秒）
///
/// # 返回
/// 解码后的响应数据


pub async fn api_sbga(
    data: Value,
    target_api: &str,
    user_agent_extra_data: String,
) -> Result<String> {
    init_logger();
    let data = serde_json::to_string(&data)?;
    println!("data:{}", data);
    println!("target_api:{}", target_api);
    let encrypted_data = AesPkcs7::encrypt(data.as_ref())?;
    let client = Client::new();
    let url = format!("{ENDPOINT}{}", get_sdgb_api_hash(target_api));

    let mut headers = HeaderMap::new();
    let hashed = format!("{}#{}", get_sdgb_api_hash(target_api), user_agent_extra_data);
    headers.insert(USER_AGENT, hashed.parse()?);
    headers.insert(CONTENT_TYPE, "application/json".parse()?);
    headers.insert("Mai-Encoding", "1.50".parse()?);
    headers.insert("Accept-Encoding", "".parse()?);
    headers.insert("Charset", "UTF-8".parse()?);
    headers.insert("Content-Encoding", "deflate".parse()?);
    headers.insert("Expect", "100-continue".parse()?);

    // 设置最大重试次数和重试间隔
    let max_retries = 3;
    let retry_interval = Duration::from_secs(1);

    for attempt in 0..max_retries {
        let response = client.post(&url)
            .headers(headers.clone())  // 克隆 headers
            .body(encrypted_data.to_vec())
            .send()
            .await?;

        println!("{:?}", response);

        // 如果响应状态是 OK，则解密并返回数据
        if response.status() == StatusCode::OK {
            let res = response.bytes().await?;

            // 添加解密重试
            let mut content = Vec::new();
            let mut decryption_attempt = 0;
            loop {
                match AesPkcs7::decrypt(res.as_ref()) {
                    Ok(decoded_content) => {
                        content = decoded_content;
                        break;
                    }
                    Err(e) => {
                        decryption_attempt += 1;
                        eprintln!("Decryption attempt {} failed: {}", decryption_attempt, e);
                        if decryption_attempt < max_retries {
                            println!("Retrying decryption in {} seconds...", retry_interval.as_secs());
                            sleep(retry_interval).await;
                        } else {
                            return Err(anyhow::Error::msg(format!("Decryption failed after {} attempts: {}", max_retries, e)));
                        }
                    }
                }
            }

            // 解密成功，返回结果
            match String::from_utf8(content) {
                Ok(decoded_content) => return Ok(decoded_content),
                Err(e) => return Err(anyhow::Error::msg(format!("UTF-8 decoding failed: {}", e))),
            }
        } else {
            // 打印错误并判断是否需要重试
            let error_text = response.text().await?;
            eprintln!("Request failed (attempt {}): {}", attempt + 1, error_text);

            if attempt + 1 < max_retries {
                // 如果不是最后一次尝试，等待一段时间然后重试
                println!("Retrying in {} seconds...", retry_interval.as_secs());
                sleep(retry_interval).await;
            } else {
                // 如果是最后一次尝试，直接返回错误
                return Err(anyhow::Error::msg(error_text));
            }
        }
    }

    Err(anyhow::Error::msg("Max retries reached"))
}
