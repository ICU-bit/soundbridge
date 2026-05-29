use std::sync::atomic::{AtomicUsize, Ordering};

/// Lock-free SPSC (Single Producer Single Consumer) ring buffer.
///
/// 用于音频采集线程和处理线程之间的数据传递。
/// 缓冲区大小必须是 2 的幂，以便用位运算取模。
pub struct RingBuffer<T: Copy + Default> {
    buffer: Vec<T>,
    read_pos: AtomicUsize,
    write_pos: AtomicUsize,
    capacity: usize,
    mask: usize,
}

impl<T: Copy + Default> RingBuffer<T> {
    /// 创建新的 ring buffer。
    ///
    /// # Arguments
    /// * `capacity` - 缓冲区大小，必须是 2 的幂。如果不是，会自动向上取整。
    ///
    /// # Examples
    /// ```
    /// use audio_core::RingBuffer;
    ///
    /// let rb = RingBuffer::<f32>::new(1024);
    /// assert_eq!(rb.capacity(), 1024);
    /// ```
    pub fn new(capacity: usize) -> Self {
        // 确保容量是 2 的幂
        let capacity = capacity.next_power_of_two();
        let mask = capacity - 1;

        let mut buffer = Vec::with_capacity(capacity);
        buffer.resize_with(capacity, T::default);

        Self {
            buffer,
            read_pos: AtomicUsize::new(0),
            write_pos: AtomicUsize::new(0),
            capacity,
            mask,
        }
    }

    /// 获取缓冲区容量。
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 写入数据到 ring buffer。
    ///
    /// 返回实际写入的数据量。如果缓冲区已满，可能写入部分数据。
    ///
    /// # Arguments
    /// * `data` - 要写入的数据
    ///
    /// # Returns
    /// 实际写入的数据量
    pub fn write(&self, data: &[T]) -> usize {
        let write_pos = self.write_pos.load(Ordering::Relaxed);
        let read_pos = self.read_pos.load(Ordering::Acquire);

        // 计算可用空间
        let available = self.available_write_impl(write_pos, read_pos);
        let to_write = data.len().min(available);

        if to_write == 0 {
            return 0;
        }

        // 写入数据
        let write_idx = write_pos & self.mask;
        let first_chunk = (self.capacity - write_idx).min(to_write);
        let second_chunk = to_write - first_chunk;

        // 安全写入：使用 unsafe 来避免边界检查
        unsafe {
            let dst = self.buffer.as_ptr().add(write_idx) as *mut T;
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst, first_chunk);

            if second_chunk > 0 {
                let dst = self.buffer.as_ptr() as *mut T;
                std::ptr::copy_nonoverlapping(
                    data.as_ptr().add(first_chunk),
                    dst,
                    second_chunk,
                );
            }
        }

        // 更新写位置
        self.write_pos.store(write_pos + to_write, Ordering::Release);

