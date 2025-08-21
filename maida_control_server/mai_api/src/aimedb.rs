use anyhow::Result;
use chrono::Local;
use log::{debug, info};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, CONNECTION, USER_AGENT};
use serde_json::{json, Value};
use sha2::{Sha256, Digest};
use std::time::SystemTime;
use regex::Regex;
use tokio;
const CHIP_ID: &str = "A63E-01E68606624";
const COMMON_KEY: &str = "XcW5FW4cPArBXEk4vzKz3CIrMuA5EVVW";
const API_URL: &str = "http://ai.sys-allnet.cn/wc_aime/api/get_data";

pub fn get_sha256(input_str: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input_str.as_bytes());
    format!("{:X}", hasher.finalize())
}

pub fn generate_sega_timestamp() -> String {
    Local::now().format("%y%m%d%H%M%S").to_string()
}

pub fn calc_sega_aimedb_auth_key(var_string: &str, timestamp: &str, common_key: &str) -> String {
    get_sha256(&format!("{}{}{}", var_string, timestamp, common_key))
}

pub async fn api_aimedb(qr_code: &str) -> Result<String> {
    let timestamp = generate_sega_timestamp();
    let current_key = calc_sega_aimedb_auth_key(CHIP_ID, &timestamp, COMMON_KEY);

    let payload = json!({
        "chipID": CHIP_ID,
        "openGameID": "MAID", 
        "key": current_key,
        "qrCode": qr_code,
        "timestamp": timestamp
    });

    println!("Payload: {}", payload.to_string());

    let mut headers = HeaderMap::new();
    headers.insert(CONNECTION, HeaderValue::from_static("Keep-Alive"));
    headers.insert(USER_AGENT, HeaderValue::from_static("WC_AIME_LIB"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::new();
    let response = client
        .post(API_URL)
        .headers(headers)
        .json(&payload)
        .send()
        .await?;

    Ok(response.text().await?)
}

pub fn is_sgwc_format(input_string: &str) -> bool {
    if input_string.len() != 84
        || !input_string.starts_with("SGWCMAID") {
        return false;
    }

    let re = Regex::new(r"^[0-9A-F]+$").unwrap();
    re.is_match(&input_string[20..])
}

pub async fn impl_aimedb(qr_code: &str, is_already_final: bool) -> Result<String> {
    let qr_code_final = if is_already_final {
        qr_code.to_string()
    } else {
        qr_code[20..].to_string()
    };

    let response = api_aimedb(&qr_code_final).await?;

    debug!("implAimeDB: Response Body is: {}", response);
    Ok(response)
}

pub async fn impl_get_uid(qr_content: &str) -> Value {
    if !is_sgwc_format(qr_content) {
        return json!({"errorID": 60001});
    }

    match impl_aimedb(qr_content, false).await {
        Ok(response) => {
            match serde_json::from_str(&response) {
                Ok(result) => {
                    info!("QRScan Got Response {:?}", result);
                    result
                }
                Err(_) => json!({"errorID": 60002})
            }
        }
        Err(_) => json!({"errorID": 60002})
    }
}

#[tokio::test]
async fn test() {
    let sgq ="SGWCMAID250702201530F50EFA944761EEE401D1B86A556603F470377B613DA5A77CEEEA219E978209AE";
    println!("{:?}", impl_get_uid(sgq).await);
}