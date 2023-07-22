use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing = 2,
    LengthNotEnough = 3,
    Encoding = 4,
    ModifyPermanentField = 5,
    EmptyName = 6,
    InvalidClusterID = 7,
    InvalidOperation = 8,
    InvalidClusterData = 11,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            ModifyPermanentField => Self::ModifyPermanentField,
            EmptyName => Self::EmptyName,
            InvalidClusterID => Self::InvalidClusterID,
            InvalidOperation => Self::InvalidOperation,
            InvalidClusterData => Self::InvalidClusterData,
            Unknown(err_code) => panic!("unexpected sys error {}", err_code),
        }
    }
}

