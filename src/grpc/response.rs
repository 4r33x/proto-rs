use crate::BytesMode;
use crate::ProtoEncode;
use crate::SunByRef;
use crate::ZeroCopy;
use crate::alloc::boxed::Box;
use crate::alloc::sync::Arc;
use crate::grpc::Response;

pub trait ProtoResponse<T>: Sized {
    type Encode: Send + Sync + 'static;
    type Mode: Send + Sync + 'static;

    fn into_response(self) -> Response<Self::Encode>;
}

impl<T> ProtoResponse<T> for Response<T>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = T;
    type Mode = SunByRef;

    fn into_response(self) -> Response<Self::Encode> {
        self
    }
}

impl<T> ProtoResponse<T> for Response<Arc<T>>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = Arc<T>;
    type Mode = crate::coders::SunByRefDeref;

    fn into_response(self) -> Response<Self::Encode> {
        self
    }
}

impl<T> ProtoResponse<T> for Response<Box<T>>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = Box<T>;
    type Mode = crate::coders::SunByRefDeref;

    fn into_response(self) -> Response<Self::Encode> {
        self
    }
}

impl<T> ProtoResponse<T> for T
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = T;
    type Mode = SunByRef;

    fn into_response(self) -> Response<Self::Encode> {
        Response::new(self)
    }
}

impl<T> ProtoResponse<T> for Response<ZeroCopy<T>>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = ZeroCopy<T>;
    type Mode = BytesMode;

    fn into_response(self) -> Response<Self::Encode> {
        self
    }
}

impl<T> ProtoResponse<T> for ZeroCopy<T>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = ZeroCopy<T>;
    type Mode = BytesMode;

    fn into_response(self) -> Response<Self::Encode> {
        Response::new(self)
    }
}

pub fn map_proto_response<R, P>(value: R) -> <R as ProtoResponse<P>>::Encode
where
    R: ProtoResponse<P>,
    P: ProtoEncode,
{
    R::into_response(value).into_inner()
}
