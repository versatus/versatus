use std::{collections::HashSet, str::FromStr};

use block::{ConvergenceBlock, ProposalBlock};
use primitives::Address;
use vrrb_core::account::{AccountDigests, UpdateArgs};
use vrrb_core::transactions::{Token, Transaction, TransactionDigest, TransactionKind};

/// Provides a wrapper around the current rounds `ConvergenceBlock` and
/// the `ProposalBlock`s that it is made up of. Provides a convenient
/// data structure to be able to access each.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct RoundBlocks {
    pub convergence: ConvergenceBlock,
    pub proposals: Vec<ProposalBlock>,
}

/// Provides variants to parse to ensure state module handles updates
/// properly, whether it be an Account receiving tokens, and
/// account sending tokens, a new claim, claim staking (TODO),
/// fees or rewards (TODO).
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum UpdateAccount {
    Sender,
    Receiver,
    Claim,
    Fee,
    Reward,
}

/// Provides a wrapper around a given account update to
/// conveniently access the data needed to produce UpdateArgs
/// which can then be consolidated into a single UpdateArgs struct
/// for each account.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct StateUpdate {
    pub address: Address,
    pub token: Option<Token>,
    pub amount: u128,
    pub nonce: Option<u128>,
    pub storage: Option<String>,
    pub package_address: Option<String>,
    pub digest: TransactionDigest,
    pub update_account: UpdateAccount,
}

/// A wrapper to provide convenient conversion from
/// a Transaction to two StateUpdates, one for the
/// sender, one for the receiver. Can also provide some
/// verification around this struct.
// TODO: receiver update here can be used to provide
// ClaimStaking functionality, in which the `update_account`
// field for the `receiver_update` herein, can be used to
// produce a `Claim` update instead of only `Account` updates
#[derive(Debug)]
pub struct IntoUpdates {
    pub sender_update: StateUpdate,
    pub receiver_update: StateUpdate,
}

/// Provides an interface to convert a `ProposalBlock`
/// into the type that implements it
pub trait FromBlock {
    fn from_block(block: ProposalBlock) -> Self;
}

/// Provides an interface to convert a `Txn`
/// into the type that implements it
pub trait FromTxn {
    fn from_txn(txn: TransactionKind) -> Self;
}

/// Converts a `StateUpdate` into `UpdateArgs`
impl From<StateUpdate> for UpdateArgs {
    fn from(item: StateUpdate) -> UpdateArgs {
        let mut digest = AccountDigests::default();
        match &item.update_account {
            UpdateAccount::Sender => {
                digest.insert_sent(item.digest);
                UpdateArgs {
                    address: item.address,
                    nonce: item.nonce,
                    credits: None,
                    debits: Some(item.amount),
                    storage: Some(item.storage.clone()),
                    package_address: Some(item.package_address.clone()),
                    digests: Some(digest.clone()),
                }
            },
            UpdateAccount::Receiver => {
                digest.insert_recv(item.digest);
                UpdateArgs {
                    address: item.address,
                    nonce: item.nonce,
                    credits: Some(item.amount),
                    debits: None,
                    storage: Some(item.storage.clone()),
                    package_address: Some(item.package_address.clone()),
                    digests: Some(digest.clone()),
                }
            },
            UpdateAccount::Claim => {
                // RFC: Should we separate "claim" txn from "stake" txn
                digest.insert_stake(item.digest);
                UpdateArgs {
                    address: item.address,
                    nonce: item.nonce,
                    credits: None,
                    debits: None,
                    storage: None,
                    package_address: None,
                    digests: Some(digest.clone()),
                }
            },
            UpdateAccount::Fee => UpdateArgs {
                address: item.address,
                nonce: item.nonce,
                credits: Some(item.amount),
                debits: None,
                storage: None,
                package_address: None,
                digests: None,
            },
            UpdateAccount::Reward => UpdateArgs {
                address: item.address,
                nonce: item.nonce,
                credits: Some(item.amount),
                debits: None,
                storage: None,
                package_address: None,
                digests: None,
            },
        }
    }
}

/// Converts a `ProposalBlock` into a `HashSet` of
/// `StateUpdate`s which can then be easily converted into
/// a `HashSet` of `UpdateArgs` to update Accounts, Claims, etc.
impl FromBlock for HashSet<StateUpdate> {
    fn from_block(block: ProposalBlock) -> Self {
        let mut set = HashSet::new();
        let mut proposer_fees = 0u128;

        block.txns.into_iter().for_each(|(_digest, txn)| {
            let fee = txn.proposer_fee_share();
            proposer_fees += fee;

            let updates = IntoUpdates::from_txn(txn.clone());
            set.insert(updates.sender_update);
            set.insert(updates.receiver_update);

            let validator_fees = HashSet::<StateUpdate>::from_txn(txn.clone());
            set.extend(validator_fees);
        });

        let fee_update = StateUpdate {
            address: block.from.address,
            token: Some(Token::default()),
            nonce: None,
            amount: proposer_fees,
            storage: None,
            package_address: None,
            digest: TransactionDigest::default(),
            update_account: UpdateAccount::Fee,
        };

        set.insert(fee_update);

        set
    }
}

/// Converts a Transaction into an `IntoUpdate`
/// which is a simple wrapper around 2 `StateUpdate`s
/// one for the sender and one for the receiver
impl FromTxn for IntoUpdates {
    fn from_txn(txn: TransactionKind) -> IntoUpdates {
        let sender_update = StateUpdate {
            address: txn.sender_address(),
            token: Some(txn.token()),
            amount: txn.amount(),
            nonce: Some(txn.nonce()),
            storage: None,
            package_address: None,
            digest: txn.id(),
            update_account: UpdateAccount::Sender,
        };

        let receiver_update = StateUpdate {
            address: txn.receiver_address(),
            token: Some(txn.token()),
            amount: txn.amount(),
            nonce: None,
            storage: None,
            package_address: None,
            digest: txn.id(),
            update_account: UpdateAccount::Receiver,
        };

        IntoUpdates {
            sender_update,
            receiver_update,
        }
    }
}

/// Converts a Transaction into a HashSet of `StateUpdate`s
/// for fee distribution among the validators of a given tx
impl FromTxn for HashSet<StateUpdate> {
    fn from_txn(txn: TransactionKind) -> HashSet<StateUpdate> {
        let mut set = HashSet::new();
        let fees = txn.validator_fee_share();
        if let Some(mut validator_set) = txn.validators() {
            validator_set.retain(|_, vote| *vote);
            let validator_share = fees / (validator_set.len() as u128);
            validator_set.iter().for_each(|(k, _v)| {
                let address = Address::from_str(k);
                if let Ok(addr) = address {
                    set.insert(StateUpdate {
                        address: addr,
                        token: None,
                        amount: validator_share,
                        nonce: None,
                        storage: None,
                        package_address: None,
                        digest: TransactionDigest::default(),
                        update_account: UpdateAccount::Fee,
                    });
                }
            });
        }

        set
    }
}
