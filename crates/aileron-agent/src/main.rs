use aileron_agent::cli::Cli;
use clap::Parser;

fn main() {
    let (private, public) = aileron_agent::secret::auth::generate_auth_token();

    println!("Public:  {:?}", public.to_bytes());
    // or hex if you add a hex crate / format manually
    println!("Public:  {:02x?}", public.as_bytes());
}
