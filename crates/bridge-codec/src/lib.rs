//! MVP screen codec.
//!
//! Encoder: compares the new frame with the previous one in 64×64 tiles; for every tile row with
//! changes, the span from the first to the last dirty tile is JPEG-encoded as one band.
//! Static screens cost nothing; typing costs a few KB per frame.
//!
//! Decoder: validates every rectangle against the frame bounds before decoding (SEC: M-53 —
//! the viewer treats the host's stream as untrusted input).
//!
//! Hardware H.264/HEVC/AV1 replaces this in a later phase (see `docs/02 - Architecture/Media Pipeline.md`).
#![forbid(unsafe_code)]

use bridge_proto::msg::{FrameUpdate, Tile};
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;

pub const TILE: usize = 64;
pub const MAX_DIM: u32 = 16_384;
pub const MAX_TILES_PER_UPDATE: usize = 4_096;

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("frame buffer size does not match {0}x{1}")]
    BadInput(u32, u32),
    #[error("invalid frame dimensions {0}x{1}")]
    BadDimensions(u32, u32),
    #[error("tile out of bounds")]
    TileOutOfBounds,
    #[error("too many tiles")]
    TooManyTiles,
    #[error("jpeg encode: {0}")]
    Encode(String),
    #[error("jpeg decode: {0}")]
    Decode(String),
}

pub struct TileEncoder {
    prev: Vec<u8>,
    width: u32,
    height: u32,
    quality: u8,
}

impl TileEncoder {
    pub fn new(quality: u8) -> Self {
        Self { prev: Vec::new(), width: 0, height: 0, quality: quality.clamp(10, 95) }
    }

    pub fn set_quality(&mut self, quality: u8) {
        self.quality = quality.clamp(10, 95);
    }

    /// Next `encode` sends the whole frame.
    pub fn force_keyframe(&mut self) {
        self.prev.clear();
    }

    /// Encode changes since the previous call. `bgra` is top-down, 4 bytes per pixel.
    pub fn encode(&mut self, bgra: &[u8], width: u32, height: u32) -> Result<Option<FrameUpdate>, CodecError> {
        if width == 0 || height == 0 || width > MAX_DIM || height > MAX_DIM {
            return Err(CodecError::BadDimensions(width, height));
        }
        let (w, h) = (width as usize, height as usize);
        if bgra.len() != w * h * 4 {
            return Err(CodecError::BadInput(width, height));
        }
        let full = self.width != width || self.height != height || self.prev.len() != bgra.len();
        let stride = w * 4;
        let tiles_x = w.div_ceil(TILE);
        let tiles_y = h.div_ceil(TILE);
        let mut tiles = Vec::new();

        for ty in 0..tiles_y {
            let y0 = ty * TILE;
            let th = TILE.min(h - y0);
            let mut span: Option<(usize, usize)> = None;
            for tx in 0..tiles_x {
                let x0 = tx * TILE;
                let tw = TILE.min(w - x0);
                if full || region_changed(&self.prev, bgra, stride, x0, y0, tw, th) {
                    span = Some(match span {
                        None => (tx, tx),
                        Some((a, _)) => (a, tx),
                    });
                }
            }
            if let Some((a, b)) = span {
                let x0 = a * TILE;
                let x1 = ((b + 1) * TILE).min(w);
                let jpeg = encode_rect(bgra, stride, x0, y0, x1 - x0, th, self.quality)?;
                tiles.push(Tile { x: x0 as u32, y: y0 as u32, width: (x1 - x0) as u32, height: th as u32, jpeg });
            }
        }

        if full {
            self.prev = bgra.to_vec();
        } else {
            self.prev.copy_from_slice(bgra);
        }
        self.width = width;
        self.height = height;

        Ok(if tiles.is_empty() { None } else { Some(FrameUpdate { width, height, tiles }) })
    }
}

fn region_changed(prev: &[u8], cur: &[u8], stride: usize, x0: usize, y0: usize, w: usize, h: usize) -> bool {
    (y0..y0 + h).any(|row| {
        let s = row * stride + x0 * 4;
        prev[s..s + w * 4] != cur[s..s + w * 4]
    })
}

fn encode_rect(bgra: &[u8], stride: usize, x: usize, y: usize, w: usize, h: usize, quality: u8) -> Result<Vec<u8>, CodecError> {
    let mut rect = Vec::with_capacity(w * h * 4);
    for row in y..y + h {
        let s = row * stride + x * 4;
        rect.extend_from_slice(&bgra[s..s + w * 4]);
    }
    let mut out = Vec::with_capacity(w * h / 4);
    let encoder = jpeg_encoder::Encoder::new(&mut out, quality);
    encoder
        .encode(&rect, w as u16, h as u16, jpeg_encoder::ColorType::Bgra)
        .map_err(|e| CodecError::Encode(e.to_string()))?;
    Ok(out)
}

