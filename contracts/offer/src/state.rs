use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Api, Extern, HumanAddr, Order, Querier, StdResult, Storage, Uint128};
use cosmwasm_storage::{bucket_read, singleton, singleton_read, ReadonlySingleton, Singleton};

pub static CONFIG_KEY: &[u8] = b"config";
pub static OFFERS_KEY: &[u8] = b"offers";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub offers_count: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Offer {
    pub id: u64,
    pub owner: HumanAddr,
    pub offer_type: OfferType,
    pub fiat_currency: FiatCurrency,
    pub min_amount: Uint128,
    pub max_amount: Uint128,
    pub state: OfferState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OfferType {
    Buy,
    Sell,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FiatCurrency {
    Brl,
    Cop,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OfferState {
    Active,
    Paused,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn query_all_offers<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    fiat_currency: FiatCurrency,
) -> StdResult<Vec<Offer>> {
    let offers: Vec<Offer> = bucket_read(OFFERS_KEY, &deps.storage)
        .range(None, None, Order::Descending)
        .flat_map(|item| item.and_then(|(_, offer)| Ok(offer)))
        .collect();

    let result: Vec<Offer> = offers
        .iter()
        .filter(|offer| offer.fiat_currency == fiat_currency)
        .cloned()
        .collect();

    Ok(result)
}
