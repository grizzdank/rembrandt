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
    ///
    /// # TODO: Implement this method
    ///
    /// Consider:
    /// - What happens when data.len() > self.capacity? (larger than entire buffer)
    /// - How do you handle wraparound at the end of the buffer?
    /// - Should you update write_pos and total_written atomically?
    pub fn write(&mut self, data: &[u8]) {
        // YOUR IMPLEMENTATION HERE
        //
        // Hints:
        // 1. If data is larger than capacity, only keep the last `capacity` bytes
        // 2. Handle the case where write wraps around the end of the buffer
        // 3. Update write_pos and total_written appropriately
        //
        // Example approach:
        // - If buffer isn't full yet (data.len() < capacity), just extend
        // - If buffer is full, overwrite starting at write_pos
        // - Handle wraparound when write_pos + data.len() > capacity

        todo!("Implement ring buffer write logic")
    }

    /// Read all available data from the buffer
    ///
    /// Returns data in chronological order (oldest first).
    ///
    /// # TODO: Implement this method
    ///
    /// Consider:
    /// - If buffer hasn't wrapped, data is contiguous from 0..write_pos
    /// - If buffer has wrapped, need to return write_pos..end + 0..write_pos
    pub fn read_all(&self) -> Vec<u8> {
        // YOUR IMPLEMENTATION HERE
        //
        // Hints:
        // 1. Check if buffer has wrapped (total_written > capacity)
        // 2. If not wrapped: return data[0..write_pos]
        // 3. If wrapped: return data[write_pos..] + data[0..write_pos]

        todo!("Implement ring buffer read logic")
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
