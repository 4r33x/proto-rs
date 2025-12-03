use proto_rs::DecodeError;
use proto_rs::ProtoExt;
use proto_rs::ProtoShadow;
use proto_rs::proto_message;

#[derive(Clone, PartialEq, Debug)]
struct TaskCtx {
    flags: u32,
    values: u32,
}

impl TaskCtx {
    fn flags(&self) -> &u32 {
        &self.flags
    }

    fn values(&self) -> &u32 {
        &self.values
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

#[proto_message(sun = Task)]
struct TaskProto {
    #[proto(tag = 1)]
    cfg_id: u64,
    #[proto(tag = 2)]
    user_id: u64,
    #[proto(tag = 3, getter = "$.ctx.flags()")]
    flags: u32,
    #[proto(tag = 4, getter = "$.ctx.values()")]
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
            ctx: TaskCtx {
                flags: self.flags,
                values: self.values,
            },
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
        ctx: TaskCtx { flags: 1, values: 2 },
    };
    let bytes = Task::encode_to_vec(&task);
    let decoded = Task::decode(bytes.as_slice()).expect("decode task with getters");

    assert_eq!(decoded, task);
}
