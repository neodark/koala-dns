pub trait DirectAccessBuf {
    fn pos(&self) -> usize;
    fn set_pos(&mut self, pos: usize);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn seek(&mut self, pos: usize) -> bool {
        if pos > self.len() {
            return false;
        }
        self.set_pos(pos);
        true
    }
    fn advance(&mut self, count: usize) -> bool {
        let new_pos = self.pos() + count;
        self.seek(new_pos)
    }
    fn reset(&mut self) {
        self.seek(0);
    }
}

pub trait BufRead : DirectAccessBuf {
    fn buf(&self) -> &[u8];

    fn peek_u8(&self) -> Option<u8> {
        if self.pos() >= self.len() {
            return None;
        }
        Some(self.buf()[self.pos()])
    }

    fn next_bytes(&mut self, bytes: usize) -> Vec<u8> {
        let mut slice = Vec::with_capacity(bytes);
        for _ in 0..bytes {
            let byte = self.next_u8();
            match byte {
                Some(b) => slice.push(b),
                None => break,
            }
        }
        slice
    }

    fn next_u8(&mut self) -> Option<u8> {
        match self.peek_u8() {
            Some(byte) => {
                self.advance(1);
                Some(byte)
            }
            None => None,
        }
    }

    ///Return the next u16 IFF there are two bytes to read. If there is only one, None is returned
    ///and pos is not changed
    ///Callers should check and call next_u8 if required
    fn next_u16(&mut self) -> Option<u16> {
        let len = self.len();
        if self.pos() + 1 >= len {
            return None;
        }
        let byte1 = self.buf()[self.pos()];
        let byte2 = self.buf()[self.pos() + 1];
        self.advance(2);
        Some(((byte1 as u16) << 8) | byte2 as u16)
    }

    fn next_u32(&mut self) -> Option<u32> {
        let len = self.len();
        if (self.pos() + 3) >= len {
            return None;
        }

        let val = (self.buf()[self.pos()] as u32) << 24 |
                  (self.buf()[self.pos() + 1] as u32) << 16 |
                  (self.buf()[self.pos() + 2] as u32) << 8 |
                  self.buf()[self.pos() + 3] as u32;
        self.advance(4);
        Some(val)
    }
}


pub trait BufWrite : BufRead {
    fn buf(&mut self) -> &mut [u8];

    fn write_u8(&mut self, byte: u8) -> bool {
        if self.pos() >= self.len() {
            return false;
        }
        self.buf()[self.pos()] = byte;
        self.advance(1);
        true
    }

    fn write_u16(&mut self, bytes: u16) -> bool {
        if self.pos() + 1 >= self.len() {            
            return false;
        }

        let pos = self.pos();
        // as takes last (rightmost) bits
        self.buf()[pos] = (bytes >> 8) as u8;
        self.buf()[pos + 1] = bytes as u8;
        self.advance(2);
        true
    }

    fn write_u32(&mut self, bytes: u32) -> bool {
        if self.pos() + 3 >= self.len() {
            return false;
        }
        let pos = self.pos();
        self.buf()[pos] = (bytes >> 24) as u8;
        self.buf()[pos + 1] = (bytes >> 16) as u8;
        self.buf()[pos + 2] = (bytes >> 8) as u8;
        self.buf()[pos + 3] = bytes as u8;
        self.advance(4);
        true
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> bool {
        if self.pos() + bytes.len() > self.len() {
            return false;
        }
        for byte in bytes {
            self.write_u8(*byte);
        }
        true
    }
}