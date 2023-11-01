use cryptoxide::chacha20poly1305::ChaCha20Poly1305;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use std::ffi::c_int;

#[no_mangle]
pub extern "C" fn comp(in_offset: c_int, len: c_int, out_offset: c_int) -> c_int {
    let in_bytes = unsafe { std::slice::from_raw_parts(in_offset as *const u8, len as usize) };
    let compressed_bytes = compress_prepend_size(in_bytes);

    let out_len = compressed_bytes.len();

    unsafe {
        std::ptr::copy(
            compressed_bytes.as_ptr(),
            out_offset as *mut _,
            out_len as usize,
        )
    };

    out_len as c_int
}

#[no_mangle]
pub extern "C" fn decomp(in_offset: c_int, len: c_int, out_offset: c_int) -> c_int {
    let in_bytes = unsafe { std::slice::from_raw_parts(in_offset as *const u8, len as usize) };
    if let Ok(decomp_bytes) = decompress_size_prepended(in_bytes) {
        let out_len = decomp_bytes.len();

        unsafe {
            std::ptr::copy(
                decomp_bytes.as_ptr(),
                out_offset as *mut _,
                out_len as usize,
            )
        };

        return out_len as c_int;
    }

    return -1;
}

#[no_mangle]
pub extern "C" fn encrypt(in_offset: c_int, len: c_int, out_offset: c_int) -> c_int {
    let in_bytes = unsafe { std::slice::from_raw_parts(in_offset as *const u8, len as usize) };

    let key = [0u8; 16];
    let nonce: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let aad: [u8; 0] = [];
    let in_bytes_len = in_bytes.len();
    let mut out = vec![0u8; in_bytes_len + 16];
    let mut tag = [0u8; 16];

    // create a new cipher
    let mut cipher = ChaCha20Poly1305::new(&key, &nonce, &aad);

    // encrypt the msg and append the tag at the end
    cipher.encrypt(in_bytes, &mut out[0..in_bytes_len], &mut tag);
    out[in_bytes_len..].copy_from_slice(&tag);

    unsafe { std::ptr::copy(out.as_ptr(), out_offset as *mut _, out.len()) };

    out.len() as c_int
}

#[no_mangle]
pub extern "C" fn decrypt(in_offset: c_int, len: c_int, out_offset: c_int) -> c_int {
    let in_bytes = unsafe { std::slice::from_raw_parts(in_offset as *const u8, len as usize) };

    let key = [0u8; 16];
    let nonce: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let aad: [u8; 0] = [];

    let mut cipher = ChaCha20Poly1305::new(&key, &nonce, &aad);

    let in_bytes_len = in_bytes.len();
    let mut decrypt_msg = vec![0u8; in_bytes_len - 16];
    if !cipher.decrypt(
        &in_bytes[0..in_bytes_len - 16],
        &mut decrypt_msg,
        &in_bytes[in_bytes_len - 16..],
    ) {
        return -1;
    }

    unsafe {
        std::ptr::copy(
            decrypt_msg.as_ptr(),
            out_offset as *mut _,
            decrypt_msg.len(),
        )
    };

    decrypt_msg.len() as c_int
}
