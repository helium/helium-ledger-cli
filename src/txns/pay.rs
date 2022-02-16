use super::*;
use crate::memo::Memo;
use std::str::FromStr;

#[derive(Debug, StructOpt)]
pub struct Cmd {
    /// Address to send the tokens to
    address: PublicKey,
    /// Amount of HNT to send
    amount: Hnt,
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

        match ledger(opts, self).await? {
            Response::Txn(_txn, hash, network) => Ok(Some((hash, network))),
            Response::InsufficientBalance(balance, send_request) => {
                println!(
                    "Account balance insufficient. {} HNT on account but attempting to send {}",
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

async fn ledger(opts: Opts, cmd: Cmd) -> Result<Response<BlockchainTxnPaymentV2>> {
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

    if account.balance.get_decimal() < amount.get_decimal() {
        return Ok(Response::InsufficientBalance(account.balance, amount));
    }
    // serialize payer
    let payer = PublicKey::from_str(&account.address)?;

    let payment = Payment {
        payee: payee.to_vec(),
        amount: u64::from(amount),
        memo: u64::from(&cmd.memo),
    };

    let mut txn = BlockchainTxnPaymentV2 {
        payer: payer.to_vec(),
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

    print_proposed_txn(&txn)?;

    let adpu_cmd = txn.apdu_serialize(opts.account)?;

    let exchange_pay_tx_result = read_from_ledger(&ledger_transport, adpu_cmd).await?;

    if exchange_pay_tx_result.data.len() == 1 {
        return Ok(Response::UserDeniedTransaction);
    }

    let txn = BlockchainTxnPaymentV2::decode(exchange_pay_tx_result.data.as_slice())?;
    let payer = PublicKey::from_bytes(&txn.payer)?;

    println!("{}", payer.to_string());
    let envelope = txn.in_envelope();
    // submit the signed tansaction to the API
    let pending_txn_status = submit_txn(&client, &envelope).await?;

    Ok(Response::Txn(txn, pending_txn_status.hash, payer.network))
}

pub fn print_proposed_txn(txn: &BlockchainTxnPaymentV2) -> Result {
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
        "Memo",
        "Nonce",
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
