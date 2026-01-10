# RPC Client Context Interceptor

The `rpc_client_ctx` attribute allows you to inject custom context into every client method call through an interceptor function.

## Usage

Add the `rpc_client_ctx` parameter to your `#[proto_rpc(...)]` macro:

```rust
#[proto_rpc(
    rpc_package = "my_package",
    rpc_server = true,
    rpc_client = true,
    rpc_client_ctx = "my_interceptor<ContextType>",
    proto_path = "protos/my_service.proto"
)]
pub trait MyService {
    async fn my_method(&self, request: Request<MyRequest>) -> Result<Response<MyResponse>, Status>;
}
```

## Syntax

The `rpc_client_ctx` value must follow this format:
```
"function_name<TypeParameter>"
```

Where:
- `function_name`: The name of your interceptor function
- `TypeParameter`: The type of the context parameter (can include nested generics)

## Interceptor Function Signature

Your interceptor function must have this signature:

```rust
fn interceptor_name<T>(ctx: YourContextType, request: &mut tonic::Request<T>) {
    // Modify the request before it's sent
}
```

The generic parameter `<T>` allows the interceptor to work with any request type.

## Example

```rust
use proto_rs::proto_rpc;
use tonic::Request;

pub type UserId = u64;

// Define the interceptor function
fn user_advanced_interceptor<T>(ctx: UserId, request: &mut tonic::Request<T>) {
    // Add user ID to request metadata
    request.metadata_mut().insert(
        "user-id",
        ctx.to_string().parse().unwrap(),
    );
}

// Define the RPC service with the interceptor
#[proto_rpc(
    rpc_package = "my_service",
    rpc_server = true,
    rpc_client = true,
    rpc_client_ctx = "user_advanced_interceptor<UserId>",
    proto_path = "protos/my_service.proto"
)]
pub trait MyService {
    async fn get_data(&self, request: Request<GetDataRequest>) -> Result<Response<GetDataResponse>, Status>;
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

// With rpc_client_ctx = "user_advanced_interceptor<UserId>":
pub async fn get_data<R>(
    &mut self,
    ctx: UserId,
    request: R,
) -> Result<...>
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
- The context type can be any Rust type, including types with generic parameters
- The interceptor applies to all methods in the service trait, both unary and streaming
