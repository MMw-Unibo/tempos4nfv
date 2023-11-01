use crate::buffer::{self, Buffer};

pub struct TemposHeader {
    pub msg_type: u8,
    pub timestamp: i64,
}

pub struct TemposMessage<'a> {
    pub header: TemposHeader,
    buffer: &'a Buffer,
}

// impl<'a> TemposMessage<'a> {
//     pub fn new(buffer: &Buffer) -> Self {
//         TemposMessage { buffer, header: () }
//     }
// }
