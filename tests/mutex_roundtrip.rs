use proto_rs::ProtoExt;
use proto_rs::proto_message;

#[proto_message(proto_path = "protos/tests/mutex.proto")]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MutexInner {
    #[proto(tag = 1)]
    pub value: String,
    #[proto(tag = 2)]
    pub count: u32,
}

#[proto_message(proto_path = "protos/tests/mutex.proto")]
#[derive(Debug)]
pub struct StdMutexHolder {
    #[proto(tag = 1)]
    pub inner: std::sync::Mutex<MutexInner>,
}

impl Default for StdMutexHolder {
    fn default() -> Self {
        Self {
            inner: std::sync::Mutex::new(MutexInner::default()),
        }
    }
}

#[test]
fn std_mutex_roundtrip_preserves_inner_values() {
    let holder = StdMutexHolder {
        inner: std::sync::Mutex::new(MutexInner { value: "alpha".into(), count: 42 }),
    };

    let encoded = <StdMutexHolder as ProtoExt>::encode_to_vec(&holder);
    let decoded = <StdMutexHolder as ProtoExt>::decode(&encoded[..]).expect("decode std mutex holder");

    assert_eq!(decoded.inner.into_inner().expect("mutex poisoned"), MutexInner { value: "alpha".into(), count: 42 });
}

#[test]
fn std_mutex_roundtrip_handles_default_values() {
    let holder = StdMutexHolder::default();

    let encoded = <StdMutexHolder as ProtoExt>::encode_to_vec(&holder);
    let decoded = <StdMutexHolder as ProtoExt>::decode(&encoded[..]).expect("decode default std mutex holder");

    assert_eq!(decoded.inner.into_inner().expect("mutex poisoned"), MutexInner::default());
}

#[cfg(feature = "parking_lot")]
#[proto_message(proto_path = "protos/tests/mutex.proto")]
#[derive(Debug)]
pub struct ParkingLotMutexHolder {
    #[proto(tag = 1)]
    pub inner: parking_lot::Mutex<MutexInner>,
}

#[cfg(feature = "parking_lot")]
impl Default for ParkingLotMutexHolder {
    fn default() -> Self {
        Self {
            inner: parking_lot::Mutex::new(MutexInner::default()),
        }
    }
}

#[cfg(feature = "parking_lot")]
#[test]
fn parking_lot_mutex_roundtrip_preserves_inner_values() {
    let holder = ParkingLotMutexHolder {
        inner: parking_lot::Mutex::new(MutexInner { value: "beta".into(), count: 7 }),
    };

    let encoded = <ParkingLotMutexHolder as ProtoExt>::encode_to_vec(&holder);
    let decoded = <ParkingLotMutexHolder as ProtoExt>::decode(&encoded[..]).expect("decode parking_lot mutex holder");

    assert_eq!(decoded.inner.into_inner(), MutexInner { value: "beta".into(), count: 7 });
}

#[cfg(feature = "parking_lot")]
#[test]
fn parking_lot_mutex_roundtrip_handles_default_values() {
    let holder = ParkingLotMutexHolder::default();

    let encoded = <ParkingLotMutexHolder as ProtoExt>::encode_to_vec(&holder);
    let decoded = <ParkingLotMutexHolder as ProtoExt>::decode(&encoded[..]).expect("decode default parking_lot mutex holder");

    assert_eq!(decoded.inner.into_inner(), MutexInner::default());
}
