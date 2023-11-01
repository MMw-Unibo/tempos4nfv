use core::slice::*;

#[derive(Default)]
pub struct Buffer {
    pub buf: Vec<u8>,
    start: usize,
    end: usize,
    current: usize,
}

impl Buffer {
    pub fn copy_bytes(&self, into: &mut [u8], start: usize) -> usize {
        let len = into.len();
        into[0..len].copy_from_slice(&self.buf[start..start + len]);
        1
    }

    // pub fn read(&mut self, into: &mut [u8], n: usize) -> usize {
    //     self.copy_bytes(into, self.current, n);
    // }

    // pub fn read_byte(&mut self) -> Option<u8> {
    //     let mut byte = 0;
    //     if self.read(core::slice::from_mut(&mut byte), 1) != 0 {
    //         Some(byte)
    //     } else {
    //         Nonex
    //     }
    // }
}

impl From<Vec<u8>> for Buffer {
    fn from(buf: Vec<u8>) -> Self {
        let end = buf.len();
        Self {
            buf: buf,
            end,
            ..Default::default()
        }
    }
}

pub struct BufferPool {
    buffers: Vec<Buffer>,
}

impl BufferPool {
    pub fn new() -> Self {
        Self { buffers: vec![] }
    }

    pub fn get(&mut self) -> Buffer {
        if self.buffers.is_empty() {
            Buffer::default()
        } else {
            self.buffers.pop().unwrap()
        }
    }

    pub fn put(&mut self, buf: Buffer) {
        self.buffers.push(buf);
    }
}
