# RPC Client Context Interceptor

The `rpc_client_ctx` attribute allows you to inject custom context into every client method call through an interceptor trait.

## Usage

Add the `rpc_client_ctx` parameter to your `#[proto_rpc(...)]` macro:

```rust
#[proto_rpc(
    rpc_package = "my_package",
    rpc_server = true,
    rpc_client = true,
    rpc_client_ctx = "MyInterceptor<Ctx>",
    proto_path = "protos/my_service.proto"
)]
pub trait MyService {
    async fn my_method(&self, request: Request<MyRequest>) -> Result<Response<MyResponse>, Status>;
}
```

## Syntax

The `rpc_client_ctx` value must follow this format:
```
"InterceptorTrait<Ctx>"
```

Where:
- `InterceptorTrait`: The name of the interceptor trait that will be generated for you.
- `Ctx`: The context type parameter name used by the generated trait and client methods.

## Interceptor Trait Signature

The macro generates a trait with this signature:

```rust
pub trait InterceptorTrait<Ctx>: Send + Sync + 'static {
    fn intercept<T>(&self, ctx: Ctx, req: &mut tonic::Request<T>);
}
```

The generic parameter `<T>` allows the interceptor to work with any request type.

## Example

```rust
use proto_rs::proto_rpc;
use tonic::Request;

pub type UserId = u64;

#[derive(Clone, Debug)]
struct UserCtx(UserId);

impl From<UserCtx> for UserId {
    fn from(value: UserCtx) -> Self {
        value.0
    }
}

// Define the RPC service with the interceptor
#[proto_rpc(
    rpc_package = "my_service",
    rpc_server = true,
    rpc_client = true,
    rpc_client_ctx = "UserAdvancedInterceptor<Ctx>",
    proto_path = "protos/my_service.proto"
)]
pub trait MyService {
    async fn get_data(&self, request: Request<GetDataRequest>) -> Result<Response<GetDataResponse>, Status>;
}

impl UserAdvancedInterceptor<UserId> for UserCtx {
    fn intercept<T>(&self, ctx: UserId, request: &mut tonic::Request<T>) {
        // Add user ID to request metadata
        request.metadata_mut().insert(
            "user-id",
            ctx.to_string().parse().unwrap(),
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

// With rpc_client_ctx = "UserAdvancedInterceptor<Ctx>":
pub async fn get_data<R, I, Ctx>(
    &mut self,
    ctx: I,
    request: R,
) -> Result<...>
where
    I: Clone + Into<Ctx> + UserAdvancedInterceptor<Ctx>;
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
- The context type is inferred via `Into<Ctx>` on the value passed to the client method
- The interceptor applies to all methods in the service trait, both unary and streaming
