use std::fmt;

pub use http::Extensions;
pub type MetadataMap = http::HeaderMap;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(i32)]
pub enum Code {
    Ok = 0,
    Cancelled = 1,
    Unknown = 2,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    AlreadyExists = 6,
    PermissionDenied = 7,
    ResourceExhausted = 8,
    FailedPrecondition = 9,
    Aborted = 10,
    OutOfRange = 11,
    Unimplemented = 12,
    Internal = 13,
    Unavailable = 14,
    DataLoss = 15,
    Unauthenticated = 16,
}

impl Code {
    pub const fn from_i32(value: i32) -> Self {
        match value {
            0 => Self::Ok,
            1 => Self::Cancelled,
            2 => Self::Unknown,
            3 => Self::InvalidArgument,
            4 => Self::DeadlineExceeded,
            5 => Self::NotFound,
            6 => Self::AlreadyExists,
            7 => Self::PermissionDenied,
            8 => Self::ResourceExhausted,
            9 => Self::FailedPrecondition,
            10 => Self::Aborted,
            11 => Self::OutOfRange,
            12 => Self::Unimplemented,
            13 => Self::Internal,
            14 => Self::Unavailable,
            15 => Self::DataLoss,
            16 => Self::Unauthenticated,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct Request<T> {
    metadata: MetadataMap,
    extensions: Extensions,
    message: T,
}

impl<T> Request<T> {
    pub fn new(message: T) -> Self {
        Self::from_parts(MetadataMap::new(), Extensions::new(), message)
    }

    pub fn from_parts(metadata: MetadataMap, extensions: Extensions, message: T) -> Self {
        Self {
            metadata,
            extensions,
            message,
        }
    }

    pub fn into_parts(self) -> (MetadataMap, Extensions, T) {
        (self.metadata, self.extensions, self.message)
    }

    pub fn get_ref(&self) -> &T {
        &self.message
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.message
    }

    pub fn into_inner(self) -> T {
        self.message
    }

    pub fn metadata(&self) -> &MetadataMap {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut MetadataMap {
        &mut self.metadata
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Request<U> {
        Request::from_parts(self.metadata, self.extensions, map(self.message))
    }
}

#[derive(Debug)]
pub struct Response<T> {
    metadata: MetadataMap,
    message: T,
    extensions: Extensions,
}

impl<T> Response<T> {
    pub fn new(message: T) -> Self {
        Self::from_parts(MetadataMap::new(), message, Extensions::new())
    }

    pub fn from_parts(metadata: MetadataMap, message: T, extensions: Extensions) -> Self {
        Self {
            metadata,
            message,
            extensions,
        }
    }

    pub fn into_parts(self) -> (MetadataMap, T, Extensions) {
        (self.metadata, self.message, self.extensions)
    }

    pub fn get_ref(&self) -> &T {
        &self.message
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.message
    }

    pub fn into_inner(self) -> T {
        self.message
    }

    pub fn metadata(&self) -> &MetadataMap {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut MetadataMap {
        &mut self.metadata
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Response<U> {
        Response::from_parts(self.metadata, map(self.message), self.extensions)
    }
}

impl<T> From<T> for Response<T> {
    fn from(message: T) -> Self {
        Self::new(message)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    code: Code,
    message: String,
    details: bytes::Bytes,
    metadata: Box<MetadataMap>,
}

impl Status {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self::with_details_and_metadata(code, message, bytes::Bytes::new(), MetadataMap::new())
    }

    pub fn with_details(code: Code, message: impl Into<String>, details: bytes::Bytes) -> Self {
        Self::with_details_and_metadata(code, message, details, MetadataMap::new())
    }

    pub fn with_metadata(code: Code, message: impl Into<String>, metadata: MetadataMap) -> Self {
        Self::with_details_and_metadata(code, message, bytes::Bytes::new(), metadata)
    }

    pub fn with_details_and_metadata(code: Code, message: impl Into<String>, details: bytes::Bytes, metadata: MetadataMap) -> Self {
        Self {
            code,
            message: message.into(),
            details,
            metadata: Box::new(metadata),
        }
    }

    pub const fn code(&self) -> Code {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn details(&self) -> &[u8] {
        &self.details
    }

    pub fn metadata(&self) -> &MetadataMap {
        &self.metadata
    }

    #[cfg(feature = "tonic")]
    pub(crate) fn into_parts(self) -> (Code, String, bytes::Bytes, MetadataMap) {
        (self.code, self.message, self.details, *self.metadata)
    }
}

macro_rules! status_constructors {
    ($($name:ident => $code:ident),* $(,)?) => {
        $(
            pub fn $name(message: impl Into<String>) -> Self {
                Self::new(Code::$code, message)
            }
        )*
    };
}

impl Status {
    status_constructors! {
        cancelled => Cancelled,
        unknown => Unknown,
        invalid_argument => InvalidArgument,
        deadline_exceeded => DeadlineExceeded,
        not_found => NotFound,
        already_exists => AlreadyExists,
        permission_denied => PermissionDenied,
        resource_exhausted => ResourceExhausted,
        failed_precondition => FailedPrecondition,
        aborted => Aborted,
        out_of_range => OutOfRange,
        unimplemented => Unimplemented,
        internal => Internal,
        unavailable => Unavailable,
        data_loss => DataLoss,
        unauthenticated => Unauthenticated,
    }
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for Status {}
