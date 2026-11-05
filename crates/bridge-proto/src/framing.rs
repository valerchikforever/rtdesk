//! Length-prefixed (u32 LE) protobuf framing with per-channel size limits.
//! SEC: M-42 — every read is bounded before allocation.

use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Control stream (auth, input, clipboard).
pub const MAX_CONTROL: usize = 2 * 1024 * 1024;
/// Clipboard text payload.
pub const MAX_CLIPBOARD_TEXT: usize = 1024 * 1024;
/// One video frame update.
pub const MAX_FRAME: usize = 32 * 1024 * 1024;
/// Rendezvous messages.
pub const MAX_RENDEZVOUS: usize = 16 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("message too large: {len} > {max}")]
    TooLarge { len: usize, max: usize },
    #[error("malformed message: {0}")]
    Decode(#[from] prost::DecodeError),
}

pub async fn write_msg<W, M>(w: &mut W, msg: &M) -> Result<(), FrameError>
where
    W: AsyncWrite + Unpin,
    M: Message,
{
    let len = msg.encoded_len();
    let len32 = u32::try_from(len).map_err(|_| FrameError::TooLarge { len, max: u32::MAX as usize })?;
    let mut buf = Vec::with_capacity(4 + len);
    buf.extend_from_slice(&len32.to_le_bytes());
    msg.encode_raw(&mut buf);
    w.write_all(&buf).await?;
    Ok(())
}

pub async fn read_msg<R, M>(r: &mut R, max: usize) -> Result<M, FrameError>
where
    R: AsyncRead + Unpin,
    M: Message + Default,
{
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > max {
        return Err(FrameError::TooLarge { len, max });
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;
    Ok(M::decode(buf.as_slice())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{Bye, Control, control::Kind};

    #[tokio::test]
    async fn roundtrip_and_limit() {
        let msg = Control::new(Kind::Bye(Bye { reason: "x".repeat(100) }));
        let mut buf = Vec::new();
        write_msg(&mut buf, &msg).await.unwrap();

        let back: Control = read_msg(&mut buf.as_slice(), 1024).await.unwrap();
        assert_eq!(back, msg);

        let err = read_msg::<_, Control>(&mut buf.as_slice(), 10).await.unwrap_err();
        assert!(matches!(err, FrameError::TooLarge { .. }));
    }

    #[tokio::test]
    async fn huge_length_prefix_is_rejected_without_allocating() {
        let bytes = u32::MAX.to_le_bytes();
        let err = read_msg::<_, Control>(&mut &bytes[..], MAX_CONTROL).await.unwrap_err();
        assert!(matches!(err, FrameError::TooLarge { .. }));
    }
}
