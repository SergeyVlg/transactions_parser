use std::fmt::{Display, Formatter};
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
pub enum TransactionType {
    #[serde(rename = "DEPOSIT")] Deposit,
    #[serde(rename = "TRANSFER")] Transfer,
    #[serde(rename = "WITHDRAWAL")] Withdrawal
}

impl FromStr for TransactionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DEPOSIT" => Ok(TransactionType::Deposit),
            "TRANSFER" => Ok(TransactionType::Transfer),
            "WITHDRAWAL" => Ok(TransactionType::Withdrawal),

            _ => Err(()),
        }
    }
}

impl Display for TransactionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionType::Deposit => write!(f, "DEPOSIT"),
            TransactionType::Transfer => write!(f, "TRANSFER"),
            TransactionType::Withdrawal => write!(f, "WITHDRAWAL"),
        }
    }
}

impl From<u8> for TransactionType {
    fn from(value: u8) -> Self {
        match value {
            0 => TransactionType::Deposit,
            1 => TransactionType::Transfer,
            2 => TransactionType::Withdrawal,
            _ => panic!("Invalid transaction type value: {}", value),
        }
    }
}

impl Into<u8> for TransactionType {
    fn into(self) -> u8 {
        match self {
            TransactionType::Deposit => 0,
            TransactionType::Transfer => 1,
            TransactionType::Withdrawal => 2,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
pub enum TransactionStatus {
    #[serde(rename = "PENDING")] Pending,
    #[serde(rename = "SUCCESS")] Success,
    #[serde(rename = "FAILURE")] Failure
}

impl FromStr for TransactionStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(TransactionStatus::Pending),
            "FAILURE" => Ok(TransactionStatus::Failure),
            "SUCCESS" => Ok(TransactionStatus::Success),

            _ => Err(()),
        }
    }
}

impl From<u8> for TransactionStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => TransactionStatus::Success,
            1 => TransactionStatus::Failure,
            2 => TransactionStatus::Pending,
            _ => panic!("Invalid transaction status value: {}", value),
        }
    }
}

impl Into<u8> for TransactionStatus {
    fn into(self) -> u8 {
        match self {
            TransactionStatus::Success => 0,
            TransactionStatus::Failure => 1,
            TransactionStatus::Pending => 2,
        }
    }
}

impl Display for TransactionStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "PENDING"),
            TransactionStatus::Success => write!(f, "SUCCESS"),
            TransactionStatus::Failure => write!(f, "FAILURE")
        }
    }
}