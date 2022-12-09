
use clap::Parser;

/// Decode transaction calldata without abi
#[derive(Parser, Debug)]
struct Args {
   /// Transaction whose calldata to decode
   tx: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let tx_hash = args.tx.trim_start_matches("0x");
    let tokens = abi_decoder::decode_transaction_calldata(tx_hash).await;

    println!("#### Decoded Tokens ####");
    for token in &tokens {
        abi_decoder::utils::print_parse_tree(token, 0);
    }
}
