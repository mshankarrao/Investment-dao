#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod dao {

    use crate::ensure;
    use ink::storage::Mapping;
    use openbrush::contracts::traits::psp22::*;
    use scale::{
        Decode,
        Encode,
    };

    type ProposalId = u32;

    #[derive(Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Debug, PartialEq, Eq, scale_info::TypeInfo))]
    pub enum VoteType {
        // to implement
        Against,
        For,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum GovernorError {
        AmountShouldNotBeZero,
        DurationError,
        ProposalNotFound,
        VotePeriodEnded,
        ProposalAlreadyExecuted,
        AlreadyVoted,
        QuorumNotReached,
        ProposalNotAccepted

    }

    #[derive(Encode, Decode)]
    #[cfg_attr(
        feature = "std",
        derive(
            Debug,
            PartialEq,
            Eq,
            scale_info::TypeInfo,
            ink::storage::traits::StorageLayout
        )
    )]

    pub struct Proposal {
        to: AccountId,
        vote_start: u64,
        vote_end: u64,
        executed: bool,
        amount: Balance,
    }

    #[derive(Encode, Decode, Default)]
    #[cfg_attr(
        feature = "std",
        derive(
            Debug,
            PartialEq,
            Eq,
            scale_info::TypeInfo,
            ink::storage::traits::StorageLayout
        )
    )]
    pub struct ProposalVote {
        // to implement
        for_votes: u64,
        against_vote: u64,
    }

    #[ink(storage)]
    pub struct Governor {
        governance_token: AccountId,
        quorum: u8,
        proposals: Mapping<ProposalId,Proposal>,
        proposal_votes: Mapping<ProposalId,ProposalVote>,
        votes: Mapping<(ProposalId,AccountId),()>,
        next_proposal_id: ProposalId,
    }

    impl Governor {
        #[ink(constructor, payable)]
        pub fn new(governance_token: AccountId, quorum: u8) -> Self {
            Self {
                governance_token,
                quorum,
                proposals: Mapping::new(),
                proposal_votes:  Mapping::new(),
                votes: Mapping::new(),
                next_proposal_id: 0,
            }
        }

        #[ink(message)]
        pub fn next_proposal_id(&mut self) -> ProposalId{
            self.next_proposal_id + 1
        }

        #[ink(message)]
        pub fn get_proposal(&mut self, proposal_id: ProposalId) -> Result<Proposal,GovernorError>{
            Ok(self.proposals.get(proposal_id).unwrap())
        }

        #[ink(message)]
        pub fn propose(
            &mut self,
            to: AccountId,
            amount: Balance,
            duration: u64,
        ) -> Result<(), GovernorError> {
            ensure!(amount == 0, GovernorError::AmountShouldNotBeZero);
            ensure!(duration == 0, GovernorError::DurationError);
            let proposal = Proposal{
                to,
                vote_start: self.env().block_timestamp(),
                vote_end: duration,
                executed: false,
                amount,
            };
            self.proposals.insert( self.next_proposal_id, &proposal);
            Ok(())
    
        }

        #[ink(message)]
        pub fn vote(
            &mut self,
            proposal_id: ProposalId,
            vote: VoteType,
        ) -> Result<(), GovernorError> {
            ensure!(self.proposals.get(proposal_id).is_none(),GovernorError::ProposalNotFound);
            ensure!(self.proposals.get(proposal_id).unwrap().executed == true,GovernorError::ProposalAlreadyExecuted);
            ensure!(self.proposals.get(proposal_id).unwrap().vote_end > self.env().block_timestamp(),GovernorError::VotePeriodEnded);
            ensure!(self.votes.get((proposal_id,self.env().caller())).is_none(),GovernorError::AlreadyVoted);
            self.votes.insert((proposal_id, self.env().caller()),&());
            let mut weight = self.env().balance();
            let total_supply = ink::env::call::build_call:: <ink::env::DefaultEnvironment>()
            .call(self.governance_token)
            .gas_limit(5_000_000_000)
            .exec_input(
                ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(ink::selector_bytes!("PSP22::total_supply")))
            )
            .returns::<Balance>()
            .try_invoke(); 
            weight = weight/total_supply.unwrap().unwrap();
        
         match vote{
            VoteType::For => self.proposal_votes.get(proposal_id).unwrap().for_votes as u128 + weight,
            VoteType::Against => self.proposal_votes.get(proposal_id).unwrap().against_vote as u128 + weight,
         };

            Ok(())
            
        }

        #[ink(message)]
        pub fn execute(&mut self, proposal_id: ProposalId) -> Result<(), GovernorError> {
            ensure!(self.proposals.get(proposal_id).is_none(),GovernorError::ProposalNotFound);
            ensure!(self.proposals.get(proposal_id).unwrap().executed == true,GovernorError::ProposalAlreadyExecuted);
            let total_votes = (self.proposal_votes.get(proposal_id).unwrap().for_votes + self.proposal_votes.get(proposal_id).unwrap().against_vote) as u8;
            if total_votes < self.quorum {
                return Err(GovernorError::QuorumNotReached)
            }
            ensure!(self.proposal_votes.get(proposal_id).unwrap().for_votes >= 50, GovernorError::ProposalNotAccepted);
            ensure!(self.votes.get((proposal_id,self.env().caller())).is_none(),GovernorError::AlreadyVoted);
            self.proposals.get(proposal_id).unwrap().executed = true;
           // self.proposals.get(proposal_id).unwrap().to.transfer(self.proposals.get(proposal_id).unwrap().amount);

            Ok(())
        }

        // used for test
        #[ink(message)]
        pub fn now(&self) -> u64 {
            self.env().block_timestamp()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const ONE_MINUTE:u64 = 60;

        fn create_contract(initial_balance: Balance) -> Governor {
            let accounts: ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> = default_accounts();
            set_sender(accounts.alice);
            set_balance(contract_id(), initial_balance);
            Governor::new(AccountId::from([0x01; 32]), 50)
        }

        fn contract_id() -> AccountId {
            ink::env::test::callee::<ink::env::DefaultEnvironment>()
        }

        fn default_accounts(
        ) -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_sender(sender: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(sender);
        }

        fn set_balance(account_id: AccountId, balance: Balance) {
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                account_id, balance,
            )
        }

        #[ink::test]
        fn propose_works() {
            let accounts = default_accounts();
            let mut governor = create_contract(1000);
            assert_eq!(
                governor.propose(accounts.django, 0, 1),
                Err(GovernorError::AmountShouldNotBeZero)
            );
            assert_eq!(
                governor.propose(accounts.django, 100, 0),
                Err(GovernorError::DurationError)
            );
            let result = governor.propose(accounts.django, 100, 1);
            assert_eq!(result, Ok(()));
            let proposal = governor.get_proposal(0).unwrap();
            let now = governor.now();
            assert_eq!(
                proposal,
                Proposal {
                    to: accounts.django,
                    amount: 100,
                    vote_start: 0,
                    vote_end: now + 1 * ONE_MINUTE,
                    executed: false,
                }
            );
            assert_eq!(governor.next_proposal_id(), 1);
        }

        #[ink::test]
        fn quorum_not_reached() {
            let mut governor = create_contract(1000);
            let result = governor.propose(AccountId::from([0x02; 32]), 100, 1);
            let voting = governor.vote(0, VoteType::Against);
            assert_eq!(result, Ok(()));
            let execute = governor.execute(0);
            assert_eq!(execute, Err(GovernorError::QuorumNotReached));
        }
    }
}


#[macro_export]
macro_rules! ensure {
    ( $x:expr, $y:expr $(,)? ) => {{
        if $x {
            return Err($y.into());
        }
    }};
}