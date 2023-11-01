use std::io::Read;

use cryptoxide::chacha20poly1305::ChaCha20Poly1305;

fn main() {
    let mut file = std::fs::File::open("Cargo.lock").unwrap();

    // read file to stirng
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();

    let start = std::time::Instant::now();

    let key = [0u8; 16];
    let nonce: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let aad: [u8; 0] = [];
    let input = b"hello world!";
    let mut out = [0u8; 12 + 16];
    let mut tag = [0u8; 16];

    // create a new cipher
    let mut cipher = ChaCha20Poly1305::new(&key, &nonce, &aad);

    // encrypt the msg and append the tag at the end
    cipher.encrypt(input, &mut out[0..12], &mut tag);
    out[12..].copy_from_slice(&tag);

    println!("out: {:?}", out);

    let mut output = [0u8; 12];
    let mut cipher = ChaCha20Poly1305::new(&key, &nonce, &aad);

    if !cipher.decrypt(&out[0..12], &mut output, &out[12..]) {
        panic!("decryption failed");
    }

    println!("output: {:?}", String::from_utf8(output.to_vec()).unwrap());
    println!("time: {:?}", start.elapsed());
}
