#![no_std]


multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub static ERROR_PERMISSION_DENIED: &[u8] = b"Permission denied";

pub type SwapTokensFixedInputResultType<BigUint> = EsdtTokenPayment<BigUint>;

mod pair_proxy {
    multiversx_sc::imports!();

    pub type SwapTokensFixedInputResultType<BigUint> = EsdtTokenPayment<BigUint>;

    #[multiversx_sc::proxy]
    pub trait Pair {
        #[payable("*")]
        #[endpoint(swapTokensFixedInput)]
        fn swap_tokens_fixed_input(
            &self,
            token_out: TokenIdentifier,
            amount_out_min: BigUint,
        ) -> SwapTokensFixedInputResultType<Self::Api>;
    }
}

mod router_proxy {
    multiversx_sc::imports!();

    pub type SwapTokensFixedInputResultType<BigUint> = EsdtTokenPayment<BigUint>;

    #[multiversx_sc::proxy]
    pub trait FactoryModule {
        #[view(getPair)]
        fn get_pair(
            &self,
            first_token_id: TokenIdentifier,
            second_token_id: TokenIdentifier
        ) -> SwapTokensFixedInputResultType<Self::Api>;
    }
}

mod aggregator_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait AggregatorModule {
        #[view(getPair)]
        fn aggregate(
            &self,
            steps: ManagedVec<AggregatorStep<Self::Api>>, 
            limits: MultiValueEncoded<TokenAmount<Self::Api>>
        ) -> ManagedVec<EsdtTokenPayment>;
    }
}

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Copy, Clone, Debug)]
pub enum Status {
    Paused,
    Unpaused,
}

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Copy, Clone, Debug)]
pub enum Permissions {
    None,
    Operator,
}

/// One of the simplest smart contracts possible,
/// it holds a single variable in storage, which anyone can increment.
#[multiversx_sc::contract]
pub trait BridgeNew: token_send::TokenSendModule {

    #[view(getStatus)]
    #[storage_mapper("status")]
    fn status(&self) -> SingleValueMapper<Status>;

    #[view(getPermissions)]
    #[storage_mapper("permissions")]
    fn permissions(&self, address: ManagedAddress) -> SingleValueMapper<Permissions>;

    #[init]
    fn init(&self) {
        self.add_operator(self.blockchain().get_caller());
    }

    #[endpoint]
    #[only_owner]
    fn pause(&self) {
        self.status().set(Status::Paused);
    }

    #[endpoint]
    #[only_owner]
    fn unpause(&self) {
        self.status().set(Status::Unpaused);
    }

    #[only_owner]
    fn add_operator(&self, addr: ManagedAddress) {
        self.permissions(addr).set(Permissions::Operator);
    }

    #[only_owner]
    fn remove_operator(&self, addr: ManagedAddress) {
        self.permissions(addr).set(Permissions::None);
    }

    fn check_operator(&self, addr: ManagedAddress) {
        require!(self.permissions(addr).get() == Permissions::Operator, ERROR_PERMISSION_DENIED);
    }
    
    /// Add desired amount to the storage variable.
    #[endpoint(swapOnXExchange)]
    fn swap_xExchange_endpoint(
        &self,
        amount_in: BigUint,
        pairs: [ManagedAddress],
        token_route: [ManagedAddress],
        amount_out_min: BigUint,
        to: &ManagedAddress
    ) -> BigUint {
        self.check_operator(self.blockchain().get_caller());
        let i = 0;
        let return_value: SwapTokensFixedInputResultType<Self::Api>;
        while i < pairs.len() {
            return_value = self.pair_contract_proxy()
                .contract(pairs[i])
                .swap_tokens_fixed_input(&token_route[i + 1], amount_out_min)
                .with_esdt_transfer((token_route[i], 0, amount_in))
                .execute_on_dest_context();
            amount_in = return_value.amount;
        }
        self.send().direct_esdt(to, &token_route[token_route.len() - 1], return_value.token_nonce, &return_value.amount);
        return return_value.amount
    }

    #[endpoint(swapOnAshswapAggregator)]
    fn swap_ashswap_endpoint(
        &self,
        steps: ManagedVec<AggregatorStep<Self::Api>>, 
        limits: MultiValueEncoded<TokenAmount<Self::Api>>,
        to: &ManagedAddress
    ) -> BigUint {
        ManagedVec<EsdtTokenPayment> payment = self.aggregator_contract_proxy().contract(aggregator).aggregate(steps, limit)
        self.send().direct_esdt(to, payment[0].address, &payment.amount);
    }

    #[proxy]
    fn pair_contract_proxy(&self) -> pair_proxy::Proxy<Self::Api>;
    fn aggregator_contract_proxy(&self) -> aggregator_proxy::AggregatorModule
}
