pub mod ffi;

pub use ffi::*;

pub mod ffi {
    use audio_core::Result;

    #[no_mangle]
    pub extern "C" fn soundbridge_init() -> i32 {
        0
    }

    #[no_mangle]
    pub extern "C" fn soundbridge_cleanup() {
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn it_works() {
            let result = 2 + 2;
            assert_eq!(result, 4);
        }
    }
}
