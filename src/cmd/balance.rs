use super::*;
use helium_api::models::Account;

#[derive(Debug, StructOpt)]
pub struct Cmd {
    /// Display QR code for a given single wallet.
    #[structopt(long = "qr")]
    pub qr_code: bool,
    /// Scans all accounts up until selected account index
    /// This is useful for displaying all balances
    #[structopt(long = "scan")]
    pub scan: bool,
}

impl Cmd {
    pub async fn run(self, opts: Opts, version: Version) -> Result<Option<(String, Network)>> {
        if version.major < 2 && opts.account != 0 {
            panic!("Upgrade the Helium Ledger App to use additional wallet accounts");
        };
        let ledger_transport = get_ledger_transport(&opts).await?;
        if self.scan {
            if self.qr_code {
                println!("WARNING: to output a QR Code, do not use scan")
            }
            let mut account_results = Vec::new();
            let network = version.network;
            for i in 0..opts.account {
                let pubkey = get_pubkey(i, &ledger_transport, PubkeyDisplay::Off).await?;
                let client = new_client(pubkey.network);
                let address = pubkey.to_string();
                let result = accounts::get(&client, &address).await;
                account_results.push((pubkey, result));
            }
            print_balance(network, &account_results).await?;
        } else {
            let pubkey = get_pubkey(opts.account, &ledger_transport, PubkeyDisplay::Off).await?;
            let pubkey_str = pubkey.to_string();
            let client = new_client(pubkey.network);
            let address = pubkey.to_string();
            let result = accounts::get(&client, &address).await;
            print_balance(pubkey.network, &vec![(pubkey, result)]).await?;
            if self.qr_code {
                print_qr(&pubkey_str)?;
            }
            // display pubkey on screen for comparison
            let _pubkey = get_pubkey(opts.account, &ledger_transport, PubkeyDisplay::On).await?;
        }
        Ok(None)
    }
}

/// The ResultsVec is used so that a failure made "at some point" while
/// fetching all of the addresses does not ruin all previous or preceding
/// addresses
type ResultsVec = Vec<(PublicKey, std::result::Result<Account, helium_api::Error>)>;

async fn print_balance(network: Network, results: &ResultsVec) -> Result {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    let balance = match network {
        Network::TestNet => "Balance TNT",
        Network::MainNet => "Balance HNT",
    };

    let staked_balance = match network {
        Network::TestNet => "Staked TNT",
        Network::MainNet => "Staked HNT",
    };

    if results.len() > 1 {
        table.set_titles(row![
            "Index",
            "Wallet",
            balance,
            staked_balance,
            "Data Credits",
            "Security Tokens"
        ]);
    } else {
        table.set_titles(row![
            "Wallet 0",
            balance,
            staked_balance,
            "Data Credits",
            "Security Tokens"
        ]);
    }
    for (account_index, (pubkey, result)) in results.iter().enumerate() {
        let address = pubkey.to_string();
        if results.len() > 1 {
            match result {
                Ok(account) => table.add_row(row![
                    account_index,
                    address,
                    account.balance,
                    account.staked_balance,
                    account.dc_balance,
                    account.sec_balance
                ]),
                Err(err) => table.add_row(row![account_index, address, H3 -> err.to_string()]),
            };
        } else {
            match result {
                Ok(account) => table.add_row(row![
                    address,
                    account.balance,
                    account.staked_balance,
                    account.dc_balance,
                    account.sec_balance
                ]),
                Err(err) => table.add_row(row![address, H3 -> err.to_string()]),
            };
        }
    }

    table.printstd();
    Ok(())
}
