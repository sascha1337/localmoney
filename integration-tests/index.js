import dotenv from 'dotenv';
dotenv.config()

import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
import * as fs from "fs";
import findFilesInDir from "./findFilesInDir.js";

const rpcEndpoint = "http://localhost:26657";
const maker_seed =
  "clip hire initial neck maid actor venue client foam budget lock catalog sweet steak waste crater broccoli pipe steak sister coyote moment obvious choose";
const taker_seed =
  "paddle prefer true embody scissors romance train replace flush rather until clap intact hello used cricket limb cake nut permit toss stove cute easily";

const min_amount = "120000000";
const max_amount = "360000000";
const offer_type = "buy";
const cw20_code_id = process.env.CW20;

const gasPrice = GasPrice.fromString("0.025ujunox");
const makerWallet = await DirectSecp256k1HdWallet.fromMnemonic(maker_seed, { prefix: "juno" });
const makerAccounts = await makerWallet.getAccounts();
const makerAddr = makerAccounts[0].address;
const makerClient = await SigningCosmWasmClient.connectWithSigner(rpcEndpoint, makerWallet, {
  broadcastTimeoutMs: 30 * 1000,
  gasPrice: gasPrice
});

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function instantiateFactory(codeIds) {
  //Instantiate Factory
  const factoryInstantiateMsg = {
    cw20_code_id: parseInt(cw20_code_id),
    trading_incentives_code_id: codeIds.trading_incentives,
    offer_code_id: codeIds.offer,
    trade_code_id: codeIds.trade,
    local_pool_addr: makerAddr, //TODO: use actual address
    local_token_addr: makerAddr,
    warchest_addr: makerAddr,
    // local_token_addr: process.env.LOCAL_TOKEN_ADDR,
  };
  const result = await makerClient.instantiate(makerAddr, codeIds.factory, factoryInstantiateMsg, "factory", "auto");
  console.log("instantiate result = ", result);
  console.log("\n");
  return result;
}

async function create_offers(offers_addr) {
  const offers = [
    {
      create: {
        offer: {
          offer_type: "buy",
          fiat_currency: "COP",
          min_amount,
          max_amount,
          rate: "1",
        },
      },
    },
    {
      create: {
        offer: {
          offer_type: "sell",
          fiat_currency: "BRL",
          min_amount,
          max_amount,
          rate: "50",
        },
      },
    },
    {
      create: {
        offer: {
          offer_type: "buy",
          fiat_currency: "USD",
          min_amount,
          max_amount,
          rate: "0",
        },
      },
    },
  ];

  let finalResult;

  for (let idx = 0; idx < offers.length; idx++) {
    const offer = offers[idx];
    console.log(`*Creating Offer ${idx}*`);
    const createOfferResult = await makerClient.execute(makerAddr, offers_addr, offer, "auto");
    console.log(`Created Offer ${idx}:`, createOfferResult);
    finalResult = createOfferResult;
  }
  return finalResult;
}

async function query_offers(offers_addr) {
  const queries = [
    {
      offers_query: {
        limit: 5,
        order: 'desc'
      },
    },
  ];

  const offers = [];
  for (let idx = 0; idx < queries.length; idx++) {
    const query = queries[idx];
    const queryResult = await makerClient.queryContractSmart(offers_addr, query);
    offers.push(queryResult)
  }

  return offers;
}

