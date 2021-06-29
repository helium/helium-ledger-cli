use crate::txns::*;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, StructOpt)]
/// Onboard one (or more) validators  with this wallet.
///
/// The payment is not submitted to the system unless the '--commit' option is
/// given.
///
/// Note that multiple staking transactions are submitted individually and not as a
/// single transaction. Any failures will abort the remaining staking entries.
pub enum Cmd {
    /// Stake a single validator
    One(Validator),
    /// Stake multiple validators via file import
    Multi(Multi),
}

#[derive(Debug, StructOpt)]
/// The input file for multiple validator stakes is expected to be json file
/// with a list of address and staking amounts. For example:
///
/// [
///     {
///         "address": "<adddress1>",
///         "stake": 10000
///     },
///     {
///         "address": "<adddress2>",
///         "stake": 10000
///     }
/// ]
pub struct Multi {
    /// File to read multiple stakes from
    path: PathBuf,
}

pub enum Response {
    Success,
    InsufficientBalance(Hnt, Hnt), // provides balance and send request
    UserDeniedTransaction,
}

impl Cmd {
    pub(crate) async fn run(
        self,
        opts: Opts,
        _version: Version,
    ) -> Result<Option<(String, Network)>> {
        match self.ledger(opts).await? {
            Response::Success => Ok(None),
            Response::InsufficientBalance(balance, send_request) => {
                println!(
                    "Account balance insufficient. {} HNT on account but attempting to stake {}",
                    balance, send_request,
                );
                Err(Error::txn())
            }
            Response::UserDeniedTransaction => {
                println!("Transaction not confirmed");
                Err(Error::txn())
            }
        }
    }

    fn collect_validators(&self) -> Result<Vec<Validator>> {
        match &self {
            Self::One(validator) => Ok(vec![validator.clone()]),
            Self::Multi(multi) => {
                let file = std::fs::File::open(multi.path.clone())?;
                let validators: Vec<Validator> = serde_json::from_reader(file)?;
                Ok(validators)
            }
        }
    }

    pub(crate) async fn ledger(self, opts: Opts) -> Result<Response> {
        let validators = self.collect_validators()?;

        let ledger_transport = get_ledger_transport(&opts).await?;

        // get account from API so we can get nonce and balance
        let owner = get_pubkey(opts.account, &ledger_transport, PubkeyDisplay::Off).await?;

        let client = Client::new_with_base_url(api_url(owner.network));

        let account = accounts::get(&client, &owner.to_string()).await?;

        let total_stake_amount = validators
            .iter()
            .map(|v| v.stake.get_decimal())
            .sum::<Decimal>();

        if account.balance.get_decimal() < total_stake_amount {
            return Ok(Response::InsufficientBalance(
                account.balance,
                Hnt::new(total_stake_amount),
            ));
        }

        for validator in validators {
            let mut txn = BlockchainTxnStakeValidatorV1 {
                owner: owner.to_vec(),
                address: validator.address.to_vec(),
                stake: u64::from(validator.stake),
                fee: 0,
                owner_signature: vec![],
            };
            txn.fee = txn
                .txn_fee(
                    &get_txn_fees(&client)
                        .await
                        .map_err(|_| Error::getting_fees())?,
                )
                .map_err(|_| Error::getting_fees())?;
            print_proposed_transaction(&txn)?;

            let cmd = txn.apdu_serialize(opts.account)?;
            let exchange_pay_tx_result = read_from_ledger(&ledger_transport, cmd).await?;

            if exchange_pay_tx_result.data.len() == 1 {
                return Ok(Response::UserDeniedTransaction);
            }

            let txn =
                BlockchainTxnStakeValidatorV1::decode(exchange_pay_tx_result.data.as_slice())?;
            let envelope = txn.in_envelope();
            // submit the signed tansaction to the API
            let pending_txn_status = submit_txn(&client, &envelope).await?;

            print_txn(pending_txn_status.hash, owner.network)
        }
        Ok(Response::Success)
    }
}

fn print_proposed_transaction(stake: &BlockchainTxnStakeValidatorV1) -> Result {
    let address = PublicKey::try_from(stake.address.clone())?;
    let units = match address.network {
        Network::TestNet => "TNT",
        Network::MainNet => "HNT",
    };

    let mut table = Table::new();
    println!("Creating the following stake transaction:");
    table.add_row(row![
        &format!("Stake Amount {}", units),
        "Validator Address",
        "DC Fee"
    ]);
    table.add_row(row![Hnt::from(stake.stake), address, stake.fee]);
    table.printstd();
    println!(
        "WARNING: do not use this output as the source of truth. Instead, rely \
    on the Ledger Display"
    );
    Ok(())
}

#[derive(Debug, Deserialize, StructOpt, Clone)]
pub struct Validator {
    /// The validator address to stake
    address: PublicKey,
    /// The amount of HNT to stake
    stake: Hnt,
}
