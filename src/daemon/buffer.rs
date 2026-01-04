//! Ring buffer for PTY output history
//!
//! The RingBuffer stores recent output from agent sessions, enabling
//! "late attach" - connecting to a session and seeing what happened
//! before you connected.

/// A fixed-capacity ring buffer for storing PTY output
///
/// When the buffer is full, old data is overwritten by new data.
/// This allows us to keep the last N bytes of output without
/// unbounded memory growth.
pub struct RingBuffer {
    /// The underlying storage
    data: Vec<u8>,
    /// Maximum capacity in bytes
    capacity: usize,
    /// Write position (where next byte goes)
    write_pos: usize,
    /// Total bytes written (may exceed capacity due to wraparound)
    total_written: usize,
}

impl RingBuffer {
    /// Create a new ring buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            capacity,
            write_pos: 0,
            total_written: 0,
        }
    }

    /// Write data to the buffer
    ///
    /// If the data exceeds remaining capacity, older data is overwritten.
    pub fn write(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        // If data is larger than capacity, only keep the last `capacity` bytes
        let data = if data.len() > self.capacity {
            &data[data.len() - self.capacity..]
        } else {
            data
        };

        // Ensure buffer is at full capacity for wraparound logic
        if self.data.len() < self.capacity {
            // Buffer not full yet - extend it
            let space_left = self.capacity - self.data.len();
            if data.len() <= space_left {
                // Fits entirely in remaining space
                self.data.extend_from_slice(data);
                self.write_pos = self.data.len();
            } else {
                // Partially fits, need to start wrapping
                self.data.extend_from_slice(&data[..space_left]);
                // Now buffer is full, write rest at beginning
                let remaining = &data[space_left..];
                self.data[..remaining.len()].copy_from_slice(remaining);
                self.write_pos = remaining.len();
            }
        } else {
            // Buffer is full, use wraparound
            let space_to_end = self.capacity - self.write_pos;
            if data.len() <= space_to_end {
                // Fits before end of buffer
                self.data[self.write_pos..self.write_pos + data.len()].copy_from_slice(data);
                self.write_pos += data.len();
                if self.write_pos >= self.capacity {
                    self.write_pos = 0;
                }
            } else {
                // Need to wrap around
                self.data[self.write_pos..].copy_from_slice(&data[..space_to_end]);
                let remaining = &data[space_to_end..];
                self.data[..remaining.len()].copy_from_slice(remaining);
                self.write_pos = remaining.len();
            }
        }

        self.total_written += data.len();
    }

    /// Read all available data from the buffer
    ///
    /// Returns data in chronological order (oldest first).
    pub fn read_all(&self) -> Vec<u8> {
        if self.data.is_empty() {
            return Vec::new();
        }

        if !self.has_wrapped() {
            // Buffer hasn't wrapped - data is contiguous from start
            self.data[..self.write_pos].to_vec()
        } else {
            // Buffer has wrapped - oldest data is at write_pos
            let mut result = Vec::with_capacity(self.capacity);
            result.extend_from_slice(&self.data[self.write_pos..]);
            result.extend_from_slice(&self.data[..self.write_pos]);
            result
        }
    }

    /// Get the number of bytes currently stored
    pub fn len(&self) -> usize {
        std::cmp::min(self.total_written, self.capacity)
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.total_written == 0
    }

    /// Check if buffer has wrapped around
    pub fn has_wrapped(&self) -> bool {
        self.total_written > self.capacity
    }

    /// Get the total bytes ever written (may exceed capacity)
    pub fn total_written(&self) -> usize {
        self.total_written
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.data.clear();
        self.write_pos = 0;
        self.total_written = 0;
    }

    /// Get current capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buf = RingBuffer::new(100);
        assert_eq!(buf.capacity(), 100);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_simple_write_read() {
        let mut buf = RingBuffer::new(100);
        buf.write(b"hello");
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.read_all(), b"hello");
    }

    #[test]
    fn test_multiple_writes() {
        let mut buf = RingBuffer::new(100);
        buf.write(b"hello ");
        buf.write(b"world");
        assert_eq!(buf.read_all(), b"hello world");
    }

    #[test]
    fn test_wraparound() {
        let mut buf = RingBuffer::new(10);
        buf.write(b"12345678"); // 8 bytes
        buf.write(b"abcd");     // 4 more, should wrap
        // Buffer should contain: "5678abcd" or similar
        // (oldest data overwritten)
        let result = buf.read_all();
        assert_eq!(result.len(), 10);
        assert!(buf.has_wrapped());
    }

    #[test]
    fn test_large_write() {
        let mut buf = RingBuffer::new(5);
        buf.write(b"this is way too long");
        // Should only keep last 5 bytes: " long"
        let result = buf.read_all();
        assert_eq!(result.len(), 5);
    }
}
