use hmac::{Hmac, Mac};
use sha1::Sha1;

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
pub const STEP: i64 = 30;

pub fn new_secret() -> String {
    encode(&rand::random::<[u8; 20]>())
}

pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for &b in bytes {
        buffer = (buffer << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            out.push(ALPHABET[((buffer >> (bits - 5)) & 31) as usize] as char);
            bits -= 5;
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

pub fn decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for c in text.chars().filter(|c| !c.is_whitespace() && *c != '=') {
        let v = ALPHABET.iter().position(|&a| a as char == c.to_ascii_uppercase())? as u32;
        buffer = (buffer << 5) | v;
        bits += 5;
        if bits >= 8 {
            out.push(((buffer >> (bits - 8)) & 255) as u8);
            bits -= 8;
        }
    }
    Some(out)
}

pub fn code_at(secret: &[u8], step: i64) -> String {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("hmac accepts any key length");
    mac.update(&step.to_be_bytes());
    let hash = mac.finalize().into_bytes();
    let offset = (hash[19] & 0xf) as usize;
    let value = u32::from_be_bytes([hash[offset] & 0x7f, hash[offset + 1], hash[offset + 2], hash[offset + 3]]);
    format!("{:06}", value % 1_000_000)
}

pub fn now_step() -> i64 {
    chrono::Utc::now().timestamp() / STEP
}

pub fn matching_step(secret: &str, code: &str, after: i64) -> Option<i64> {
    let code: String = code.chars().filter(|c| c.is_ascii_digit()).collect();
    if code.len() != 6 {
        return None;
    }
    let key = decode(secret)?;
    let now = now_step();
    (now - 1..=now + 1).find(|&s| s > after && code_at(&key, s) == code)
}

pub fn uri(secret: &str, account: &str, issuer: &str) -> String {
    let enc = |s: &str| s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>();
    format!("otpauth://totp/{}:{}?secret={secret}&issuer={}&algorithm=SHA1&digits=6&period={STEP}", enc(issuer), enc(account), enc(issuer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc6238_vectors() {
        let key = b"12345678901234567890";
        assert_eq!(code_at(key, 59 / STEP), "287082");
        assert_eq!(code_at(key, 1111111109 / STEP), "081804");
        assert_eq!(code_at(key, 2000000000 / STEP), "279037");
    }

    #[test]
    fn base32_roundtrip() {
        let bytes = rand::random::<[u8; 20]>();
        assert_eq!(decode(&encode(&bytes)).unwrap(), bytes);
        assert_eq!(encode(b"foobar"), "MZXW6YTBOI");
    }
}
