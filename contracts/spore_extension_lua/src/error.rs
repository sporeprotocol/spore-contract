use ckb_std::syscalls::SysError;
use spore_errors::error::Error;

pub enum WrappedError {
    SystemError(Error),
    LuaError(i8),
}

impl From<WrappedError> for i8 {
    fn from(value: WrappedError) -> Self {
        match value {
            WrappedError::SystemError(error) => error as i8,
            WrappedError::LuaError(error) => error,
        }
    }
}

impl From<Error> for WrappedError {
    fn from(value: Error) -> Self {
        Self::SystemError(value)
    }
}

impl From<SysError> for WrappedError {
    fn from(value: SysError) -> Self {
        Self::SystemError(value.into())
    }
}
