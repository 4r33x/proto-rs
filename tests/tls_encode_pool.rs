use proto_rs::EncodePoolConfig;
use proto_rs::EncodedSnapshot;
use proto_rs::ProtoEncode;
use proto_rs::configure_encode_pool;

// Separate integration-test process: startup configuration is intentionally
// immutable, so this test does not race other tests' first encoding operation.
#[test]
fn startup_settings_apply_to_every_thread_without_service_pool_handles() {
    let config = EncodePoolConfig {
        max_buffers: 2,
        max_buffer_capacity: 513,
    };
    configure_encode_pool(config).unwrap();
    assert_eq!(configure_encode_pool(config), Err(config));

    for _ in 0..2 {
        let (snapshot, bytes) = std::thread::spawn(|| {
            let value = vec![7u8; 8192];
            let first = EncodedSnapshot::new(&value);
            let first_pointer = first.as_bytes().as_ptr();
            let retained = first.clone().into_bytes();
            drop(first);
            let second = EncodedSnapshot::new(&value);
            let second_pointer = second.as_bytes().as_ptr();
            // Both slots are leased: temporary outputs must not alias either.
            for _ in 0..8 {
                let overflow = EncodedSnapshot::new(&value);
                assert_ne!(overflow.as_bytes().as_ptr(), first_pointer);
                assert_ne!(overflow.as_bytes().as_ptr(), second_pointer);
                assert_eq!(overflow.as_bytes(), value.encode_to_vec());
            }
            std::thread::spawn(move || drop(retained)).join().unwrap();
            // Returned 513-byte slot must safely grow for a larger output.
            let reused = EncodedSnapshot::new(&vec![9u8; 4096]);
            assert_eq!(reused.as_bytes(), vec![9u8; 4096].encode_to_vec());
            (second, reused.into_bytes())
        })
        .join()
        .unwrap();
        // Both leases outlive the originating TLS pool and thread.
        assert_eq!(snapshot.as_bytes(), vec![7u8; 8192].encode_to_vec());
        assert_eq!(bytes.as_ref(), vec![9u8; 4096].encode_to_vec());
        drop((snapshot, bytes));
    }
}