async function test(codeIds) {
  let factoryCfg;
  let factoryAddr = process.env.FACTORY;
  let tradeAddr;

  let setup = new Promise(async (resolve, reject) => {
    if (factoryAddr) {
      console.log("*Querying Factory Config*");
      const queryResult = await makerClient.queryContractSmart(factoryAddr, {config:{}});
      resolve(queryResult);
    } else {
      console.log("*Instantiating Factory*");
      instantiateFactory(codeIds).then((r) => {
        const factoryAddr = getAttribute(
          r,
          "instantiate",
          "_contract_address"
        );
        console.log("**Factory Addr:", factoryAddr);

        console.log("*Querying Factory Config*");
        const queryResult = makerClient.queryContractSmart(factoryAddr, {"config":{}});
        resolve(queryResult)
      });
    }
  });
  setup
    .then(async (r) => {
      factoryCfg = r;
      console.log("Factory Config result", r);
      if (process.env.CREATE_OFFERS) {
        await create_offers(factoryCfg.offers_addr);
      }
      return query_offers(factoryCfg.offers_addr);
    })
    .then((r) => {
      const newTradeMsg = {
        new_trade: {
          offer_id: r[0][0].id + "",
          ust_amount: min_amount + "",
          taker: makerAddr,
        },
      };
      console.log("*Creating Trade*");
      return makerClient.execute(makerAddr, factoryCfg.offers_addr, newTradeMsg, "auto");
    })
    .then((result) => {
      console.log("Trade Result:", result);
      tradeAddr = result.logs[0].events
        .find((e) => e.type === "instantiate")
        .attributes.find((a) => a.key === "_contract_address").value;
      console.log("**Trade created with address:", tradeAddr);
      //Send UST and fund trade
      const coin = Coin.fromData({
        denom: "uusd",
        amount: min_amount * 2 + "",
      });
      const coins = new Coins([coin]);
      let fundEscrowMsg = new MsgExecuteContract(
        taker,
        tradeAddr,
        { fund_escrow: {} },
        coins
      );
      console.log("*Funding Escrow*");
      return executeMsg(fundEscrowMsg, taker_wallet);
    })
    .then((r) => {
      if (r.txhash) {
        console.log("**Escrow Funded**");
      } else {
        console.log("%Error%");
      }
      const releaseMsg = new MsgExecuteContract(taker, tradeAddr, {
        release: {},
      });
      console.log("*Releasing Trade*");
      return executeMsg(releaseMsg, taker_wallet);
    })
    .then((r) => {
      console.log("**Trade Released**");
    });
}

function getContractNameFromPath(path) {
  let regex = RegExp(/artifacts\/(.*?)\.wasm/, "i");
  return path.match(regex)[1];
}

function getCodeIdFromResult(result) {
  return parseInt(getAttribute(result, "store_code", "code_id"));
}

function getAttribute(result, event, attribute) {
  return result.logs[0].events
    .find((e) => e.type === event)
    .attributes.find((e) => e.key === attribute).value;
}

function timeout(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function deploy(contract) {
  let codeIds = {};
  let contracts = findFilesInDir(process.env.CONTRACTS, ".wasm");

  if (contract.toLowerCase() === "all") {
    for (const i in contracts) {
      let c = contracts[i];
      codeIds[getContractNameFromPath(c)] = await uploadContract(c, makerAddr);
    }
    fs.writeFileSync("codeIds.json", JSON.stringify(codeIds), "utf8");
    console.log("Deploy Finished!", JSON.stringify(codeIds));
    await test(codeIds);
  } else {
    //Filter by name
    let codeIds = JSON.parse(fs.readFileSync("codeIds.json", "utf8"));
    console.log(codeIds);
    let names;
    if (contract.indexOf(",")) {
      names = contract.split(",");
    } else {
      names = [contract];
    }
    for (const i in names) {
      let name = names[i];
      for (const i in contracts) {
        let c = contracts[i];
        if (c.indexOf(name) >= 0) {
          codeIds[getContractNameFromPath(c)] = await uploadContract(c, makerAddr);
        }
      }
    }
    fs.writeFileSync("codeIds.json", JSON.stringify(codeIds), "utf8");
    console.log("Deploy Finished!", JSON.stringify(codeIds));
    await test(codeIds);
  }
}

async function uploadContract(filePath, addr) {
  const wasm = fs.readFileSync(filePath);
  const uploadResult = await makerClient.upload(addr, wasm, "auto");
  console.log('upload result:', uploadResult);
  await sleep(333);
  return uploadResult.codeId;
}

function fundEscrow(tradeAddr) {
  const coin = Coin.fromData({
    denom: "uusd",
    amount: min_amount + "",
  });
  const coins = new Coins([coin]);
  let fundEscrowMsg = new MsgExecuteContract(
    taker,
    tradeAddr,
    { fund_escrow: {} },
    coins
  );
  console.log("*Funding Escrow*");
  executeMsg(fundEscrowMsg, taker_wallet).then((r) => {
    console.log("Result", r);
    if (r.txhash) {
      release(tradeAddr, taker_wallet);
    }
  });
}

function release(tradeAddr, wallet) {
  console.log("Sending release msg");
  const releaseMsg = new MsgExecuteContract(taker, tradeAddr, {
    release: {},
  });
  console.log("Release Msg:", releaseMsg);
  executeMsg(releaseMsg, wallet).then((r) => {
    r.toJSON().then((r) => console.log(r));
  });
}

if (process.env.DEPLOY) {
  await deploy(process.env.DEPLOY);
} else if (process.env.FUND) {
  fundEscrow(process.env.FUND);
} else if (process.env.RELEASE) {
  release(process.env.RELEASE, taker_wallet);
} else {
  let codeIds = JSON.parse(fs.readFileSync("codeIds.json", "utf8"));
  await test(codeIds);
}