        to_write
    }

    /// 从 ring buffer 读取数据。
    ///
    /// 返回实际读取的数据量。如果缓冲区为空，可能读取部分数据。
    ///
    /// # Arguments
    /// * `output` - 读取数据的目标缓冲区
    ///
    /// # Returns
    /// 实际读取的数据量
    pub fn read(&self, output: &mut [T]) -> usize {
        let read_pos = self.read_pos.load(Ordering::Relaxed);
        let write_pos = self.write_pos.load(Ordering::Acquire);

        // 计算可读数据量
        let available = self.available_read_impl(read_pos, write_pos);
        let to_read = output.len().min(available);

        if to_read == 0 {
            return 0;
        }

        // 读取数据
        let read_idx = read_pos & self.mask;
        let first_chunk = (self.capacity - read_idx).min(to_read);
        let second_chunk = to_read - first_chunk;

        // 安全读取
        unsafe {
            let src = self.buffer.as_ptr().add(read_idx);
            std::ptr::copy_nonoverlapping(src, output.as_mut_ptr(), first_chunk);

            if second_chunk > 0 {
                let src = self.buffer.as_ptr();
                std::ptr::copy_nonoverlapping(
                    src,
                    output.as_mut_ptr().add(first_chunk),
                    second_chunk,
                );
            }
        }

        // 更新读位置
        self.read_pos.store(read_pos + to_read, Ordering::Release);

        to_read
    }

    /// 获取可读数据量。
    pub fn available_read(&self) -> usize {
        let read_pos = self.read_pos.load(Ordering::Relaxed);
        let write_pos = self.write_pos.load(Ordering::Acquire);
        self.available_read_impl(read_pos, write_pos)
    }

    /// 获取可写空间。
    pub fn available_write(&self) -> usize {
        let write_pos = self.write_pos.load(Ordering::Relaxed);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        self.available_write_impl(write_pos, read_pos)
    }

    /// 清空缓冲区。
    pub fn clear(&self) {
        self.read_pos.store(0, Ordering::Release);
        self.write_pos.store(0, Ordering::Release);
    }

    /// 检查缓冲区是否为空。
    pub fn is_empty(&self) -> bool {
        self.available_read() == 0
    }

    /// 检查缓冲区是否已满。
    pub fn is_full(&self) -> bool {
        self.available_write() == 0
    }

    // 内部辅助函数
    fn available_read_impl(&self, read_pos: usize, write_pos: usize) -> usize {
        write_pos - read_pos
    }

    fn available_write_impl(&self, write_pos: usize, read_pos: usize) -> usize {
        self.capacity - (write_pos - read_pos)
    }
}

// 线程安全：RingBuffer 可以在线程间共享
unsafe impl<T: Copy + Default> Send for RingBuffer<T> {}
unsafe impl<T: Copy + Default> Sync for RingBuffer<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_write_read() {
        let rb = RingBuffer::<f32>::new(16);
        let data = [1.0, 2.0, 3.0, 4.0];

        // 写入
        let written = rb.write(&data);
        assert_eq!(written, 4);
        assert_eq!(rb.available_read(), 4);

        // 读取
        let mut output = [0.0f32; 4];
        let read = rb.read(&mut output);
        assert_eq!(read, 4);
        assert_eq!(output, data);
        assert_eq!(rb.available_read(), 0);
    }

    #[test]
    fn test_buffer_full() {
        let rb = RingBuffer::<f32>::new(4);
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];

        // 写入超过容量
        let written = rb.write(&data);
        assert_eq!(written, 4); // 只能写入 4 个
        assert!(rb.is_full());
    }

    #[test]
    fn test_buffer_empty() {
        let rb = RingBuffer::<f32>::new(16);
        let mut output = [0.0f32; 4];

        // 从空缓冲区读取
        let read = rb.read(&mut output);
        assert_eq!(read, 0);
        assert!(rb.is_empty());
    }

    #[test]
    fn test_wrap_around() {
        let rb = RingBuffer::<f32>::new(4);

        // 写入 4 个
        rb.write(&[1.0, 2.0, 3.0, 4.0]);

        // 读取 2 个
        let mut output = [0.0f32; 2];
        rb.read(&mut output);
        assert_eq!(output, [1.0, 2.0]);

        // 再写入 2 个（绕回，只有 2 个空间）
        rb.write(&[5.0, 6.0]);

        // 读取所有
        let mut output = [0.0f32; 4];
        let read = rb.read(&mut output);
        assert_eq!(read, 4);
        assert_eq!(output, [3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_capacity_power_of_two() {
        let rb = RingBuffer::<f32>::new(10);
        assert_eq!(rb.capacity(), 16); // 向上取整到 16
    }

    #[test]
    fn test_clear() {
        let rb = RingBuffer::<f32>::new(16);
        rb.write(&[1.0, 2.0, 3.0]);

        assert_eq!(rb.available_read(), 3);

        rb.clear();
        assert_eq!(rb.available_read(), 0);
        assert!(rb.is_empty());
    }

    #[test]
    fn test_available_write() {
        let rb = RingBuffer::<f32>::new(8);

        assert_eq!(rb.available_write(), 8);

        rb.write(&[1.0, 2.0, 3.0]);
        assert_eq!(rb.available_write(), 5);

        rb.write(&[4.0, 5.0]);
        assert_eq!(rb.available_write(), 3);
    }
}
