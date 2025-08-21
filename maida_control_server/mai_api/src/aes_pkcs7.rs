

use aes::Aes256;
use block_modes::{BlockMode, Cbc};
use block_modes::block_padding::Pkcs7;
use std::str;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};
use flate2::read::ZlibDecoder;
use super::config::AES_KEY;
use super::config::AES_IV;
use anyhow::Result;
// 类型别名
type Aes256Cbc = Cbc<Aes256, Pkcs7>;

pub struct AesPkcs7;

impl AesPkcs7 {
    pub fn encrypt(content: &[u8]) -> Result<Vec<u8>> {
            let content = zlib_compress(content)?;
            let cipher = Aes256Cbc::new_from_slices(AES_KEY.as_ref(), AES_IV.as_ref())?;
            Ok(cipher.encrypt_vec(&*content))
    }

    pub fn decrypt(content: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Cbc::new_from_slices(AES_KEY.as_ref(), AES_IV.as_ref())?;
        let data =cipher.decrypt_vec(&*content)?;
        Ok(zlib_uncompress(&*data)?)
    }

    pub fn pkcs7padding(text: &str) -> String {
        let bs = 16;
        let length = text.len();
        let bytes_length = text.as_bytes().len();
        let padding_size = if bytes_length == length { length } else { bytes_length };
        let padding = bs - padding_size % bs;
        let padding_text = std::iter::repeat(char::from_u32(padding as u32).unwrap())
            .take(padding)
            .collect::<String>();
        format!("{}{}", text, padding_text)
    }

    pub fn pkcs7unpadding(text: &str) -> String {
        let length = text.len();
        let unpadding = text.chars().last().unwrap() as usize;
        text[..length - unpadding].to_string()
    }
}

pub fn zlib_compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}
pub fn zlib_uncompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

// 示例用法
// let aes = AesPkcs7::new("a>32bVP7v<63BVLkY[xM>daZ1s9MBP<R", "d6xHIKq]1J]Dt^ue");
// let encrypted = aes.encrypt(b"hello world");
// let decrypted = aes.decrypt(&encrypted);