/// Viewer-side framebuffer in `0x00RRGGBB` (softbuffer format).
#[derive(Default)]
pub struct FrameDecoder {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn apply(&mut self, update: &FrameUpdate) -> Result<(), CodecError> {
        let (fw, fh) = (update.width, update.height);
        if fw == 0 || fh == 0 || fw > MAX_DIM || fh > MAX_DIM {
            return Err(CodecError::BadDimensions(fw, fh));
        }
        if update.tiles.len() > MAX_TILES_PER_UPDATE {
            return Err(CodecError::TooManyTiles);
        }
        if fw != self.width || fh != self.height {
            self.width = fw;
            self.height = fh;
            self.pixels = vec![0; fw as usize * fh as usize];
        }
        for tile in &update.tiles {
            self.apply_tile(tile)?;
        }
        Ok(())
    }

    fn apply_tile(&mut self, t: &Tile) -> Result<(), CodecError> {
        let fits = |pos: u32, len: u32, max: u32| len > 0 && pos.checked_add(len).is_some_and(|end| end <= max);
        if !fits(t.x, t.width, self.width) || !fits(t.y, t.height, self.height) {
            return Err(CodecError::TileOutOfBounds);
        }
        let (tw, th) = (t.width as usize, t.height as usize);
        let opts = DecoderOptions::default()
            .jpeg_set_out_colorspace(ColorSpace::RGB)
            .set_max_width(tw)
            .set_max_height(th);
        let mut dec = zune_jpeg::JpegDecoder::new_with_options(&t.jpeg, opts);
        let rgb = dec.decode().map_err(|e| CodecError::Decode(format!("{e:?}")))?;
        let (dw, dh) = dec.dimensions().ok_or_else(|| CodecError::Decode("no dimensions".into()))?;
        if dw != tw || dh != th || rgb.len() != tw * th * 3 {
            return Err(CodecError::Decode("tile size mismatch".into()));
        }
        let fw = self.width as usize;
        for row in 0..th {
            let dst = &mut self.pixels[(t.y as usize + row) * fw + t.x as usize..][..tw];
            let src = &rgb[row * tw * 3..][..tw * 3];
            for (d, s) in dst.iter_mut().zip(src.as_chunks::<3>().0) {
                *d = (u32::from(s[0]) << 16) | (u32::from(s[1]) << 8) | u32::from(s[2]);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: usize, h: usize, color: [u8; 4]) -> Vec<u8> {
        color.iter().copied().cycle().take(w * h * 4).collect()
    }

    #[test]
    fn full_then_incremental() {
        let (w, h) = (200, 130);
        let mut enc = TileEncoder::new(80);
        let mut dec = FrameDecoder::new();

        // BGRA: pure red
        let f1 = frame(w, h, [0, 0, 255, 0]);
        let u1 = enc.encode(&f1, w as u32, h as u32).unwrap().unwrap();
        assert_eq!(u1.tiles.len(), 3, "one band per tile row");
        dec.apply(&u1).unwrap();
        let p = dec.pixels()[w * 10 + 10];
        assert!((p >> 16) & 0xFF > 230 && (p >> 8) & 0xFF < 30, "red, got {p:06x}");

        assert!(enc.encode(&f1, w as u32, h as u32).unwrap().is_none(), "no changes → nothing sent");

        let mut f2 = f1.clone();
        let off = (70 * w + 150) * 4;
        f2[off..off + 4].copy_from_slice(&[255, 0, 0, 0]);
        let u2 = enc.encode(&f2, w as u32, h as u32).unwrap().unwrap();
        assert_eq!(u2.tiles.len(), 1);
        assert_eq!((u2.tiles[0].x, u2.tiles[0].y, u2.tiles[0].width), (128, 64, 64));
        dec.apply(&u2).unwrap();
    }

    #[test]
    fn decoder_rejects_out_of_bounds_tiles() {
        let mut enc = TileEncoder::new(50);
        let f = frame(64, 64, [1, 2, 3, 0]);
        let mut u = enc.encode(&f, 64, 64).unwrap().unwrap();
        u.tiles[0].x = u32::MAX - 10;
        assert!(matches!(FrameDecoder::new().apply(&u), Err(CodecError::TileOutOfBounds)));
    }

    #[test]
    fn decoder_rejects_lying_tile_size() {
        let mut enc = TileEncoder::new(50);
        let f = frame(128, 64, [1, 2, 3, 0]);
        let mut u = enc.encode(&f, 128, 64).unwrap().unwrap();
        u.tiles[0].width = 64; // JPEG is really 128 wide
        assert!(FrameDecoder::new().apply(&u).is_err());
    }

    #[test]
    fn decoder_rejects_garbage_and_huge_frames() {
        let bad = FrameUpdate { width: 10, height: 10, tiles: vec![Tile { x: 0, y: 0, width: 10, height: 10, jpeg: vec![0xFF, 0xD8, 1, 2, 3] }] };
        assert!(FrameDecoder::new().apply(&bad).is_err());
        let huge = FrameUpdate { width: 100_000, height: 10, tiles: vec![] };
        assert!(FrameDecoder::new().apply(&huge).is_err());
    }
}
