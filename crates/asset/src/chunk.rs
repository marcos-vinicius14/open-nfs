//! Bounded, safe traversal of hierarchical EA chunk containers.

use crate::error::ChunkError;
use open_nfs_core::types::ChunkId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawChunk<'a> {
    pub id: ChunkId,
    pub size: u32,
    pub offset: usize,
    pub payload: &'a [u8],
}

impl<'a> RawChunk<'a> {
    #[inline]
    pub const fn is_container(&self) -> bool {
        (self.id.raw() & 0x8000_0000) != 0
    }

    #[inline]
    pub fn iter_subchunks(&self) -> ChunkIterator<'a> {
        ChunkIterator::new(self.payload, self.offset + 8)
    }
}

#[derive(Debug, Clone)]
pub struct ChunkIterator<'a> {
    buffer: &'a [u8],
    cursor: usize,
    base_offset: usize,
}

impl<'a> ChunkIterator<'a> {
    pub fn new(buffer: &'a [u8], base_offset: usize) -> Self {
        Self {
            buffer,
            cursor: 0,
            base_offset,
        }
    }

    pub fn next_chunk(&mut self) -> Result<Option<RawChunk<'a>>, ChunkError> {
        while self.cursor < self.buffer.len() {
            let remaining_slice = &self.buffer[self.cursor..];
            if remaining_slice.len() < 8 {
                if remaining_slice.iter().all(|&b| b == 0x11 || b == 0x00) {
                    self.cursor = self.buffer.len();
                    return Ok(None);
                } else {
                    return Err(ChunkError::UnexpectedEof {
                        offset: self.base_offset + self.cursor,
                        expected: 8,
                        available: remaining_slice.len(),
                    });
                }
            }

            let id_bytes: [u8; 4] =
                remaining_slice[0..4]
                    .try_into()
                    .map_err(|_| ChunkError::UnexpectedEof {
                        offset: self.base_offset + self.cursor,
                        expected: 4,
                        available: remaining_slice.len(),
                    })?;
            let size_bytes: [u8; 4] =
                remaining_slice[4..8]
                    .try_into()
                    .map_err(|_| ChunkError::UnexpectedEof {
                        offset: self.base_offset + self.cursor + 4,
                        expected: 4,
                        available: remaining_slice.len() - 4,
                    })?;

            let raw_id = u32::from_le_bytes(id_bytes);
            let size = u32::from_le_bytes(size_bytes);

            if raw_id == 0 && size == 0 {
                self.cursor += 8;
                continue;
            }

            if raw_id == 0x1111_1111 {
                self.cursor += 4;
                continue;
            }

            let size_usize = size as usize;
            let payload_start = self.cursor + 8;
            let payload_end =
                payload_start
                    .checked_add(size_usize)
                    .ok_or(ChunkError::IntegerOverflow {
                        context: "chunk payload end calculation",
                    })?;

            if payload_end > self.buffer.len() {
                if raw_id == 0 {
                    self.cursor += 4;
                    continue;
                }
                return Err(ChunkError::InvalidChunkSize {
                    size: size_usize,
                    remaining: self.buffer.len() - payload_start,
                });
            }

            let chunk_offset = self.base_offset + self.cursor;
            let payload = &self.buffer[payload_start..payload_end];
            self.cursor = payload_end;

            return Ok(Some(RawChunk {
                id: ChunkId::new(raw_id),
                size,
                offset: chunk_offset,
                payload,
            }));
        }

        Ok(None)
    }
}
