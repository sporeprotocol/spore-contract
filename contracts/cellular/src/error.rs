use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,
    InvalidCell, // not a valid cellular cell
    InvalidCellularData,
    DestroyImmortalCellular, // cannot destroy an immortal cellular cell
    ConflictDualOperation, // try to create and destroy cellular cell at the same time
    InsufficientCapacity, // no enough capacity as input
    EmptyContent, // content is empty
    SeriesNotInDep,
    LockedNFT,
    InvalidUpdate,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            InvalidCell => Self::InvalidCell,
            InvalidCellularData => Self::InvalidCellularData,
            DestroyImmortalCellular => Self::DestroyImmortalCellular,
            ConflictDualOperation => Self::ConflictDualOperation,
            EmptyContent => Self::EmptyContent,
            SeriesNotInDep => Self::SeriesNotInDep,
            LockedNFT => Self::LockedNFT,
            Unknown(err_code) => panic!("unexpected sys error {}", err_code),
        }
    }
}
