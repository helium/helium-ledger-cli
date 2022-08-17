use crate::*;
use byteorder::{LittleEndian as LE, WriteBytesExt};
use helium_api::{
    accounts,
    models::{Hnt, Hst, Iot, Mobile, Token},
};
pub use helium_proto::{
    BlockchainTxnPaymentV1, BlockchainTxnPaymentV2, BlockchainTxnSecurityExchangeV1,
    BlockchainTxnStakeValidatorV1, BlockchainTxnTokenBurnV1, BlockchainTxnTransferValidatorStakeV1,
    BlockchainTxnUnstakeValidatorV1, Payment,
};
pub use helium_wallet::{
    keypair::{Network, PublicKey},
    traits::{TxnEnvelope, TxnFee, TxnFeeConfig},
};
pub use ledger_transport::*;
pub use prost::Message;
use std::convert::TryFrom;

pub mod balance;
pub mod burn;
pub mod pay;
pub mod serializer;
pub mod validator;

pub use serializer::*;

const RETURN_CODE_OK: u16 = 0x9000;

// This parameter indicates whether the ledgers screen display the public key or not
// Thus, the `pay` function can do the Adpu transaction quietly to get the public key
#[derive(Copy, Clone)]
pub enum PubkeyDisplay {
    Off = 0,
    On = 1,
}

pub async fn get_ledger_transport(opts: &Opts) -> Result<Box<dyn LedgerTransport>> {
    Ok(if let Some(port) = opts.emulator {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
        Box::new(TransportTcp::new(socket).await?)
    } else {
        Box::new(TransportNativeHID::new()?)
    })
}

pub async fn get_app_version(opts: &Opts) -> Result<Version> {
    let ledger = get_ledger_transport(opts).await?;
    let request = VersionRequest.apdu_serialize(0)?;
    let read = read_from_ledger(&ledger, request).await?;
    let data = read.data;
    if read.retcode == RETURN_CODE_OK {
        if data.len() == 4 {
            Ok(Version::from_bytes([data[0], data[1], data[2], data[3]])?)
        } else {
            Err(Error::VersionError(format!(
                "Your ledger application may not be running or require an update. \
            Unexpected response from ledger ({} bytes).",
                data.len()
            )))
        }
    } else {
        Err(Error::VersionError(
            "App unresponsive. Is it waiting for a command?".to_string(),
        ))
    }
}
#[allow(clippy::borrowed_box)]
pub async fn get_pubkey(
    account: u8,
    ledger: &Box<dyn LedgerTransport>,
    display: PubkeyDisplay,
) -> Result<PublicKey> {
    let cmd = PubkeyRequest { display }.apdu_serialize(account)?;
    let public_key_result = read_from_ledger(ledger, cmd).await?;
    Ok(PublicKey::try_from(&public_key_result.data[1..34])?)
}

pub enum Response<T> {
    Txn(T, String, Network),
    InsufficientHntBalance(Hnt, Hnt),
    InsufficientIotBalance(Iot, Iot),
    InsufficientMobBalance(Mobile, Mobile),
    InsufficientHstBalance(Hst, Hst),
    InsufficientSecBalance(Hst, Hst),
    /// to deprecate when deprecating sec
    UserDeniedTransaction,
}

#[allow(clippy::borrowed_box)]
pub async fn read_from_ledger(
    ledger: &Box<dyn LedgerTransport>,
    command: APDUCommand,
) -> Result<APDUAnswer> {
    let answer = ledger.exchange(&command).await?;

    if answer.data.is_empty() {
        Err(Error::AppNotRunning)
    } else {
        Ok(answer)
    }
}

pub async fn get_txn_fees(client: &Client) -> Result<TxnFeeConfig> {
    let vars = helium_api::vars::get(client).await?;
    if vars.contains_key("txn_fees") {
        match vars["txn_fees"].as_bool() {
            Some(true) => {
                let config: TxnFeeConfig = serde_json::from_value(serde_json::Value::Object(vars))?;
                Ok(config)
            }
            _ => Ok(TxnFeeConfig::legacy()),
        }
    } else {
        Ok(TxnFeeConfig::legacy())
    }
}
