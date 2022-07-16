use super::*;
use crate::memo::Memo;
use helium_api::models::Account;
use helium_proto::BlockchainTokenTypeV1;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub enum TokenInput {
    Hnt,
    Iot,
    Mobile,
    Hst,
}

#[derive(Debug, StructOpt)]
pub struct Cmd {
    /// Address to send the tokens to
    address: PublicKey,
    /// Amount of token to send
    amount: Token,
    /// Type of token to send (hnt, iot, mobile, hst).
    #[structopt(default_value = "hnt")]
    token: TokenInput,
    /// Memo field to include. Provide as a base64 encoded string
    #[structopt(long, default_value = "AAAAAAAAAAA=")]
    memo: Memo,
    /// Manually set the DC fee to pay for the transaction
    #[structopt(long)]
    fee: Option<u64>,
    /// Manually set the nonce for the transaction
    #[structopt(long)]
    nonce: Option<u64>,
}

impl Cmd {
    pub async fn run(self, opts: Opts, version: Version) -> Result<Option<(String, Network)>> {
        if version.major < 2 && opts.account != 0 {
            panic!("Upgrade the Helium Ledger App to use additional wallet accounts");
        };

        // versions before 2.2.3 are invalid
        if version.major < 2
            || version.major == 2
                && ((version.minor == 2 && version.revision < 3) || (version.minor < 2))
        {
            println!("WARNING: Helium Ledger application is outdated. Using payment_v1.");
            if self.memo.0 != 0 {
                panic!("Non-default memo provided. Update Helium Ledger application to use payment_v2 which includes memo.");
            }

            match ledger_v1(opts, self).await? {
                Response::Txn(_txn, hash, network) => Ok(Some((hash, network))),
                Response::InsufficientHntBalance(balance, send_request) => {
                    println!(
                        "Account balance insufficient. {} HNT on account but attempting to send {}",
                        balance, send_request,
                    );
                    Err(Error::txn())
                }
                Response::InsufficientIotBalance(balance, send_request) => {
                    println!(
                        "Account balance insufficient. {} IOT on account but attempting to send {}",
                        balance, send_request,
                    );
                    Err(Error::txn())
                }
                Response::InsufficientMobBalance(balance, send_request) => {
                    println!(
                        "Account balance insufficient. {} MOB on account but attempting to send {}",
                        balance, send_request,
                    );
                    Err(Error::txn())
                }
                Response::InsufficientHstBalance(balance, send_request) => {
                    println!(
                        "Account balance insufficient. {} HST on account but attempting to send {}",
                        balance, send_request,
                    );
                    Err(Error::txn())
                }
                Response::InsufficientSecBalance(balance, send_request) => {
                    println!(
                        "Account security balance insufficient. {} HST on account but attempting to send {}",
                        balance, send_request,
                    );
                    Err(Error::txn())
                }
                Response::UserDeniedTransaction => {
                    println!("Transaction not confirmed");
                    Err(Error::txn())
                }
            }
        } else {
            match ledger_v2(opts, self).await? {
                Response::Txn(_txn, hash, network) => Ok(Some((hash, network))),
                Response::InsufficientHntBalance(balance, send_request) => {
                    println!(
                        "Account balance insufficient. {} HNT on account but attempting to send {}",
                        balance, send_request,
                    );
                    Err(Error::txn())
                }
                Response::InsufficientIotBalance(balance, send_request) => {
                    println!(
                        "Account balance insufficient. {} IOT on account but attempting to send {}",
                        balance, send_request,
                    );
                    Err(Error::txn())
                }
                Response::InsufficientMobBalance(balance, send_request) => {
                    println!(
                        "Account balance insufficient. {} MOB on account but attempting to send {}",
                        balance, send_request,
                    );
                    Err(Error::txn())
                }
                Response::InsufficientHstBalance(balance, send_request) => {
                    println!(
                        "Account balance insufficient. {} HST on account but attempting to send {}",
                        balance, send_request,
                    );
                    Err(Error::txn())
                }
                Response::InsufficientSecBalance(balance, send_request) => {
                    println!(
                        "Account security balance insufficient. {} HST on account but attempting to send {}",
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
    }
}

async fn ledger_v2(opts: Opts, cmd: Cmd) -> Result<Response<BlockchainTxnPaymentV2>> {
    let ledger_transport = get_ledger_transport(&opts).await?;
    let amount = cmd.amount;
    let payee = cmd.address;

    // get nonce
    let pubkey = get_pubkey(opts.account, &ledger_transport, PubkeyDisplay::Off).await?;
    let client = new_client(pubkey.network);

    let account = accounts::get(&client, &pubkey.to_string()).await?;
    let nonce: u64 = if let Some(nonce) = cmd.nonce {
        nonce
    } else {
        account.speculative_nonce + 1
    };

    if let Some(response) = invalid_balance_response(&cmd.token, &account, amount) {
        return Ok(response);
    }

    let payment = Payment {
        payee: payee.to_vec(),
        amount: u64::from(amount),
        memo: u64::from(&cmd.memo),
        max: false,
        token_type: match cmd.token {
            TokenInput::Hnt => BlockchainTokenTypeV1::Hnt.into(),
            TokenInput::Hst => BlockchainTokenTypeV1::Hst.into(),
            TokenInput::Iot => BlockchainTokenTypeV1::Iot.into(),
            TokenInput::Mobile => BlockchainTokenTypeV1::Mobile.into(),
        },
    };

    let mut txn = BlockchainTxnPaymentV2 {
        payer: pubkey.to_vec(),
        payments: vec![payment],
        nonce,
        fee: 0,
        signature: vec![],
    };
    txn.fee = if let Some(fee) = cmd.fee {
        fee
    } else {
        txn.txn_fee(
            &get_txn_fees(&client)
                .await
                .map_err(|_| Error::getting_fees())?,
        )
        .map_err(|_| Error::getting_fees())?
    };

    print_proposed_txn_v2(&txn)?;

    let adpu_cmd = txn.apdu_serialize(opts.account)?;

    let exchange_pay_tx_result = read_from_ledger(&ledger_transport, adpu_cmd).await?;

    if exchange_pay_tx_result.data.len() == 1 {
        return Ok(Response::UserDeniedTransaction);
    }

    let txn = BlockchainTxnPaymentV2::decode(exchange_pay_tx_result.data.as_slice())?;
    let payer = PublicKey::from_bytes(&txn.payer)?;

    let envelope = txn.in_envelope();
    // submit the signed tansaction to the API
    let pending_txn_status = submit_txn(&client, &envelope).await?;

    Ok(Response::Txn(txn, pending_txn_status.hash, payer.network))
}

pub fn print_proposed_txn_v2(txn: &BlockchainTxnPaymentV2) -> Result {
    let payment = &txn.payments[0];
    let payee = PublicKey::try_from(payment.payee.clone())?;
    let units = match payee.network {
        Network::TestNet => "TNT",
        Network::MainNet => "HNT",
    };

    let mut table = Table::new();
    println!("Creating the following transaction:");
    table.add_row(row![
        "Payee",
        &format!("Pay Amount {}", units),
        "Nonce",
        "Memo",
        "DC Fee"
    ]);
    table.add_row(row![
        payee,
        Hnt::from(payment.amount),
        txn.nonce,
        Memo::from(payment.memo).to_string(),
        txn.fee
    ]);
    table.printstd();
    println!(
        "WARNING: do not use this output as the source of truth. Instead, rely \
    on the Ledger Display"
    );
    Ok(())
}

async fn ledger_v1(opts: Opts, cmd: Cmd) -> Result<Response<BlockchainTxnPaymentV1>> {
    let ledger_transport = get_ledger_transport(&opts).await?;
    let amount = cmd.amount;
    let payee = cmd.address;

    // get nonce
    let pubkey = get_pubkey(opts.account, &ledger_transport, PubkeyDisplay::Off).await?;
    let client = new_client(pubkey.network);

    let account = accounts::get(&client, &pubkey.to_string()).await?;
    let nonce: u64 = if let Some(nonce) = cmd.nonce {
        nonce
    } else {
        account.speculative_nonce + 1
    };

    if let Some(response) = invalid_balance_response(&cmd.token, &account, amount) {
        return Ok(response);
    }

    let mut txn = BlockchainTxnPaymentV1 {
        payee: payee.to_vec(),
        payer: pubkey.to_vec(),
        amount: u64::from(amount),
        nonce,
        fee: 0,
        signature: vec![],
    };
    txn.fee = if let Some(fee) = cmd.fee {
        fee
    } else {
        txn.txn_fee(
            &get_txn_fees(&client)
                .await
                .map_err(|_| Error::getting_fees())?,
        )
        .map_err(|_| Error::getting_fees())?
    };

    print_proposed_txn_v1(&txn)?;

    let adpu_cmd = txn.apdu_serialize(opts.account)?;

    let exchange_pay_tx_result = read_from_ledger(&ledger_transport, adpu_cmd).await?;

    if exchange_pay_tx_result.data.len() == 1 {
        return Ok(Response::UserDeniedTransaction);
    }

    let txn = BlockchainTxnPaymentV1::decode(exchange_pay_tx_result.data.as_slice())?;

    let envelope = txn.in_envelope();
    // submit the signed tansaction to the API
    let pending_txn_status = submit_txn(&client, &envelope).await?;

    Ok(Response::Txn(
        txn,
        pending_txn_status.hash,
        Network::TestNet,
    ))
}

pub fn print_proposed_txn_v1(txn: &BlockchainTxnPaymentV1) -> Result {
    let payee = PublicKey::try_from(txn.payee.clone())?;
    let units = match payee.network {
        Network::TestNet => "TNT",
        Network::MainNet => "HNT",
    };

    let mut table = Table::new();
    println!("Creating the following transaction:");
    table.add_row(row![
        "Payee",
        &format!("Pay Amount {}", units),
        "Nonce",
        "DC Fee"
    ]);
    table.add_row(row![payee, Hnt::from(txn.amount), txn.nonce, txn.fee]);
    table.printstd();
    println!(
        "WARNING: do not use this output as the source of truth. Instead, rely \
    on the Ledger Display"
    );
    Ok(())
}

impl FromStr for TokenInput {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.to_lowercase();
        match s.as_str() {
            "hnt" => Ok(TokenInput::Hnt),
            "iot" => Ok(TokenInput::Iot),
            "mobile" => Ok(TokenInput::Mobile),
            "hst" => Ok(TokenInput::Hst),
            _ => Err(Error::TokenTypeInput(s)),
        }
    }
}

fn invalid_balance_response<T>(
    token: &TokenInput,
    account: &Account,
    amount: Token,
) -> Option<Response<T>> {
    match token {
        TokenInput::Hnt => {
            if account.balance.get_decimal() < amount.get_decimal() {
                return Some(Response::InsufficientHntBalance(
                    account.balance,
                    Hnt::new(amount.get_decimal()),
                ));
            }
        }
        TokenInput::Hst => {
            if account.sec_balance.get_decimal() < amount.get_decimal() {
                return Some(Response::InsufficientHstBalance(
                    account.sec_balance,
                    Hst::new(amount.get_decimal()),
                ));
            }
        }
        TokenInput::Iot => {
            if account.iot_balance.get_decimal() < amount.get_decimal() {
                return Some(Response::InsufficientIotBalance(
                    account.iot_balance,
                    Iot::new(amount.get_decimal()),
                ));
            }
        }
        TokenInput::Mobile => {
            if account.mobile_balance.get_decimal() < amount.get_decimal() {
                return Some(Response::InsufficientMobBalance(
                    account.mobile_balance,
                    Mobile::new(amount.get_decimal()),
                ));
            }
        }
    };
    None
}
