use helium_ledger::*;

#[tokio::main]
async fn main() {
    println!("Communicating with Ledger - follow prompts on screen");

    let cli = Cli::from_args();
    if let Err(e) = run(cli).await {
        println!("error: {}", e);
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result {
    let version = cmd::get_app_version(&cli.opts).await?;
    println!("Ledger running Helium App {}\r\n", version);

    let result = match cli.cmd {
        Cmd::Balance(balance) => balance.run(cli.opts, version).await?,
        Cmd::Burn(burn) => burn.run(cli.opts, version).await?,
        Cmd::Pay(pay) => pay.run(cli.opts, version).await?,
        Cmd::Validators(validator) => validator.run(cli.opts, version).await?,
        Cmd::Securities(securities) => securities.run(cli.opts, version).await?,
    };
    if let Some((hash, network)) = result {
        print_txn(hash, network);
    }

    Ok(())
}
