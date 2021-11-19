#[macro_use]
extern crate prettytable;

use helium_api::models::{transactions::PendingTxnStatus, Hnt};
use helium_proto::BlockchainTxn;
use helium_wallet::keypair::Network;
use ledger_transport::exchange::Exchange as LedgerTransport;
use qr2term::print_qr;
use std::{env, fmt, process};
use structopt::StructOpt;
mod error;
mod memo;
mod txns;

pub use error::Error;
pub type Result<T = ()> = std::result::Result<T, Error>;

const DEFAULT_TESTNET_BASE_URL: &str = "https://testnet-api.helium.wtf/v1";
pub(crate) static USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(StructOpt, Debug)]
enum Units {
    /// Pay using Bones as units (must be integer)
    Bones { bones: u64 },
    /// Pay using HNT as units (up to 8 decimals are tolerated)
    Hnt { hnt: Hnt },
}

/// Common options for most wallet commands
#[derive(Debug, StructOpt)]
pub struct Opts {
    /// Select account index to stake from
    #[structopt(long = "account", default_value = "0")]
    pub account: u8,

    /// Enable interaction with emulator for development and testing
    /// by configuring port for TCP connection here (typically 9999
    /// or 40000)
    #[structopt(long = "emulator")]
    pub emulator: Option<u16>,
}

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(flatten)]
    opts: Opts,

    #[structopt(flatten)]
    cmd: Cmd,
}

/// Interact with Ledger Nano S for hardware wallet management
#[derive(Debug, StructOpt)]
#[allow(clippy::large_enum_variant)]
enum Cmd {
    /// Get wallet information
    Balance(txns::balance::Cmd),
    /// Burn to given address.
    Burn(txns::burn::Cmd),
    /// Pay a given address.
    Pay(txns::pay::Cmd),
    /// Stake a validator
    Validators(txns::validator::Cmd),
}

#[tokio::main]
async fn main() {
    println!("Communicating with Ledger - follow prompts on screen");

    let cli = Cli::from_args();
    if let Err(e) = run(cli).await {
        println!("error: {}", e);
        process::exit(1);
    }
}

pub(crate) struct Version {
    major: u8,
    minor: u8,
    revision: u8,
}

impl Version {
    pub(crate) fn from_bytes(bytes: [u8; 3]) -> Version {
        Version {
            major: bytes[0],
            minor: bytes[1],
            revision: bytes[2],
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.revision)
    }
}

async fn run(cli: Cli) -> Result {
    let version = txns::get_app_version(&cli.opts).await?;
    println!("Ledger running Helium App {}\r\n", version);

    let result = match cli.cmd {
        Cmd::Balance(balance) => balance.run(cli.opts, version).await?,
        Cmd::Burn(burn) => burn.run(cli.opts, version).await?,
        Cmd::Pay(pay) => pay.run(cli.opts, version).await?,
        Cmd::Validators(validator) => validator.run(cli.opts, version).await?,
    };
    if let Some((hash, network)) = result {
        print_txn(hash, network);
    }

    Ok(())
}

fn print_txn(hash: String, network: Network) {
    println!("\nSuccessfully submitted transaction to API:");

    let mut table = Table::new();
    table.add_row(row!["Network", "Hash"]);
    table.add_row(row![network, hash]);
    table.printstd();

    println!("To check on transaction status, monitor the following URL:");
    println!("     {}/pending_transactions/{}", api_url(network), hash);
}

use helium_api::Client;
use prettytable::{format, Table};

pub async fn submit_txn(client: &Client, txn: &BlockchainTxn) -> Result<PendingTxnStatus> {
    use helium_proto::Message;
    let mut data = vec![];
    txn.encode(&mut data)?;
    helium_api::pending_transactions::submit(client, &data)
        .await
        .map_err(|e| e.into())
}

fn new_client(network: Network) -> Client {
    println!("{}", USER_AGENT);
    Client::new_with_base_url(api_url(network), USER_AGENT)
}

fn api_url(network: Network) -> String {
    match network {
        Network::MainNet => {
            env::var("HELIUM_API_URL").unwrap_or_else(|_| helium_api::DEFAULT_BASE_URL.to_string())
        }
        Network::TestNet => env::var("HELIUM_TESTNET_API_URL")
            .unwrap_or_else(|_| DEFAULT_TESTNET_BASE_URL.to_string()),
    }
}
