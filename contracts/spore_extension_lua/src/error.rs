use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    InternalError = -1,
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,
    ModifyPermanentField = 5,
    ConflictCreation = 6,
    MultipleSpend = 7,
    InvalidOperation = 8,
    InvalidExtensionID = 9,
    InvalidExtensionArg = 10,
    InvalidLuaScript = 11,
    InvalidLuaLib = 12,
    FailedToLoadLuaLib = 13,
    FailedToCreateLuaInstance = 14,
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

