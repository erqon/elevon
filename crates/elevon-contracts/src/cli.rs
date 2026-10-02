pub mod app;

use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Args, Serialize, Deserialize)]
pub struct ListQueryParams {
    #[arg(
        short,
        long,
        default_value_t = 25,
        help = "Limit the results. Set 0 to make it unlimited"
    )]
    pub limit: usize,

    #[arg(short, long, default_value_t = 0, help = "Starting offset")]
    pub offset: usize,
}

impl ListQueryParams {
    pub fn get_params(&self) -> Self {
        Self {
            limit: if self.limit == 0 {
                i64::MAX as usize
            } else {
                self.limit
            },
            offset: self.offset,
        }
    }
}
