use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,

    // common
    ClusterCellNotInDep,
    ClusterOwnershipVerifyFailed,

    // spore_extension_lua errors
    ModifyExtensionPermanentField = 10,
    ConflictExtensionCreation,
    ExtensionMultipleSpend,
    InvalidExtensionOperation,
    InvalidExtensionID,
    InvalidExtensionArg,
    InvalidLuaScript,
    InvalidLuaLib,
    InvalidLuaParameters,
    FailedToLoadLuaLib,
    FailedToCreateLuaInstance,

    // cluster_proxy errors
    InvalidProxyOperation = 30,
    ImmutableProxyFieldModification,
    InvalidProxyID,

    // cluster_agent errors
    InvalidAgentOperation = 40,
    ImmutableAgentFieldModification,
    InvalidAgentArgs,
    ProxyCellNotInDep,
    PaymentNotEnough,
    PaymentMethodNotSupport,
    RefCellNotClusterProxy,

    // cluster errors
    InvalidClusterOperation = 50,
    ModifyClusterPermanentField,
    EmptyName,
    InvalidClusterID,
    InvalidClusterData,
    MutantNotInDeps,

    // spore errors
    ModifySporePermanentField = 60,
    InvalidNFTData,
    InvalidNFTID,
    InvalidContentType, // failed to parse content-type
    DestroyImmortalNFT, // cannot destroy an immortal cellular cell
    EmptyContent,       // content is empty
    ConflictCreation,
    MultipleSpend,
    InvalidMultipartContent,
    MIMEParsingError,
    ExtensionCellNotInDep,
    ExtensionPaymentNotEnough,
    ClusterRequiresMutantApplied,

    // mime errors
    Illformed = 80,
    InvaliMainType,
    InvalidSubType,
    InvalidParams,
    InvalidParamValue,
    MutantIDNotValid,

    Unknown,
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
