use private::TaskCtx;
use proto_rs::DecodeError;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoShadowDecode;
use proto_rs::ProtoShadowEncode;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;
mod private {
    #[derive(Clone, PartialEq, Debug)]
    pub struct TaskCtx {
        flags: u32,
        values: u32,
    }
    impl TaskCtx {
        pub fn new(flags: u32, values: u32) -> Self {
            Self { flags, values }
        }
        pub fn flags(&self) -> &u32 {
            &self.flags
        }

        pub fn values(&self) -> &u32 {
            &self.values
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
struct Task {
    cfg_id: u64,
    user_id: u64,
    ctx: TaskCtx,
}

struct TaskRef<'a> {
    cfg_id: u64,
    user_id: u64,
    ctx: &'a TaskCtx,
}

//flattened with getters
#[proto_message(sun = Task)]
struct TaskProto {
    cfg_id: u64,
    user_id: u64,
    #[proto(getter = "&*$.ctx.flags()")]
    flags: u32,
    #[proto(tag = 4, getter = "&*$.ctx.values()")]
    values: u32,
}

impl ProtoShadowDecode<Task> for TaskProto {
    fn to_sun(self) -> Result<Task, DecodeError> {
        Ok(Task {
            cfg_id: self.cfg_id,
            user_id: self.user_id,
            ctx: TaskCtx::new(self.flags, self.values),
        })
    }
}

impl<'a> ProtoShadowEncode<'a, Task> for TaskProto {
    fn from_sun(value: &'a Task) -> Self {
        let view = TaskRef {
            cfg_id: value.cfg_id,
            user_id: value.user_id,
            ctx: &value.ctx,
        };
        Self {
            cfg_id: view.cfg_id,
            user_id: view.user_id,
            flags: *view.ctx.flags(),
            values: *view.ctx.values(),
        }
    }
}

impl ProtoEncode for Task {
    type Shadow<'a> = TaskProto;
}

impl ProtoDecode for Task {
    type ShadowDecoded = TaskProto;
}

#[test]
fn encode_decode_reference_with_getter() {
    let task = Task {
        cfg_id: 7,
        user_id: 9,
        ctx: TaskCtx::new(1, 2),
    };
    let bytes = Task::encode_to_vec(&task);
    let decoded = <Task as ProtoDecode>::decode(bytes.as_slice(), DecodeContext::default())
        .expect("decode task with getters");

    assert_eq!(decoded, task);
}

#[cfg(feature = "papaya")]
#[test]
fn encode_decode_papaya_getters() {
    use papaya::HashMap;
    use papaya::HashSet;

    #[proto_message]
    struct PapayaHolder {
        #[proto(getter = "&$.map")]
        map: HashMap<u64, u64>,
        #[proto(getter = "&$.set")]
        set: HashSet<u64>,
    }

    let map = HashMap::default();
    let set = HashSet::default();
    let guard = map.pin();
    guard.insert(1, 10);
    guard.insert(2, 20);
    drop(guard);
    let guard = set.pin();
    guard.insert(7);
    guard.insert(8);
    drop(guard);
    let holder = PapayaHolder { map, set };

    let bytes = PapayaHolder::encode_to_vec(&holder);
    let decoded = <PapayaHolder as ProtoDecode>::decode(bytes.as_slice(), DecodeContext::default())
        .expect("decode papaya holder");

    let map_guard = decoded.map.pin();
    assert_eq!(map_guard.get(&1), Some(&10));
    assert_eq!(map_guard.get(&2), Some(&20));
    drop(map_guard);
    let set_guard = decoded.set.pin();
    assert!(set_guard.contains(&7));
    assert!(set_guard.contains(&8));
}
