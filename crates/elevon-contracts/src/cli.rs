pub mod app;

use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Args, Serialize, Deserialize)]
pub struct ListQueryParams {
    #[arg(long, default_value_t = 1, help = "Page")]
    pub page: usize,
}

impl ListQueryParams {
    pub fn get_limits(&self, count: u64) -> (usize, u64, usize) {
        let limit = 8_usize;
        let total_pages = count.div_ceil(limit as u64);
        let offset = (self.page - 1) * limit;

        (limit, total_pages, offset)
    }
}
