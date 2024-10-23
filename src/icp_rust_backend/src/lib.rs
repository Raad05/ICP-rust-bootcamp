use candid::{CandidType, Decode, Deserialize, Encode};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;

const MAX_VALUE_SIZE: u32 = 100;

// enums
#[derive(CandidType, Deserialize)]
enum Choice {
    Approve,
    Reject,
    Pass,
}

#[derive(CandidType, Deserialize)]
enum VoteError {
    AlreadyVoted,
    ProposalIsNotActive,
    InvalidChoice,
    NoSuchProposal,
    AccessRejected,
    UpdateError,
}

// structs
#[derive(CandidType, Deserialize)]
struct Proposal {
    description: String,
    approve: u32,
    reject: u32,
    pass: u32,
    is_active: bool,
    voted: Vec<candid::Principal>,
    owner: candid::Principal,
}
#[derive(CandidType, Deserialize)]
struct CreateProposal {
    description: String,
    is_active: bool,
}

// impmlementations
impl Storable for Proposal {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Proposal {
    const MAX_SIZE: u32 = MAX_VALUE_SIZE;
    const IS_FIXED_SIZE: bool = false;
}

// thread_local
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static PROPOSAL_MAP: RefCell<StableBTreeMap<u64, Proposal, Memory>> = RefCell::new(StableBTreeMap::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0)))));
}

// getter functions
#[ic_cdk::query]
fn get_proposal(key: u64) -> Option<Proposal> {
    PROPOSAL_MAP.with(|p| p.borrow().get(&key))
}

#[ic_cdk::query]
fn get_proposal_count() -> u64 {
    PROPOSAL_MAP.with(|p| p.borrow().len())
}

// setter functions
#[ic_cdk::update]
fn create_proposal(key: u64, proposal: CreateProposal) -> Option<Proposal> {
    let value: Proposal = Proposal {
        description: proposal.description,
        approve: 0,
        reject: 0,
        pass: 0,
        is_active: proposal.is_active,
        voted: vec![],
        owner: ic_cdk::caller(),
    };
    PROPOSAL_MAP.with(|p| p.borrow_mut().insert(key, value))
}

#[ic_cdk::update]
fn edit_proposal(key: u64, proposal: CreateProposal) -> Result<(), VoteError> {
    PROPOSAL_MAP.with(|p| {
        let old_proposal_opt: Option<Proposal> = p.borrow().get(&key);
        let old_proposal: Proposal;

        match old_proposal_opt {
            Some(value) => old_proposal = value,
            None => return Err(VoteError::NoSuchProposal),
        }

        if ic_cdk::caller() != old_proposal.owner {
            return Err(VoteError::AccessRejected);
        }

        let value: Proposal = Proposal {
            description: proposal.description,
            approve: old_proposal.approve,
            reject: old_proposal.reject,
            pass: old_proposal.pass,
            is_active: proposal.is_active,
            voted: old_proposal.voted,
            owner: ic_cdk::caller(),
        };

        let res: Option<Proposal> = p.borrow_mut().insert(key, value);

        match res {
            Some(_) => Ok(()),
            None => Err(VoteError::UpdateError),
        }
    })
}

#[ic_cdk::update]
fn end_proposal(key: u64) -> Result<(), VoteError> {
    PROPOSAL_MAP.with(|p| {
        let proposal_opt: Option<Proposal> = p.borrow().get(&key);
        let mut proposal: Proposal;

        match proposal_opt {
            Some(value) => proposal = value,
            None => return Err(VoteError::NoSuchProposal),
        }

        if ic_cdk::caller() != proposal.owner {
            return Err(VoteError::AccessRejected);
        }

        proposal.is_active = false;

        let res: Option<Proposal> = PROPOSAL_MAP.with(|p| p.borrow_mut().insert(key, proposal));

        match res {
            Some(_) => Ok(()),
            None => Err(VoteError::UpdateError),
        }
    })
}
