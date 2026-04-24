use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PayResult {
    Completed {
        payment_statement_id: u64,
        product_id: String,
        transaction_id: String,
    },
    Rejected {
        message: String,
    },
}
