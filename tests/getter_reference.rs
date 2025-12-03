use private::TaskCtx;
use proto_rs::DecodeError;
use proto_rs::ProtoExt;
use proto_rs::ProtoShadow;
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

impl ProtoShadow<Task> for TaskProto {
    type Sun<'a> = &'a Task;
    type OwnedSun = Task;
    type View<'a> = TaskRef<'a>;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(Task {
            cfg_id: self.cfg_id,
            user_id: self.user_id,
            ctx: TaskCtx::new(self.flags, self.values),
        })
    }

    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        TaskRef {
            cfg_id: value.cfg_id,
            user_id: value.user_id,
            ctx: &value.ctx,
        }
    }
}

#[test]
fn encode_decode_reference_with_getter() {
    let task = Task {
        cfg_id: 7,
        user_id: 9,
        ctx: TaskCtx::new(1, 2),
    };
    let bytes = Task::encode_to_vec(&task);
    let decoded = Task::decode(bytes.as_slice()).expect("decode task with getters");

    assert_eq!(decoded, task);
}
