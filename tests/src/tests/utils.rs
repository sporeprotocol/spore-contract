use spore_types::generated::spore;
use spore_utils::{compatible_load_cluster_data, MIME};

use molecule::prelude::*;

#[test]
fn test_mime_basic() {
    assert!(MIME::str_parse("image/png").is_ok());
    assert!(MIME::str_parse("image/png;immortal=true").is_ok());
    assert!(MIME::str_parse("image/").is_err());
    assert!(MIME::str_parse("image/;").is_err());
    assert!(MIME::str_parse("/;").is_err());
    assert!(MIME::str_parse(";").is_err());
    assert!(MIME::str_parse("").is_err());

    let content_type = "image/png;immortal=true;mutant[]=c219351b150b900e50a7039f1e448b844110927e5fd9bd30425806cb8ddff1fd,9c87faf08de5c15c727d5350399115431bf4f0226fbc4abd400e63492faac3d2";
    let mime = MIME::str_parse(content_type)
        .map_err(|err| format!("mutant str_parse: {}", err as u8))
        .unwrap();
    let expected_value = b"c219351b150b900e50a7039f1e448b844110927e5fd9bd30425806cb8ddff1fd,9c87faf08de5c15c727d5350399115431bf4f0226fbc4abd400e63492faac3d2";
    let value_range = mime
        .get_param(content_type.as_bytes(), "mutant[]")
        .map_err(|err| format!("mutant get_param: {}", err as u8))
        .unwrap()
        .expect("empty range");
    assert!(content_type.as_bytes()[value_range] == expected_value[..]);

    let expected_value = b"true";
    let value_range = mime
        .get_param(content_type.as_bytes(), "immortal")
        .map_err(|err| format!("mutant get_param: {}", err as u8))
        .unwrap()
        .expect("empty range");
    assert!(content_type.as_bytes()[value_range] == expected_value[..]);
}

#[test]
fn test_compatible_load_cluster_data() {
    // test ClusterDataV1 -> ClusterDataV2
    let cluster_data_v1 = spore::ClusterData::new_builder()
        .name("Test Cluster Name".as_bytes().into())
        .description("Test Cluster Description".as_bytes().into())
        .build();
    let raw_cluster_data = cluster_data_v1.as_slice();
    let cluster_data_v2 = compatible_load_cluster_data(raw_cluster_data)
        .map_err(|_| "compatible_load_cluster_data error")
        .expect("test ClusterDataV1 -> ClusterDataV2");
    assert_eq!(
        cluster_data_v2.name().unpack(),
        cluster_data_v1.name().unpack(),
    );
    assert_eq!(
        cluster_data_v2.description().as_slice(),
        cluster_data_v1.description().as_slice()
    );
    assert!(cluster_data_v2.mutant_id().is_none());

    // test ClusterDataV2 -> ClusterDataV2
    let cluster_data_v2_with_mutant_id = cluster_data_v2
        .as_builder()
        .mutant_id("mock mutant_id".as_bytes().into())
        .build();
    let raw_cluster_data = cluster_data_v2_with_mutant_id.as_slice();
    let cluster_data_v1 = spore::ClusterData::from_compatible_slice(raw_cluster_data)
        .map_err(|_| "spore::ClusterData::from_compatible_slice error")
        .expect("test old format -> new format");
    assert!(cluster_data_v1.has_extra_fields());
    assert_eq!(cluster_data_v1.field_count(), 3);
    assert_eq!(cluster_data_v1.count_extra_fields(), 1);
    let cluster_data_v2 = compatible_load_cluster_data(raw_cluster_data)
        .map_err(|_| "compatible_load_cluster_data error")
        .expect("test ClusterDataV2 -> ClusterDataV2");
    assert!(cluster_data_v2.mutant_id().is_some());
}
