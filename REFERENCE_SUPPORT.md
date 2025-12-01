# Reference Field Support Status

## Current Status

The proto_message macro now **accepts** lifetime parameters in struct definitions:

```rust
#[proto_message]
struct MyType<'a> {
    value: u32,
    name: String,  // Works fine
    // reference fields below need more work
}
```

## Reference Fields - Architectural Challenge

Supporting actual reference fields like `&'a String` requires significant architectural changes:

```rust
#[proto_message]
struct RefType<'a> {
    value: u32,
    name: &'a String,  // ← This needs special handling
}
```

### Why It's Complex

1. **Shadow Type Mismatch**: Currently, `Shadow<'b> = RefType<'b>` but `RefType<'b>` contains references
2. **Encoding vs Decoding Asymmetry**:
   - **Encoding**: Can work - we borrow from `&'a String` and serialize
   - **Decoding**: Problem - we create owned `String` but need `&'a String`

### What's Needed

To fully support reference fields, we need to:

1. **Generate a separate Shadow struct** with owned fields:
   ```rust
   // Original (user-defined)
   struct RefType<'a> {
       value: u32,
       name: &'a String,
   }

   // Generated Shadow (owns data)
   struct RefTypeShadow {
       value: u32,
       name: String,  // Owned, not borrowed
   }
   ```

2. **Implement ProtoShadow** to handle conversion
3. **Update encode logic** to borrow from references
4. **Update decode logic** to create owned data
5. **Document that decoding produces owned data**, not the reference type

### Workaround

For now, use owned types in proto messages and create reference wrappers separately:

```rust
// Proto message with owned data
#[proto_message]
struct DataOwned {
    value: u32,
    name: String,
}

// Wrapper with references (not a proto message)
struct DataRef<'a> {
    value: u32,
    name: &'a str,
}

impl<'a> From<&'a DataOwned> for DataRef<'a> {
    fn from(owned: &'a DataOwned) -> Self {
        DataRef {
            value: owned.value,
            name: &owned.name,
        }
    }
}
```

## Implementation Complexity

Implementing full reference support would require:
- ~200-300 lines of macro code changes
- New Shadow struct generation logic
- Field-by-field type transformation
- ProtoShadow trait implementation generation
- Comprehensive test coverage

This is a substantial feature addition that changes the fundamental architecture of how the macro works.
