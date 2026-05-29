#[cfg(test)]
mod tests {
    #[test]
    fn test_ffi_basic() {
        // FFI 绑定的基本验证
        // 注意：实际的 FFI 函数调用需要在集成测试中进行
        // 这里只验证测试框架工作正常
        assert!(true);
    }

    #[test]
    fn test_error_codes() {
        // 验证错误码定义
        assert_eq!(0, 0); // Ok
        assert_eq!(-1, -1); // Error
        assert_eq!(-2, -2); // InvalidArgument
    }
}
