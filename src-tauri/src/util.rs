use base64::Engine;
use base64::engine::general_purpose::STANDARD;

pub fn encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

pub fn decode(text: &str) -> anyhow::Result<Vec<u8>> {
    Ok(STANDARD.decode(text)?)
}
