use spore_utils::MIME;

#[test]
fn test_mime_basic() {
    assert!(MIME::str_parse("image/png").is_ok());
    assert!(MIME::str_parse("image/png;immortal=true").is_ok());
    assert!(MIME::str_parse("image/png;immortal=true;mutant[]=c219351b150b900e50a7039f1e448b844110927e5fd9bd30425806cb8ddff1fd,9c87faf08de5c15c727d5350399115431bf4f0226fbc4abd400e63492faac3d2")
        .map_err(|err| format!("mutant verify_param: {}", err as u8))
        .unwrap()
        .verify_param(
            b"image/png;immortal=true;mutant[]=c219351b150b900e50a7039f1e448b844110927e5fd9bd30425806cb8ddff1fd,9c87faf08de5c15c727d5350399115431bf4f0226fbc4abd400e63492faac3d2",
            "mutant[]",
            b"c219351b150b900e50a7039f1e448b844110927e5fd9bd30425806cb8ddff1fd,9c87faf08de5c15c727d5350399115431bf4f0226fbc4abd400e63492faac3d2"
        )
        .map_err(|_| "mutant verify_param")
        .unwrap());
    assert!(MIME::str_parse("image/png;immortal=true;mutant[]=c219351b150b900e50a7039f1e448b844110927e5fd9bd30425806cb8ddff1fd")
        .map_err(|err| format!("mutant verify_param: {}", err as u8))
        .unwrap()
        .verify_param(
            b"image/png;immortal=true;mutant[]=c219351b150b900e50a7039f1e448b844110927e5fd9bd30425806cb8ddff1fd",
            "immortal",
            b"true"
        )
        .map_err(|_| "mutant verify_param")
        .unwrap());
    assert!(MIME::str_parse("image/").is_err());
    assert!(MIME::str_parse("image/;").is_err());
    assert!(MIME::str_parse("/;").is_err());
    assert!(MIME::str_parse(";").is_err());
    assert!(MIME::str_parse("").is_err());
}
