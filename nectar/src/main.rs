#![warn(
    unused_extern_crates,
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::fallible_impl_from,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::dbg_macro
)]
#![cfg_attr(not(test), warn(clippy::unwrap_used))]
#![forbid(unsafe_code)]
#![recursion_limit = "512"]

mod bitcoin;
mod command;
mod config;
mod database;
mod ethereum;
mod float_maths;
mod fs;
mod history;
mod jsonrpc;
mod maker;
mod network;
mod order;
mod rate;
mod seed;
mod swap;
mod swap_id;
mod trace;

#[cfg(test)]
mod test_harness;

use crate::{
    command::{
        balance, create_transaction, deposit, dump_config, migrate_db, resume_only, trade,
        wallet_info, withdraw, Command, Options,
    },
    config::{read_config, Settings},
    fs::default_config_path,
};
use anyhow::Context;
use conquer_once::Lazy;

pub use anyhow::Result;
pub use maker::Maker;
pub use rate::{Rate, Spread};
pub use seed::Seed;
pub use swap_id::SwapId;

use crate::database::Database;
#[cfg(test)]
pub use test_harness::StaticStub;

pub static SECP: Lazy<::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All>> =
    Lazy::new(::bitcoin::secp256k1::Secp256k1::new);

#[tokio::main]
async fn main() -> Result<()> {
    let options = Options::from_args();

    let file = read_config(&options.config_file, default_config_path)?;
    let settings = Settings::from_config_file_and_defaults(file, options.network)
        .context("could not initialize configuration")?;

    if let Command::DumpConfig = options.cmd {
        dump_config(settings).expect("dump config");
        std::process::exit(0);
    }

    trace::init_tracing(settings.logging.level).expect("initialize tracing");

    let _guard = settings.sentry.as_ref().map(|sentry| {
        tracing::info!("Initializing sentry with URL {}", sentry.url.as_str());
        sentry::init(sentry.url.as_str())
    });

    let seed = config::Seed::from_file_or_generate(&settings.data.dir)
        .expect("Could not retrieve/initialize seed")
        .into();

    let bitcoin_wallet = bitcoin::Wallet::new(
        seed,
        settings.bitcoin.bitcoind.node_url.clone(),
        settings.bitcoin.network,
    )
    .await;

    let ethereum_wallet = ethereum::Wallet::new(
        seed,
        settings.ethereum.node_url.clone(),
        settings.ethereum.chain,
    )
    .await;

    match options.cmd {
        Command::Trade => trade(
            &seed,
            settings,
            bitcoin_wallet.expect("could not initialise bitcoin wallet"),
            ethereum_wallet.expect("could not initialise ethereum wallet"),
            options.network.unwrap_or_default(),
        )
        .await
        .expect("Start trading"),
        Command::WalletInfo => {
            let wallet_info = wallet_info(
                ethereum_wallet.ok(),
                bitcoin_wallet.ok(),
                &seed,
                settings.bitcoin.network,
            )
            .await
            .expect("get wallet info");
            println!("{}", wallet_info);
        }
        Command::Balance => {
            let balance = balance(
                ethereum_wallet.expect("could not initialise ethereum wallet"),
                bitcoin_wallet.expect("could not initialise bitcoin wallet"),
            )
            .await
            .expect("get wallet balances");
            println!("{}", balance);
        }
        Command::Deposit => {
            let deposit = deposit(
                ethereum_wallet.expect("could not initialise ethereum wallet"),
                bitcoin_wallet.expect("could not initialise bitcoin wallet"),
            )
            .await
            .expect("get wallet addresses");
            println!("{}", deposit);
        }
        Command::Withdraw(arguments) => {
            let ethereum_gas_price = ethereum::GasPrice::new(settings.ethereum.gas_price);
            let tx_id = withdraw(
                ethereum_wallet.expect("could not initialise ethereum wallet"),
                ethereum_gas_price,
                bitcoin_wallet.expect("could not initialise bitcoin wallet"),
                arguments,
            )
            .await
            .expect("Withdraw assets");
            println!("Withdraw successful. Transaction Id: {}", tx_id);
        }
        Command::DumpConfig => unreachable!(),
        Command::ResumeOnly => {
            let bitcoind_client = bitcoin::Client::new(settings.bitcoin.bitcoind.node_url.clone());
            let bitcoin_fee = bitcoin::Fee::new(settings.bitcoin.clone(), bitcoind_client);

            let ethereum_gas_price = ethereum::GasPrice::new(settings.ethereum.gas_price.clone());

            resume_only(
                settings,
                bitcoin_wallet.expect("could not initialise bitcoin wallet"),
                bitcoin_fee,
                ethereum_wallet.expect("could not initialise ethereum wallet"),
                ethereum_gas_price,
            )
            .await
            .expect("Wrapping up")
        }
        Command::CreateTransaction(input) => {
            let bitcoind_client = bitcoin::Client::new(settings.bitcoin.bitcoind.node_url.clone());
            let bitcoin_fee = bitcoin::Fee::new(settings.bitcoin.clone(), bitcoind_client);
            let ethereum_gas_price = ethereum::GasPrice::new(settings.ethereum.gas_price.clone());
            #[cfg(not(test))]
            let db = Database::new(&settings.data.dir.join("database"))?;
            #[cfg(test)]
            let db = Database::new_test()?;

            let hex = create_transaction(
                input,
                db,
                bitcoin_wallet.context("could not initialize bitcoin wallet")?,
                bitcoin_fee,
                ethereum_wallet.context("could not initialize ethereum wallet")?,
                ethereum_gas_price,
            )
            .await
            .context("failed to create transaction")?;

            println!("{}", hex);
        }
        Command::ArchiveSwap { id } => {
            #[cfg(not(test))]
            let db = Database::new(&settings.data.dir.join("database"))?;
            #[cfg(test)]
            let db = Database::new_test()?;

            db.archive_swap(&id)
                .await
                .context("failed to archive swap")?;
        }
        Command::MigrateDb(action) => {
            migrate_db(action, &settings.data.dir.join("database")).await?
        }
    };

    Ok(())
}
