use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about=None)]
pub struct RoxyArgs {
    #[arg(short, long)]
    pub port: Option<u16>,

    #[arg(short, long)]
    pub script: Option<String>,
}
