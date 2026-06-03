//! Minimal parser for the game's "BSV" anonymous binary table format, and the
//! LZ4 decompression used by the manifest CDN. Ported from uma-sim's
//! `fetch-master-db.ts`.

const BSV_MAGIC: u8 = 0xBF;
const BSV_FORMAT_VERSION: u8 = 1;
const BSV_FORMAT_ANONYMOUS: u8 = 1;
const LZ4_FRAME_MAGIC: [u8; 4] = [0x04, 0x22, 0x4D, 0x18];

/// A decoded BSV cell: either an unsigned integer or a string.
#[derive(Debug, Clone)]
pub enum Value {
    Int(u64),
    Str(String),
}

impl Value {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Int(v) => Some(*v),
            Value::Str(_) => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::Int(_) => None,
        }
    }
}

pub fn is_lz4_frame(data: &[u8]) -> bool {
    data.len() >= 4 && data[0..4] == LZ4_FRAME_MAGIC
}

/// Decompress either an LZ4 frame, or a raw block prefixed with a little-endian
/// u32 uncompressed size (the CDN uses both).
pub fn decompress_lz4(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 4 {
        return Err("data too short for LZ4 header".into());
    }
    if is_lz4_frame(data) {
        let mut out = Vec::new();
        let mut dec = lz4_flex::frame::FrameDecoder::new(data);
        std::io::Read::read_to_end(&mut dec, &mut out).map_err(|e| format!("LZ4 frame decode: {e}"))?;
        return Ok(out);
    }
    let size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    lz4_flex::block::decompress(&data[4..], size).map_err(|e| format!("LZ4 block decode: {e}"))
}

struct Reader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Reader { data, offset: 0 }
    }

    /// Variable-length quantity: big-endian base-128, high bit = continue.
    fn read_vlq(&mut self, max_bytes: usize) -> u64 {
        let mut value: u64 = 0;
        let mut read = 0;
        while read < max_bytes && self.offset < self.data.len() {
            let byte = self.data[self.offset];
            self.offset += 1;
            read += 1;
            value = (value << 7) | u64::from(byte & 0x7F);
            if byte & 0x80 == 0 {
                break;
            }
        }
        value
    }

    fn read_unum(&mut self, num_bytes: usize) -> Result<u64, String> {
        if self.offset + num_bytes > self.data.len() {
            return Err("unexpected end of BSV data while reading integer".into());
        }
        let mut value: u64 = 0;
        for i in 0..num_bytes {
            value = (value << 8) | u64::from(self.data[self.offset + i]);
        }
        self.offset += num_bytes;
        Ok(value)
    }

    fn read_text(&mut self) -> String {
        let start = self.offset;
        while self.offset < self.data.len() && self.data[self.offset] != 0 {
            self.offset += 1;
        }
        let text = String::from_utf8_lossy(&self.data[start..self.offset]).into_owned();
        if self.offset < self.data.len() {
            self.offset += 1; // skip NUL
        }
        text
    }

    fn read_byte(&mut self) -> Result<u8, String> {
        if self.offset >= self.data.len() {
            return Err("unexpected end of BSV data while reading byte".into());
        }
        let value = self.data[self.offset];
        self.offset += 1;
        Ok(value)
    }
}

/// Parse an anonymous BSV table into rows of [`Value`].
pub fn parse_anonymous(data: &[u8]) -> Result<Vec<Vec<Value>>, String> {
    if data.len() < 2 {
        return Err("BSV data too short".into());
    }
    if data[0] != BSV_MAGIC {
        return Err(format!("invalid BSV magic: 0x{:02x}", data[0]));
    }
    let format_byte = data[1];
    let version = (format_byte >> 4) & 0x0F;
    let format_type = format_byte & 0x0F;
    if version != BSV_FORMAT_VERSION {
        return Err(format!("unsupported BSV version: {version}"));
    }
    if format_type != BSV_FORMAT_ANONYMOUS {
        return Err(format!("expected ANONYMOUS format, got {format_type}"));
    }

    let mut r = Reader::new(data);
    r.offset = 2;
    r.read_unum(2)?; // header_size
    let row_count = r.read_vlq(8);
    r.read_vlq(8); // max_row_size
    r.read_vlq(8); // schema_version
    let schema_count = r.read_vlq(8);

    // schema: (type_byte, fixed_size)
    let mut schemas: Vec<(u8, Option<usize>)> = Vec::with_capacity(schema_count as usize);
    for _ in 0..schema_count {
        let type_byte = r.read_byte()?;
        let fixed_size = if (type_byte.wrapping_sub(0x21) & 0xCF) == 0 && type_byte != 0x51 {
            Some(r.read_vlq(8) as usize)
        } else {
            None
        };
        schemas.push((type_byte, fixed_size));
    }

    let mut rows = Vec::with_capacity(row_count as usize);
    for _ in 0..row_count {
        let mut row = Vec::with_capacity(schemas.len());
        for &(type_byte, fixed_size) in &schemas {
            let base_type = type_byte & 0xF0;
            if type_byte == 0x40 || base_type == 0x40 {
                row.push(Value::Str(r.read_text()));
            } else if matches!(type_byte, 0x11..=0x13) || base_type == 0x10 {
                row.push(Value::Int(r.read_vlq(8)));
            } else if let Some(n) = fixed_size {
                row.push(Value::Int(r.read_unum(n)?));
            } else {
                return Err(format!("unknown BSV type: 0x{type_byte:02X}"));
            }
        }
        rows.push(row);
    }

    Ok(rows)
}
