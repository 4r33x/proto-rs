use proto_rs::ProtoExt;
use proto_rs::proto_message;

#[proto_message(proto_path = "protos/tests/mutex_example.proto")]
#[derive(Debug, Default)]
struct MutexInner {
    #[proto(tag = 1)]
    value: String,
    #[proto(tag = 2)]
    count: u32,
}

#[proto_message(proto_path = "protos/tests/mutex_example.proto")]
#[derive(Debug)]
struct ExampleStdMutex {
    #[proto(tag = 1)]
    value: std::sync::Mutex<MutexInner>,
}

impl Default for ExampleStdMutex {
    fn default() -> Self {
        Self {
            value: std::sync::Mutex::new(MutexInner {
                value: String::from("hello"),
                count: 1,
            }),
        }
    }
}

#[cfg(feature = "parking_lot")]
#[proto_message(proto_path = "protos/tests/mutex_example.proto")]
#[derive(Debug)]
struct ExampleParkingLotMutex {
    #[proto(tag = 1)]
    value: parking_lot::Mutex<MutexInner>,
}

#[cfg(feature = "parking_lot")]
impl Default for ExampleParkingLotMutex {
    fn default() -> Self {
        Self {
            value: parking_lot::Mutex::new(MutexInner {
                value: String::from("world"),
                count: 2,
            }),
        }
    }
}

fn main() {
    let std_holder = ExampleStdMutex::default();
    let encoded = <ExampleStdMutex as ProtoExt>::encode_to_vec(&std_holder);
    let decoded = <ExampleStdMutex as ProtoExt>::decode(&encoded[..]).expect("decode std mutex");
    let inner = decoded.value.into_inner().expect("mutex poisoned");
    println!("decoded std mutex: {} ({})", inner.value, inner.count);

    #[cfg(feature = "parking_lot")]
    {
        let parking_holder = ExampleParkingLotMutex::default();
        let encoded = <ExampleParkingLotMutex as ProtoExt>::encode_to_vec(&parking_holder);
        let decoded = <ExampleParkingLotMutex as ProtoExt>::decode(&encoded[..]).expect("decode parking_lot mutex");
        let inner = decoded.value.into_inner();
        println!("decoded parking_lot mutex: {} ({})", inner.value, inner.count);
    }
}
