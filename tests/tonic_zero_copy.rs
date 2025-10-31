use proto_rs::ProtoExt;
use proto_rs::ToZeroCopyRequest;
use proto_rs::ToZeroCopyResponse;

mod encoding_messages;

use encoding_messages::SampleMessage;
use encoding_messages::sample_message;

#[test]
fn zero_copy_request_preserves_metadata() {
    let message = sample_message();
    let mut request = tonic::Request::new(message.clone());
    request.metadata_mut().insert("x-trace", "abc123".parse().expect("valid metadata value"));
    request.extensions_mut().insert::<usize>(99);

    let expected_bytes = SampleMessage::encode_to_vec(&message);

    let zero_copy: proto_rs::ZeroCopyRequest<_> = request.into();
    let inner = zero_copy.as_request();

    assert_eq!(inner.get_ref(), &expected_bytes);
    assert_eq!(inner.metadata().get("x-trace").unwrap(), "abc123");
    assert_eq!(*inner.extensions().get::<usize>().unwrap(), 99);

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

    let zero_copy: proto_rs::ZeroCopyResponse<_> = response.into();
    let inner = zero_copy.as_response();

    assert_eq!(inner.get_ref(), &expected_bytes);
    assert_eq!(inner.metadata().get("x-resp").unwrap(), "value");
    assert_eq!(inner.extensions().get::<String>().unwrap(), "ext");

    let back_to_response: tonic::Response<Vec<u8>> = zero_copy.into();
    assert_eq!(back_to_response.get_ref(), &expected_bytes);
}

#[test]
fn borrowed_request_zero_copy_matches_manual_encoding() {
    let message = sample_message();
    let zero_copy = tonic::Request::new(&message).to_zero_copy();
    let expected = SampleMessage::encode_to_vec(&message);

    assert_eq!(zero_copy.as_request().get_ref(), &expected);
}

#[test]
fn borrowed_response_zero_copy_matches_manual_encoding() {
    let message = sample_message();
    let zero_copy = tonic::Response::new(&message).to_zero_copy();
    let expected = SampleMessage::encode_to_vec(&message);

    assert_eq!(zero_copy.as_response().get_ref(), &expected);
}

#[test]
fn zero_copy_conversion_roundtrip_maintains_bytes_identity() {
    let mut request = tonic::Request::new(sample_message());
    request.metadata_mut().insert("id", "42".parse().unwrap());

    let zero_from_owned: proto_rs::ZeroCopyRequest<_> = request.into();
    let zero_from_borrowed = tonic::Request::new(&sample_message()).to_zero_copy();

    assert_eq!(zero_from_owned.as_request().get_ref(), zero_from_borrowed.as_request().get_ref());
}

#[test]
fn zero_copy_response_roundtrip_maintains_bytes_identity() {
    let message = sample_message();
    let zero_from_owned: proto_rs::ZeroCopyResponse<_> = tonic::Response::new(message.clone()).into();
    let zero_from_borrowed = tonic::Response::new(&message).to_zero_copy();

    assert_eq!(zero_from_owned.as_response().get_ref(), zero_from_borrowed.as_response().get_ref());
}
