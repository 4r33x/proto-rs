use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Code {
    Unknown,
    InvalidArgument,
    Internal,
    Unimplemented,
}

#[derive(Default)]
pub struct Extensions {
    values: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Extensions {
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> Option<T> {
        self.values.insert(TypeId::of::<T>(), Box::new(value)).and_then(|old| old.downcast().ok()).map(|old| *old)
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.values.get(&TypeId::of::<T>())?.downcast_ref()
    }

    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.values.get_mut(&TypeId::of::<T>())?.downcast_mut()
    }
}

pub struct Request<T> {
    message: T,
    extensions: Extensions,
}

impl<T> Request<T> {
    pub fn new(message: T) -> Self {
        Self {
            message,
            extensions: Extensions::default(),
        }
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

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

pub struct Response<T> {
    message: T,
}

impl<T> Response<T> {
    pub const fn new(message: T) -> Self {
        Self { message }
    }

    pub const fn get_ref(&self) -> &T {
        &self.message
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.message
    }

    pub fn into_inner(self) -> T {
        self.message
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    code: Code,
    message: String,
}

impl Status {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(Code::InvalidArgument, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(Code::Internal, message)
    }

    pub const fn code(&self) -> Code {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
