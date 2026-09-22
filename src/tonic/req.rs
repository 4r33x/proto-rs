use tonic::Request;

use crate::EncodedSnapshot;
use crate::ProtoEncode;

/// Synchronously prepare a request. The result owns its payload and does not
/// retain the input, even when `Self` contains a non-static borrow.
pub trait PrepareRequest<T>: Sized {
    fn prepare_request(self, limit: usize) -> Result<Request<crate::PreparedMessage>, tonic::Status>;
}

fn prepare<T: ProtoEncode + crate::ProtoExt>(request: Request<&T>, limit: usize) -> Result<Request<crate::PreparedMessage>, tonic::Status> {
    let (metadata, extensions, value) = request.into_parts();
    Ok(Request::from_parts(
        metadata,
        extensions,
        crate::traits::prepare_owned(value, limit)?,
    ))
}

impl<T: ProtoEncode + crate::ProtoExt> PrepareRequest<T> for &T {
    fn prepare_request(self, limit: usize) -> Result<Request<crate::PreparedMessage>, tonic::Status> {
        prepare(Request::new(self), limit)
    }
}

impl<T: ProtoEncode + crate::ProtoExt> PrepareRequest<T> for T {
    fn prepare_request(self, limit: usize) -> Result<Request<crate::PreparedMessage>, tonic::Status> {
        prepare(Request::new(&self), limit)
    }
}

impl<T: ProtoEncode + crate::ProtoExt> PrepareRequest<T> for Request<&T> {
    fn prepare_request(self, limit: usize) -> Result<Request<crate::PreparedMessage>, tonic::Status> {
        prepare(self, limit)
    }
}

impl<T: ProtoEncode + crate::ProtoExt> PrepareRequest<T> for Request<T> {
    fn prepare_request(self, limit: usize) -> Result<Request<crate::PreparedMessage>, tonic::Status> {
        let (metadata, extensions, value) = self.into_parts();
        prepare(Request::from_parts(metadata, extensions, &value), limit)
    }
}

impl<T: ProtoEncode> PrepareRequest<T> for EncodedSnapshot<T> {
    fn prepare_request(self, _: usize) -> Result<Request<crate::PreparedMessage>, tonic::Status> {
        self.into_owned_frame().map(Request::new)
    }
}

impl<T: ProtoEncode> PrepareRequest<T> for Request<EncodedSnapshot<T>> {
    fn prepare_request(self, _: usize) -> Result<Request<crate::PreparedMessage>, tonic::Status> {
        let (metadata, extensions, value) = self.into_parts();
        Ok(Request::from_parts(metadata, extensions, value.into_owned_frame()?))
    }
}

macro_rules! pointer_request {
    ($ptr:ident) => {
        impl<T: ProtoEncode + crate::ProtoExt> PrepareRequest<T> for $ptr<T> {
            fn prepare_request(self, limit: usize) -> Result<Request<crate::PreparedMessage>, tonic::Status> {
                prepare(Request::new(self.as_ref()), limit)
            }
        }
        impl<T: ProtoEncode + crate::ProtoExt> PrepareRequest<T> for Request<$ptr<T>> {
            fn prepare_request(self, limit: usize) -> Result<Request<crate::PreparedMessage>, tonic::Status> {
                let (metadata, extensions, value) = self.into_parts();
                prepare(Request::from_parts(metadata, extensions, value.as_ref()), limit)
            }
        }
    };
}
use std::sync::Arc;
pointer_request!(Arc);
pointer_request!(Box);
