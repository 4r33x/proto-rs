# RPC Client Context Interceptor

The `rpc_client_ctx` attribute allows you to inject custom context into every client method call through an interceptor trait.

## Usage

Add the `rpc_client_ctx` parameter to your `#[proto_rpc(...)]` macro:

```rust
#[proto_rpc(
    rpc_package = "my_package",
    rpc_server = true,
    rpc_client = true,
    rpc_client_ctx = "MyInterceptor",
    proto_path = "protos/my_service.proto"
)]
pub trait MyService {
    async fn my_method(&self, request: Request<MyRequest>) -> Result<Response<MyResponse>, Status>;
}
```

## Interceptor Trait Signature

Define an interceptor trait with an associated payload type and an `intercept` method:

```rust
pub trait InterceptorTrait: Send + Sync + 'static + Sized {
    type Payload: From<Self>;
    fn intercept<T>(&self, req: &mut tonic::Request<T>);
}
```

The generic parameter `<T>` allows the interceptor to work with any request type.
The payload type is used to accept context values without explicit conversion at the call site.

## Example

```rust
use proto_rs::proto_rpc;
use tonic::Request;

pub type UserId = u64;

#[derive(Clone, Debug)]
struct UserCtx(UserId);

// Define the RPC service with the interceptor
#[proto_rpc(
    rpc_package = "my_service",
    rpc_server = true,
    rpc_client = true,
    rpc_client_ctx = "UserAdvancedInterceptor",
    proto_path = "protos/my_service.proto"
)]
pub trait MyService {
    async fn get_data(&self, request: Request<GetDataRequest>) -> Result<Response<GetDataResponse>, Status>;
}

pub trait UserAdvancedInterceptor: Send + Sync + 'static + Sized {
    type Payload: From<Self>;
    fn intercept<T>(&self, req: &mut tonic::Request<T>);
}

impl From<UserCtx> for UserId {
    fn from(value: UserCtx) -> Self {
        value.0
    }
}

impl UserAdvancedInterceptor for UserCtx {
    type Payload = UserId;
    fn intercept<T>(&self, request: &mut tonic::Request<T>) {
        // Add user ID to request metadata
        request.metadata_mut().insert(
            "user-id", self.0.to_string().parse().unwrap(),
        );
    }
}
```

## Generated Client Methods

With the `rpc_client_ctx` attribute, all generated client methods will include the context parameter:

```rust
// Without rpc_client_ctx:
pub async fn get_data<R>(
    &mut self,
    request: R,
) -> Result<...>

// With rpc_client_ctx = "UserAdvancedInterceptor":
pub async fn get_data<R, I, ProtoInter>(
    &mut self,
    ctx: I,
    request: R,
) -> Result<...>
where
    I: Into<<ProtoInter as UserAdvancedInterceptor>::Payload>,
    ProtoInter: UserAdvancedInterceptor + From<<ProtoInter as UserAdvancedInterceptor>::Payload>;
```

## Use Cases

- **Authentication**: Add user credentials to every request
- **Tracing**: Add trace IDs for distributed tracing
- **Multitenancy**: Add tenant IDs to requests
- **Custom Headers**: Add any custom metadata to requests
- **Request Modification**: Transform requests before sending

## Notes

- The interceptor is called after the request is converted using `ProtoRequest::into_request()`
- The interceptor receives a mutable reference to the `tonic::Request`, allowing full customization
- The context type is inferred via `Into<<Interceptor as UserAdvancedInterceptor>::Payload>` on the value passed to the client method
- Your interceptor type should implement `From<Payload>` so the client can construct it from the payload value
- The interceptor applies to all methods in the service trait, both unary and streaming
