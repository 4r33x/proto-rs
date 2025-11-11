use proto_rs::ProtoExt;
use proto_rs::ToZeroCopyRequest;
use proto_rs::ToZeroCopyResponse;

mod encoding_messages;

use bytes::Bytes;
use encoding_messages::SampleMessage;
use encoding_messages::ZeroCopyContainer;
use encoding_messages::sample_message;

#[test]
fn zero_copy_request_preserves_metadata() {
    let message = sample_message();
    let mut request = tonic::Request::new(message.clone());
    request.metadata_mut().insert("x-trace", "abc123".parse().expect("valid metadata value"));
    request.extensions_mut().insert::<usize>(99);

    let expected_bytes = SampleMessage::encode_to_vec(&message);

    let zero_copy: proto_rs::ZeroCopy<_> = request.into();
    assert_eq!(zero_copy.bytes(), expected_bytes.as_slice());
    assert_eq!(zero_copy.metadata().get("x-trace").unwrap(), "abc123");
    assert_eq!(*zero_copy.extensions().get::<usize>().unwrap(), 99);

    let back_to_request: tonic::Request<Vec<u8>> = zero_copy.into();
    assert_eq!(back_to_request.get_ref(), &expected_bytes);
}

#[test]
fn zero_copy_response_preserves_metadata() {
    let message = sample_message();
    let mut response = tonic::Response::new(message.clone());
    response.metadata_mut().insert("x-resp", "value".parse().expect("valid metadata value"));
    response.extensions_mut().insert::<String>("ext".into());

    let expected_bytes = SampleMessage::encode_to_vec(&message);

    let zero_copy: proto_rs::ZeroCopy<_> = response.into();
    assert_eq!(zero_copy.bytes(), expected_bytes.as_slice());
    assert_eq!(zero_copy.metadata().get("x-resp").unwrap(), "value");
    assert_eq!(zero_copy.extensions().get::<String>().unwrap(), "ext");

    let back_to_response: tonic::Response<Vec<u8>> = zero_copy.into();
    assert_eq!(back_to_response.get_ref(), &expected_bytes);
}

#[test]
fn borrowed_request_zero_copy_matches_manual_encoding() {
    let message = sample_message();
    let zero_copy = tonic::Request::new(&message).to_zero_copy();
    let expected = SampleMessage::encode_to_vec(&message);

    assert_eq!(zero_copy.bytes(), expected.as_slice());
}

#[test]
fn borrowed_response_zero_copy_matches_manual_encoding() {
    let message = sample_message();
    let zero_copy = tonic::Response::new(&message).to_zero_copy();
    let expected = SampleMessage::encode_to_vec(&message);

    assert_eq!(zero_copy.bytes(), expected.as_slice());
}

#[test]
fn zero_copy_conversion_roundtrip_maintains_bytes_identity() {
    let mut request = tonic::Request::new(sample_message());
    request.metadata_mut().insert("id", "42".parse().unwrap());

    let zero_from_owned: proto_rs::ZeroCopy<_> = request.into();
    let zero_from_borrowed = tonic::Request::new(&sample_message()).to_zero_copy();

    assert_eq!(zero_from_owned.bytes(), zero_from_borrowed.bytes());
}

#[test]
fn zero_copy_response_roundtrip_maintains_bytes_identity() {
    let message = sample_message();
    let zero_from_owned: proto_rs::ZeroCopy<_> = tonic::Response::new(message.clone()).into();
    let zero_from_borrowed = tonic::Response::new(&message).to_zero_copy();

    assert_eq!(zero_from_owned.bytes(), zero_from_borrowed.bytes());
}

#[test]
fn nested_zero_copy_roundtrip_preserves_inner_payload() {
    let base = ZeroCopyContainer::default();
    let inner = proto_rs::ZeroCopy::<ZeroCopyContainer>::from_message(base);
    let inner_expected = inner.bytes().to_vec();
    let inner_payload = proto_rs::ZeroCopy::<ZeroCopyContainer>::encode_to_vec(inner);

    let nested = proto_rs::ZeroCopy::<proto_rs::ZeroCopy<ZeroCopyContainer>>::from_bytes(inner_payload.clone());
    let expected_outer = nested.bytes().to_vec();

    let encoded = proto_rs::ZeroCopy::<proto_rs::ZeroCopy<ZeroCopyContainer>>::encode_to_vec(nested);
    let decoded = proto_rs::ZeroCopy::<proto_rs::ZeroCopy<ZeroCopyContainer>>::decode(Bytes::from(encoded)).expect("decode nested zero-copy payload");

    assert_eq!(decoded.bytes(), expected_outer.as_slice());

    let (_, _, inner_bytes) = decoded.into_parts();
    let inner_decoded = proto_rs::ZeroCopy::<ZeroCopyContainer>::decode(Bytes::from(inner_bytes.clone())).expect("decode inner payload");

    assert_eq!(inner_decoded.bytes(), inner_expected.as_slice());
}
