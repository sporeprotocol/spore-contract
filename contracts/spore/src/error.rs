use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing = 2,
    LengthNotEnough = 3,
    Encoding = 4,
    ModifyPermanentField = 5,
    InvalidNFTData = 6,
    InvalidNFTID = 7,
    InvalidContentType = 8, // failed to parse content-type
    DestroyImmortalNFT = 9, // cannot destroy an immortal cellular cell
    EmptyContent = 10, // content is empty
    ClusterCellNotInDep = 11,
    ClusterCellCanNotUnlock = 12,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            Unknown(err_code) => panic!("unexpected sys error {}", err_code),
        }
    }
}
