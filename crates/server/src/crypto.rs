use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::Key;
use rand::RngCore;
use std::path::Path;
use std::sync::OnceLock;

static KEY: OnceLock<[u8; 32]> = OnceLock::new();

pub fn init(data_dir: &Path) {
    let key_path = data_dir.join(".crypto_key");
    let key: [u8; 32] = if key_path.exists() {
        let raw = std::fs::read(&key_path).expect("Error al leer llave criptográfica");
        if raw.len() != 32 {
            panic!("Archivo .crypto_key corrupto: tamaño inválido");
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&raw);
        arr
    } else {
        let mut arr = [0u8; 32];
        OsRng.fill_bytes(&mut arr);
        std::fs::write(&key_path, &arr).expect("Error al guardar llave criptográfica");
        arr
    };
    KEY.set(key).ok();
}

pub fn encrypt(plaintext: &str) -> String {
    let key_bytes = KEY.get().expect("Crypto no inicializado");
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(&nonce.into(), plaintext.as_bytes())
        .expect("Error de cifrado AES-256-GCM");
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    encode_hex(&combined)
}

pub fn decrypt(encoded: &str) -> Option<String> {
    let key_bytes = KEY.get()?;
    let data = decode_hex(encoded)?;
    if data.len() < 12 {
        return None;
    }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    cipher
        .decrypt(nonce_bytes.into(), ciphertext)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

pub fn encrypt_opt(value: &Option<String>) -> Option<String> {
    value.as_ref().filter(|s| !s.is_empty()).map(|s| encrypt(s))
}

pub fn decrypt_opt(value: &Option<String>) -> Option<String> {
    match value {
        Some(s) if !s.is_empty() && s.len() > 20 => decrypt(s).or_else(|| Some("[ERROR DESCIFRADO]".to_string())),
        other => other.clone(),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}
