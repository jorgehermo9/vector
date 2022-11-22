use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use bytes::{Bytes, BytesMut};
use codecs::{
    decoding::format::Deserializer, encoding::format::Serializer, NativeDeserializerConfig,
    NativeJsonDeserializerConfig, NativeJsonSerializerConfig, NativeSerializerConfig,
};
use similar_asserts::assert_eq;
use vector_core::{config::LogNamespace, event::Event};

#[test]
fn pre_v24_fixtures_match() {
    fixtures_match("pre-v24");
}

#[test]
fn current_fixtures_match() {
    fixtures_match("");
}

#[test]
fn roundtrip_current_native_json_fixtures() {
    roundtrip_fixtures(
        "json",
        "",
        &NativeJsonDeserializerConfig.build(),
        &mut NativeJsonSerializerConfig.build(),
        false,
    );
}

#[test]
fn roundtrip_current_native_proto_fixtures() {
    roundtrip_fixtures(
        "proto",
        "",
        &NativeDeserializerConfig.build(),
        &mut NativeSerializerConfig.build(),
        false,
    );
}

/// The event proto file was changed in v0.24. This test ensures we can still load the old version
/// binary and that when serialized and deserialized in the new format we still get the same event.
#[test]
fn reserialize_pre_v24_native_json_fixtures() {
    roundtrip_fixtures(
        "json",
        "pre-v24",
        &NativeJsonDeserializerConfig.build(),
        &mut NativeJsonSerializerConfig.build(),
        true,
    );
}

#[test]
fn reserialize_pre_v24_native_proto_fixtures() {
    roundtrip_fixtures(
        "proto",
        "pre-v24",
        &NativeDeserializerConfig.build(),
        &mut NativeSerializerConfig.build(),
        true,
    );
}

#[test]
fn current_native_decoding_matches() {
    decoding_matches("");
}

#[test]
fn pre_v24_native_decoding_matches() {
    decoding_matches("pre-v24");
}

/// This test ensures that the different sets of protocol fixture names match.
fn fixtures_match(suffix: &str) {
    let json_entries = list_fixtures("json", suffix);
    let proto_entries = list_fixtures("proto", suffix);
    for (json_path, proto_path) in json_entries.into_iter().zip(proto_entries.into_iter()) {
        // Make sure we're looking at the matching files for each format
        assert_eq!(
            json_path.file_stem().unwrap(),
            proto_path.file_stem().unwrap(),
        );
    }
}

/// This test ensures we can load the serialized binaries binary and that they match across
/// protocols.
fn decoding_matches(suffix: &str) {
    let json_deserializer = NativeJsonDeserializerConfig.build();
    let proto_deserializer = NativeDeserializerConfig.build();

    let json_entries = list_fixtures("json", suffix);
    let proto_entries = list_fixtures("proto", suffix);

    for (json_path, proto_path) in json_entries.into_iter().zip(proto_entries.into_iter()) {
        let (_, json_event) = load_deserialize(&json_path, &json_deserializer);

        let (_, proto_event) = load_deserialize(&proto_path, &proto_deserializer);

        // Ensure that the json version and proto versions were parsed into equivalent
        // native representations
        assert_eq!(
            json_event,
            proto_event,
            "Parsed events don't match: {} {}",
            json_path.display(),
            proto_path.display()
        );
    }
}

fn list_fixtures(proto: &str, suffix: &str) -> Vec<PathBuf> {
    let path = format!("tests/data/native_encoding/{proto}/{suffix}");
    let mut entries = fs::read_dir(path)
        .unwrap()
        .map(Result::unwrap)
        .filter(|e| e.file_type().unwrap().is_file())
        .map(|e| e.path())
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

fn roundtrip_fixtures(
    proto: &str,
    suffix: &str,
    deserializer: &dyn Deserializer,
    serializer: &mut dyn Serializer,
    reserialize: bool,
) {
    for path in list_fixtures(proto, suffix) {
        let (buf, event) = load_deserialize(&path, deserializer);

        if reserialize {
            // Serialize the parsed event
            let mut buf = BytesMut::new();
            serializer.encode(event.clone(), &mut buf).unwrap();
            // Deserialize the event from these bytes
            let new_events = deserializer
                .parse(buf.into(), LogNamespace::Legacy)
                .unwrap();

            // Ensure we have the same event.
            assert_eq!(new_events.len(), 1);
            assert_eq!(new_events[0], event);
        } else {
            // Ensure that the parsed event is serialized to the same bytes
            let mut new_buf = BytesMut::new();
            serializer.encode(event.clone(), &mut new_buf).unwrap();
            assert_eq!(buf, new_buf);
        }
    }
}

fn load_deserialize(path: &Path, deserializer: &dyn Deserializer) -> (Bytes, Event) {
    let mut file = File::open(path).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    let buf = Bytes::from(buf);

    // Ensure that we can parse the json fixture successfully
    let mut events = deserializer
        .parse(buf.clone(), LogNamespace::Legacy)
        .unwrap();
    assert_eq!(events.len(), 1);
    (buf, events.pop().unwrap())
}
