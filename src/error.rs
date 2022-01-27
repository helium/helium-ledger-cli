use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not find ledger. Is it disconnected or locked?")]
    CouldNotFindLedger(#[from] ledger_transport::errors::TransportError),
    #[error("Ledger is connected but Helium application does not appear to be running")]
    AppNotRunning,
    #[error("Error getting version. App must be waiting for a command.")]
    VersionError,
    #[error("Error generating QR")]
    Qr(#[from] qr2term::QrError),
    #[error("Error accessing Ledger HID Device. Be sure that Ledger Live is not running.")]
    Hid(#[from] ledger_transport::LedgerHIDError),
    #[error("Connection refused by Ledger emulator")]
    Tcp(#[from] ledger_transport::TransportTcpError),
    #[error("Helium API Error")]
    HeliumApi(#[from] helium_api::Error),
    #[error("Helium Crypto Error")]
    HeliumCrypto(#[from] helium_crypto::Error),

    #[error("Getting Fees")]
    GettingFees,
    #[error("Io Error")]
    Io(#[from] std::io::Error),
    #[error("Decoding Error")]
    Decode(#[from] prost::DecodeError),
    #[error("Encoding Error")]
    Encode(#[from] prost::EncodeError),
    #[error("Transaction Error")]
    Txn,
    #[error("Into Envelope Error")]
    IntoEnvelope,
    #[error("FromB64 Error")]
    FromB64,
    #[error("Decode Base64 Error")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("From Json Parsing Error")]
    SerdeJson(#[from] serde_json::Error),
}

impl Error {
    pub fn getting_fees() -> Error {
        Error::GettingFees
    }
    pub fn txn() -> Error {
        Error::Txn
    }
    pub fn into_envelope() -> Error {
        Error::IntoEnvelope
    }
    pub fn from_b64() -> Error {
        Error::FromB64
    }
}
