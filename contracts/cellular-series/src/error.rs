use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,
    InvalidTypesArg,
    InvalidSeriesData,
    SeriesCellCountError,
    EmptyName, // name can not be empty
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            InvalidTypesArg => Self::InvalidTypesArg,
            SeriesCellCountError => Self::SeriesCellCountError,
            EmptyName => Self::EmptyName,
            Unknown(err_code) => panic!("unexpected sys error {}", err_code),
        }
    }
}

