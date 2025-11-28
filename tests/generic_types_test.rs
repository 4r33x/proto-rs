use std::collections::HashMap;

// This generates proto messages for all K,V combinations:
// - MapWrapperStructU64String
// - MapWrapperStructU64U16
// - MapWrapperStructU32String
// - MapWrapperStructU32U16
#[::proto_rs::proto_message(
    proto_path = "test.proto",
    proto_generic_types = [K = [u64, u32], V = [String, u16]]
)]
pub struct MapWrapperStruct<K, V>(
    #[proto(tag = 1)]
    pub HashMap<K, V>,
);

#[test]
fn test_generic_types_compile() {
    // Test that the struct definition works with different generic instantiations
    let _map1: MapWrapperStruct<u64, String> = MapWrapperStruct(HashMap::new());
    let _map2: MapWrapperStruct<u32, u16> = MapWrapperStruct(HashMap::new());

    // Proto files are generated for all concrete type combinations
    // The proto messages can be used for interop with other languages
    assert!(true);
}
