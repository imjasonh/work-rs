use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use worker::*;

/// Compute SHA-256 hash of data using Web Crypto API
pub async fn compute_sha256(data: &[u8]) -> Result<String> {
    // In Cloudflare Workers, we use the global crypto object
    let global = js_sys::global();
    let crypto = js_sys::Reflect::get(&global, &JsValue::from_str("crypto"))
        .map_err(|_| Error::RustError("Failed to get crypto".to_string()))?;

    let subtle = js_sys::Reflect::get(&crypto, &JsValue::from_str("subtle"))
        .map_err(|_| Error::RustError("Failed to get subtle crypto".to_string()))?;

    // Create a Uint8Array from our data
    let data_array = Uint8Array::new_with_length(data.len() as u32);
    data_array.copy_from(data);

    // Compute SHA-256
    let digest_fn = js_sys::Reflect::get(&subtle, &JsValue::from_str("digest"))
        .map_err(|_| Error::RustError("Failed to get digest function".to_string()))?;

    let digest_fn = digest_fn
        .dyn_ref::<js_sys::Function>()
        .ok_or_else(|| Error::RustError("digest is not a function".to_string()))?;

    let promise = digest_fn
        .call2(&subtle, &JsValue::from_str("SHA-256"), &data_array)
        .map_err(|_| Error::RustError("Failed to call digest".to_string()))?;

    let promise = js_sys::Promise::from(promise);
    let result = JsFuture::from(promise)
        .await
        .map_err(|_| Error::RustError("Failed to compute hash".to_string()))?;

    // Convert result to Uint8Array
    let array = Uint8Array::new(&result);
    let mut bytes = vec![0u8; array.length() as usize];
    array.copy_to(&mut bytes);

    // Convert to hex string
    Ok(bytes_to_hex(&bytes))
}

/// Convert bytes to lowercase hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_hex() {
        assert_eq!(bytes_to_hex(&[0x00, 0xFF, 0x42]), "00ff42");
        assert_eq!(bytes_to_hex(&[]), "");
        assert_eq!(bytes_to_hex(&[0xDE, 0xAD, 0xBE, 0xEF]), "deadbeef");
    }
}
