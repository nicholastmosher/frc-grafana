use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct FrcGrafanaCli {
    #[clap(long)]
    pub host: String,

    #[clap(long, short, default_value = "7777")]
    pub port: u16,
}

impl FrcGrafanaCli {
    pub fn run(self) -> Result<()> {
        Ok(())
    }
}
