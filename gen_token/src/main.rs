use anyhow::Result;
use bcrypt::{hash, DEFAULT_COST};
use clap::{Parser, Subcommand};

mod tokens;
use tokens::Keys;

#[derive(Parser)]
#[command(about = "Generate a public/private key pair or a jwt token (.env must be utf-8)")]
struct Args {
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Token {
        /// sub as uid
        #[arg(short, long)]
        sub: String,
        /// aud as appid
        #[arg(short, long)]
        aud: String,
        /// role
        #[arg(short, long)]
        role: String,
    },
    Keys {},
    Bcrypt {
        /// plain
        #[arg(short, long)]
        plain: String,
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    match &args.command {
        Commands::Token { sub, aud, role } => {
            let k = Keys::new()?;
            let token = k.gen_token(sub, aud, role)?;
            println!("{}", token);
        }
        Commands::Keys {} => {
            let k = Keys::new_keys();
            let pub_key = k.public_key_string();
            let prv_key = k.private_key_string();
            println!("PUB_KEY={}\nPRV_KEY={}", pub_key, prv_key);
        }
        Commands::Bcrypt { plain } => {
            let encrypted: String = hash(plain, DEFAULT_COST).unwrap();
            println!("Encrypted password={}", encrypted);
        }
    }

    Ok(())
}
