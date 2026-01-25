use tonic::Request;

use crate::BytesMode;
use crate::ProtoEncode;
use crate::ZeroCopy;
use crate::coders::SunByRef;

pub trait ProtoRequest<T>: Sized {
    type Encode: Send + Sync + 'static;
    type Mode: Send + Sync + 'static;
    fn into_request(self) -> Request<Self::Encode>;
}

impl<T> ProtoRequest<T> for Request<T>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = T;
    type Mode = SunByRef;
    #[inline]
    fn into_request(self) -> Request<Self::Encode> {
        self
    }
}

impl<T> ProtoRequest<T> for T
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = T;
    type Mode = SunByRef;
    #[inline]
    fn into_request(self) -> Request<Self::Encode> {
        Request::new(self)
    }
}

impl<T> ProtoRequest<T> for Request<ZeroCopy<T>>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = ZeroCopy<T>;
    type Mode = BytesMode;
    #[inline]
    fn into_request(self) -> Request<Self::Encode> {
        self
    }
}

impl<T> ProtoRequest<T> for ZeroCopy<T>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = ZeroCopy<T>;
    type Mode = BytesMode;
    #[inline]
    fn into_request(self) -> Request<Self::Encode> {
        Request::new(self)
    }
}
