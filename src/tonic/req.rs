use tonic::Request;

use crate::ProtoEncode;
use crate::ProtoExt;
use crate::coders::SunByRef;
use crate::coders::ZeroCopyMode;
use crate::traits::ProtoArchive;
use crate::traits::ZeroCopy;

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

// ZeroCopy<T> can be used in place of T for requests
impl<T> ProtoRequest<T> for ZeroCopy<T>
where
    T: ProtoEncode + ProtoExt + Send + Sync + 'static,
    for<'s> <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    type Encode = ZeroCopy<T>;
    type Mode = ZeroCopyMode;
    #[inline]
    fn into_request(self) -> Request<Self::Encode> {
        Request::new(self)
    }
}

impl<T> ProtoRequest<T> for Request<ZeroCopy<T>>
where
    T: ProtoEncode + ProtoExt + Send + Sync + 'static,
    for<'s> <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    type Encode = ZeroCopy<T>;
    type Mode = ZeroCopyMode;
    #[inline]
    fn into_request(self) -> Request<Self::Encode> {
        self
    }
}
