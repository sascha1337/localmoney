use crate::errors::TradeError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, OfferMsg, QueryMsg};
use crate::state::{config, config_read, State, TradeState};
use crate::taxation::deduct_tax;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Attribute, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Response, StdResult, SubMsg, Uint128, WasmQuery,
};
use offer::state::{Offer, OfferType};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, TradeError> {
    //Load Offer
    let offer_id = msg.offer_id;
    let load_offer_result: StdResult<Offer> =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: msg.offer_contract.to_string(),
            msg: to_binary(&OfferMsg::LoadOffer { id: offer_id }).unwrap(),
        }));
    if load_offer_result.is_err() {
        return Err(TradeError::OfferNotFound { offer_id });
    }
    let offer = load_offer_result.unwrap();

    //TODO: it's probably a good idea to store this kind of configuration in a Gov contract.
    let expire_height = env.block.height + 600; //Roughly 1h.

    //Check that ust_amount is inside Offer limits
    let amount = Uint128::from(msg.ust_amount);
    if amount > offer.max_amount || amount < offer.min_amount {
        return Err(TradeError::AmountError {
            amount,
            min_amount: offer.min_amount,
            max_amount: offer.max_amount,
        });
    }

    //Instantiate recipient and sender addresses according to Offer type (buy, sell)
    let recipient: Addr;
    let sender: Addr;
    let fee_collector: Addr = msg.fee_collector;
    let _taker_is_buying: bool;

    if offer.offer_type == OfferType::Buy {
        _taker_is_buying = false;
        recipient = offer.owner;
        sender = info.sender.clone();
    } else {
        _taker_is_buying = true;
        recipient = info.sender.clone();
        sender = offer.owner;
    }

    //Instantiate Trade state
    let mut state = State {
        recipient,
        sender,
        fee_collector,
        offer_id,
        state: TradeState::Created,
        expire_height,
        ust_amount: Uint128::from(msg.ust_amount),
        final_asset: msg.final_asset,
        terraswap_factory: msg.terraswap_factory,
    };

    //Set state to EscrowFunded if enough UST was sent in the message.
    if !info.funds.is_empty() {
        //TODO: Check for Luna or other Terra native tokens.
        let ust_amount = get_ust_amount(info.clone());
        if ust_amount >= Uint128::from(msg.ust_amount) {
            state.state = TradeState::EscrowFunded
        }
    }

    //Save state.
    let save_state_result = config(deps.storage).save(&state);
    if save_state_result.is_err() {
        return Err(TradeError::InstantiationError {
            message: "Couldn't save state.".to_string(),
        });
    }

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, TradeError> {
    let cfg = config(deps.storage);
    let state = cfg.load().unwrap();
    match msg {
        ExecuteMsg::FundEscrow {} => try_fund_escrow(deps, env, info, state),
        ExecuteMsg::Refund {} => try_refund(deps, env, state),
        ExecuteMsg::Release {} => try_release(deps, env, info, state),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = config_read(deps.storage).load()?;
    Ok(state)
}

fn try_fund_escrow(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut state: State,
) -> Result<Response, TradeError> {
    let ust_amount = if !info.funds.is_empty() {
        get_ust_amount(info.clone())
    } else {
        let ust_balance = deps
            .querier
            .query_balance(env.contract.address, "uusd".to_string());
        ust_balance
            .unwrap_or(Coin {
                denom: "uusd".to_string(),
                amount: Uint128::zero(),
            })
            .amount
    };
    if ust_amount >= state.ust_amount {
        state.state = TradeState::EscrowFunded;
    } else {
        return Err(TradeError::ExecutionError {
            message: "UST amount is less than required to fund the escrow.".to_string(),
        });
    }
    config(deps.storage).save(&state).unwrap();
    Ok(Response::default())
}

fn try_release(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    state: State,
) -> Result<Response, TradeError> {
    //Check if sender can release
    if info.sender != state.sender {
        return Err(TradeError::Unauthorized {
            owner: state.sender,
            caller: info.sender,
        });
    }

    // throws error if state is expired
    if env.block.height > state.expire_height {
        return Err(TradeError::Expired {
            expire_height: state.expire_height,
            current_height: env.block.height,
        });
    }

    //Load and check balance
    let balance_result = deps.querier.query_all_balances(&env.contract.address);
    if balance_result.is_err() {
        return Err(TradeError::ReleaseError {
            message: "Contract has no funds.".to_string(),
        });
    }

    //Update trade State to TradeState::Closed
    let balance = balance_result.unwrap();
    let mut cfg = config(deps.storage);
    let mut state = cfg.load().unwrap();
    state.state = TradeState::Closed;
    let save_result = cfg.save(&state);
    if save_result.is_err() {
        return Err(TradeError::ExecutionError {
            message: "Failed to save state.".to_string(),
        });
    }

    if None == state.final_asset || "uusd" == state.final_asset.clone().unwrap() {
        let local_terra_fee = calculate_local_terra_fee(&balance).unwrap();
        let fee_response = send_tokens(
            &deps,
            state.fee_collector.clone(),
            local_terra_fee.clone(),
            "approve"
        ).unwrap();

        let final_balance = deduct_local_terra_fee(balance, local_terra_fee).unwrap();
        let amount_response = send_tokens(
            &deps,
            state.recipient.clone(),
            final_balance.clone(),
            "approve"
        ).unwrap();

        let r = Response::new()
            .add_submessage(fee_response.messages[0].clone())
            .add_submessage(amount_response.messages[0].clone());
        Ok(r)
    } else {
        Err(TradeError::ExecutionError {
            message: "Unsupported asset.".to_string(),
        })
    }
}

fn try_refund(deps: DepsMut, env: Env, state: State) -> Result<Response, TradeError> {
    // anyone can try to refund, as long as the contract is expired
    if state.expire_height > env.block.height {
        return Err(TradeError::RefundError {
            message: "Only expired trades can be refunded.".to_string(),
        });
    }

    let balance_result = deps.querier.query_all_balances(&env.contract.address);
    return if balance_result.is_ok() {
        let balance = balance_result.unwrap();
        send_tokens(&deps, state.sender, balance, "refund")
    } else {
        Err(TradeError::RefundError {
            message: "Contract has no funds.".to_string(),
        })
    };
}

fn get_ust_amount(info: MessageInfo) -> Uint128 {
    let mut ust_amount = Uint128::zero();
    let ust_index: &Option<usize> = &info.funds.iter().position(|coin| coin.denom.eq("uusd"));
    if Into::<usize>::into(ust_index.unwrap()) >= usize::MIN {
        let ust_coin: &Coin = &info.funds[ust_index.unwrap()];
        ust_amount = ust_coin.amount;
    }
    return ust_amount;
}

fn calculate_local_terra_fee(balance: &Vec<Coin>) -> StdResult<Vec<Coin>> {
    let fee_amount = balance[0].clone().amount.checked_div(Uint128::new(1000)).unwrap();
    Ok([Coin {
        amount: fee_amount,
        denom: balance[0].clone().denom
    }].to_vec())
}

fn deduct_local_terra_fee(balance: Vec<Coin>, local_terra_fee: Vec<Coin>) -> StdResult<Vec<Coin>> {
    Ok([Coin {
        amount: (balance[0].amount.checked_sub(local_terra_fee[0].amount)).unwrap(),
        denom: balance[0].clone().denom
    }].to_vec())
}

//Help to send native coins.
// this is a helper to move the tokens, so the business logic is easy to read
fn send_tokens(
    deps: &DepsMut,
    to_address: Addr,
    amount: Vec<Coin>,
    action: &str,
) -> Result<Response, TradeError> {
    let attributes = vec![attr("action", action), attr("to", to_address.clone())];
    let amount = [deduct_tax(&deps.querier, amount[0].clone()).unwrap()].to_vec();

    let r = Response::new()
        .add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: to_address.to_string(),
            amount,
        })))
        .add_attributes(attributes);
    Ok(r)
}

pub fn attr<K: ToString, V: ToString>(key: K, value: V) -> Attribute {
    Attribute {
        key: key.to_string(),
        value: value.to_string(),
    }
}
