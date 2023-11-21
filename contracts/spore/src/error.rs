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
    EmptyContent = 10,      // content is empty
    ClusterCellNotInDep = 11,
    ClusterOwnershipVerifyFailed = 12,
    ConflictCreation = 13,
    MultipleSpend = 14,
    InvalidMultipartContent = 15,
    MIMEParsingError = 16,
    ExtensionCellNotInDep = 17,
    ExtensionPaymentNotEnough = 18,
    ClusterRequiresMutantApplied = 19,
    Unknown = 100,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            Unknown(code) if code >= 100 && code <= 104 =>  Self::MIMEParsingError ,
            _ => Self::Unknown
        }
    }
}
