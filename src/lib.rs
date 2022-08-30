use concordium_std::*;

#[derive(Serialize, PartialEq, Eq, Debug, Clone, Copy)]
enum PiggyBankState {
    Intact,
    Smashed,
}

#[derive(Debug, PartialEq, Eq, Serial, Reject)]
enum SmashError {
    NotOwner,
    AlreadySmashed,
    TransferError,
}

#[init(contract = "PiggyBank")]
fn piggy_init<S: HasStateApi>(
    _ctx: &impl HasInitContext,
    _state_builder: &mut StateBuilder<S>,
) -> InitResult<PiggyBankState> {
    Ok(PiggyBankState::Intact)
}

#[receive(contract = "PiggyBank", name = "insert", payable)]
fn piggy_insert<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<PiggyBankState, StateApiType = S>,
    _amount: Amount,
) -> ReceiveResult<()> {
    ensure!(*host.state() == PiggyBankState::Intact);
    Ok(())
}

#[receive(contract = "PiggyBank", name = "smash", mutable)]
fn piggy_smash<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<PiggyBankState, StateApiType = S>,
) -> Result<(), SmashError> {
    let owner: AccountAddress = ctx.owner();
    let sender: Address = ctx.sender();
    ensure!(sender.matches_account(&owner), SmashError::NotOwner);
    ensure!(
        *host.state() == PiggyBankState::Intact,
        SmashError::AlreadySmashed
    );

    *host.state_mut() = PiggyBankState::Smashed;

    let balance: Amount = host.self_balance();
    let transfer_result: Result<(), TransferError> = host.invoke_transfer(&owner, balance);
    ensure!(transfer_result.is_ok(), SmashError::TransferError);

    Ok(())
}

#[receive(contract = "PiggyBank", name = "view")]
fn piggy_view<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<PiggyBankState, StateApiType = S>,
) -> ReceiveResult<(PiggyBankState, Amount)> {
    let current_state: PiggyBankState = *host.state();
    let current_balance: Amount = host.self_balance();
    Ok((current_state, current_balance))
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    #[concordium_test]
    fn test_init() {
        let ctx: TestContext<TestInitOnlyData> = TestInitContext::empty();
        let mut state_builder: StateBuilder<TestStateApi> = TestStateBuilder::new();

        let state_result: Result<PiggyBankState, Reject> = piggy_init(&ctx, &mut state_builder);

        let state: PiggyBankState =
            state_result.expect_report("Contract initialization results in error.");

        claim_eq!(
            state,
            PiggyBankState::Intact,
            "Piggy bank state should be intact after initialization."
        );
    }

    #[concordium_test]
    fn test_insert_intact() {
        let ctx: TestContext<TestReceiveOnlyData> = TestReceiveContext::empty();
        let host: TestHost<PiggyBankState> =
            TestHost::new(PiggyBankState::Intact, TestStateBuilder::new());
        let amount: Amount = Amount::from_micro_ccd(100);

        let result: Result<(), Reject> = piggy_insert(&ctx, &host, amount);

        claim!(result.is_ok(), "Inserting CCD results in error");
    }

    #[concordium_test]
    fn test_smash_intact() {
        let mut ctx: TestContext<TestReceiveOnlyData> = TestReceiveContext::empty();
        let owner: AccountAddress = AccountAddress([0u8; 32]);
        ctx.set_owner(owner);
        let sender: Address = Address::Account(owner);
        ctx.set_sender(sender);
        let mut host: TestHost<PiggyBankState> =
            TestHost::new(PiggyBankState::Intact, TestStateBuilder::new());
        let balance: Amount = Amount::from_micro_ccd(100);
        host.set_self_balance(balance);

        let result: Result<(), SmashError> = piggy_smash(&ctx, &mut host);

        claim!(
            result.is_ok(),
            "Smashing intact piggy bank results in error."
        );
        claim_eq!(
            *host.state(),
            PiggyBankState::Smashed,
            "Piggy bank should be smashed."
        );
        claim_eq!(
            host.get_transfers(),
            [(owner, balance)],
            "Smashing did not produce the correct transfers."
        );
    }

    #[concordium_test]
    fn test_smash_intact_not_owner() {
        let mut ctx: TestContext<TestReceiveOnlyData> = TestReceiveContext::empty();
        let owner: AccountAddress = AccountAddress([0u8; 32]);
        ctx.set_owner(owner);
        let sender: Address = Address::Account(AccountAddress([1u8; 32]));
        ctx.set_sender(sender);
        let mut host: TestHost<PiggyBankState> =
            TestHost::new(PiggyBankState::Intact, TestStateBuilder::new());
        let balance: Amount = Amount::from_micro_ccd(100);
        host.set_self_balance(balance);

        let result: Result<(), SmashError> = piggy_smash(&ctx, &mut host);

        claim_eq!(
            result,
            Err(SmashError::NotOwner),
            "Expected to fail with error NotOwner."
        );
    }
}
