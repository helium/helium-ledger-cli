use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not find ledger. Is it disconnected or locked? {0}")]
    CouldNotFindLedger(#[from] ledger_transport::errors::TransportError),
    #[error("Ledger is connected but Helium application does not appear to be running")]
    AppNotRunning,
    #[error("Error getting version: {0}")]
    VersionError(String),
    #[error("Error generating QR {0}")]
    Qr(#[from] qr2term::QrError),
    #[error("Error accessing Ledger HID Device. Be sure that Ledger Live is not running. {0}")]
    Hid(#[from] ledger_transport::LedgerHIDError),
    #[error("Connection refused by Ledger emulator {0}")]
    Tcp(#[from] ledger_transport::TransportTcpError),
    #[error("Helium API Error {0}")]
    HeliumApi(#[from] helium_api::Error),
    #[error("Helium Crypto Error {0}")]
    HeliumCrypto(#[from] helium_crypto::Error),
    #[error("Getting Fees")]
    GettingFees,
    #[error("Io Error {0}")]
    Io(#[from] std::io::Error),
    #[error("Decoding Error {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("Encoding Error {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("Transaction Error")]
    Txn,
    #[error("Into Envelope Error")]
    IntoEnvelope,
    #[error("FromB64 Error")]
    FromB64,
    #[error("Decode Base64 Error {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("From Json Parsing Error {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Invalid token type input: {0}")]
    TokenTypeInput(String),
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
