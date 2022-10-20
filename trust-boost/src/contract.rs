#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Order, Reply, Response,
    StdError, StdResult, SubMsg, wasm_execute, WasmMsg, Storage, Addr, Timestamp,
};

use std::convert::TryInto;

use cw2::set_contract_version;
use std::cmp::Ordering;
use std::collections::HashSet;
use sha2::{Digest, Sha256};

use crate::error::ContractError;
use crate::ibc_msg::{Msg, PacketMsg};
use crate::queue_handler::{receive_queue, send_all_party};
use crate::utils::{get_timeout, init_receive_map, get_id_channel_pair_from_storage, convert_send_ibc_msg, derive_addr_from_pubkey, get_seconds_diff};
use crate::view_change::{view_change, convert_queue_to_ibc_msgs, testing_add2queue};
// use crate::ibc_msg::PacketMsg;
use crate::msg::{
    AbortResponse, ChannelsResponse, DoneQueryResponse, EchoQueryResponse, ExecuteMsg,
    HighestAbortResponse, HighestReqResponse, InstantiateMsg, Key1QueryResponse, Key2QueryResponse,
    Key3QueryResponse, LockQueryResponse, QueryMsg, ReceivedSuggestResponse, SendAllUponResponse,
    StateResponse, TestQueueResponse,
};
use crate::state::{
    State, CHANNELS, DEBUG, HIGHEST_ABORT, HIGHEST_REQ, RECEIVED, RECEIVED_ECHO, DEBUG_CTR,
    RECEIVED_KEY1, RECEIVED_KEY2, RECEIVED_KEY3, RECEIVED_LOCK, STATE, TEST, RECEIVED_DONE, IBC_MSG_SEND_DEBUG, InputType,
    DEBUG_RECEIVE_MSG
};
use crate::state::{SEND_ALL_UPON, TEST_QUEUE};
use crate::malicious_trigger::{trigger_done, trigger_done_2, trigger_abort, trigger_key1_diff_val, trigger_multi_propose};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:simple-storage";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REQUEST_REPLY_ID: u64 = 100;
pub const SUGGEST_REPLY_ID: u64 = 101;
pub const PROOF_REPLY_ID: u64 = 102;
pub const PROPOSE_REPLY_ID: u64 = 103;
pub const VIEW_TIMEOUT_SECONDS: u64 = 10;
pub const ALLOW_DEBUG: bool = true;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State::new(msg.chain_id, msg.input, deps.api.addr_validate(&msg.contract_addr)?, env.block.time);
    // let exe_msg = WasmMsg::Execute { contract_addr: , msg: , funds: () };
    // let exe_msg: ContractExecuteMsg = serde_json::from_str(&msg.msg).unwrap();
    // let exe_msg = wasm_execute(state.contract_addr.to_string(), &msg.msg, vec![])?;
    STATE.save(deps.storage, &state)?;
    for msg_type in vec!["Suggest", "Proof"] {
        RECEIVED.save(deps.storage, msg_type.to_string(), &HashSet::new())?;
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    DEBUG_CTR.save(deps.storage, &0)?;

    // let action = |_| -> StdResult<u32> { Ok(u32::MAX) };
    Ok(Response::new()
        // .add_message(exe_msg)
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

// execute entry_point is used for beginning new instance of IT-HS consensus
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Input { value } => handle_execute_input(deps, env, info, value),
        ExecuteMsg::PreInput { value } => handle_execute_preinput(deps, env, info, value),
        ExecuteMsg::ForceAbort {} => {
            todo!()
        },
        ExecuteMsg::Abort {} => handle_execute_abort(deps, env),
        ExecuteMsg::Trigger { behavior } => handle_trigger(deps, env, behavior),
        ExecuteMsg::Key3 { val, view, local_channel_id } => {
            if !ALLOW_DEBUG {
                return Ok(Response::new())
            }
            HIGHEST_REQ.save(deps.storage, 0,&0)?;
            HIGHEST_REQ.save(deps.storage, 1, &0)?;
            HIGHEST_REQ.save(deps.storage, 2, &0)?;
            HIGHEST_REQ.save(deps.storage, 3, &0)?;

            let state = STATE.load(deps.storage)?;
            let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
            let mut result;
            if local_channel_id != "None" {
                result =receive_queue(
                    deps.storage,
                    get_timeout(&env),
                    Some(local_channel_id),
                    vec![Msg::Key3 { val: val, view: view }],
                    &mut queue,
                    &env, 
                    deps.api
                )?;
            } else {
                result = receive_queue(
                    deps.storage,
                    get_timeout(&env),
                    None,
                    vec![Msg::Key3 { val: val, view: view }],
                    &mut queue,
                    &env,
                    deps.api
                )?;
            }

            let messages = result.messages;
            Ok(Response::new().add_submessages(messages))
        },
        ExecuteMsg::Lock { val, view, local_channel_id } => {
            if !ALLOW_DEBUG {
                return Ok(Response::new())
            }
            let state = STATE.load(deps.storage)?;
            let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
            let mut result;
            if local_channel_id != "None" {
                result = receive_queue(
                    deps.storage,
                    get_timeout(&env),
                    Some(local_channel_id),
                    vec![Msg::Lock { val: val, view: view }],
                    &mut queue,
                    &env,
                    deps.api
                )?;
            } else {
                result = receive_queue(
                    deps.storage,
                    get_timeout(&env),
                    None,
                    vec![Msg::Lock { val: val, view: view }],
                    &mut queue,
                    &env,
                    deps.api
                )?;
            }
    
            let messages = result.messages;
            Ok(Response::new().add_submessages(messages))
        },
        ExecuteMsg::Done { val, view, local_channel_id } => {
            if !ALLOW_DEBUG {
                return Ok(Response::new())
            }
            let state = STATE.load(deps.storage)?;
            let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
            let mut result;
            if local_channel_id != "None" {
                result = receive_queue(
                    deps.storage,
                    get_timeout(&env),
                    Some(local_channel_id),
                    vec![Msg::Done { val: val }],
                    &mut queue,
                    &env,
                    deps.api
                )?;
            } else {
                result = receive_queue(
                    deps.storage,
                    get_timeout(&env),
                    None,
                    vec![Msg::Done { val: val }],
                    &mut queue,
                    &env,
                    deps.api
                )?;
            }
            
            let messages = result.messages;
            Ok(Response::new().add_submessages(messages))
        },         
        ExecuteMsg::SetContractAddr { addr } => {
            let mut state = STATE.load(deps.storage)?;
            state.contract_addr = cosmwasm_std::Addr::unchecked(addr);
            STATE.save(deps.storage, &state)?;
            Ok(Response::new())
        },
    }
}

pub fn handle_trigger(
    deps: DepsMut,
    env: Env,
    behavior: String,
) -> Result<Response, ContractError> {

    match behavior.as_str() {
        "multi_propose" => trigger_multi_propose(deps, env),
        "key1_diff_val" => trigger_key1_diff_val(deps, env),
        "abort" => trigger_abort(deps, &env),
        "done" => trigger_done(deps, env),
        "done_2" => trigger_done_2(deps, env),
        _ => Ok(Response::new()
                .add_attribute("action", "trigger")
                .add_attribute("trigger_behavior", "unknown"))
    }
}

pub fn handle_execute_input(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    input: InputType,
) -> Result<Response, ContractError> {
    // set timeout for broadcasting
    let timeout: IbcTimeout = get_timeout(&env);
    /* a better way?
    CHANNELS
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|id| HIGHEST_REQ.save(deps.storage, id?, &0)? );
    */

    // Initialization
    init_receive_map(deps.storage)?;
    // Re-init
    let mut state = STATE.load(deps.storage)?;
    state.re_init(input, env.block.time.clone());

    // Store values to state
    STATE.save(deps.storage, &state)?;

    // By calling view_change(), Request messages will be delivered to all chains that we established a channel with
    view_change(deps.storage, timeout.clone(), &env, deps.api)

}

pub fn handle_execute_preinput(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    input: InputType,
) -> Result<Response, ContractError> {
    // Initialization
    init_receive_map(deps.storage)?;

    // Re-init
    let mut state = STATE.load(deps.storage)?;
    state.re_init(input, env.block.time.clone());
    // Store values to state
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "execute")
        .add_attribute("msg_type", "pre_input"))
}

pub fn handle_execute_abort(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;


    match state.done {
        Some(val) => {
            return Err(ContractError::CustomError {val: "Process is Done Cannot abort".to_string()});
        },
        None => ()
    };

    let end_time = state.start_time.plus_seconds(VIEW_TIMEOUT_SECONDS);
    match env.block.time.cmp(&end_time) {
        Ordering::Greater => {
            let abort_packet = Msg::Abort {
                view: state.view,
                chain_id: state.chain_id,
            };
            let mut queue: Vec<Vec<Msg>> =
                vec![vec![abort_packet.clone()]; state.n.try_into().unwrap()];

            let response = receive_queue(
                deps.storage,
                get_timeout(&env),
                Some("ABORT_UNUSED_CHANNEL".to_string()),
                vec![abort_packet.clone()],
                &mut queue,
                &env,
                deps.api
            )?;

            let sub_msgs = response.messages;

            IBC_MSG_SEND_DEBUG.save(deps.storage, "ABORT".to_string(), &sub_msgs)?;
            Ok(Response::new()
                .add_attribute("action", "execute")
                .add_submessages(sub_msgs)
                .add_attribute("msg_type", "abort"))
        }
        _ => {
            // handle_abort(deps.storage, state.view, state.chain_id);
            // Ok(response)
            Err(ContractError::CustomError {
                val: "Invalid Abort timetsamp hasn't passed yet".to_string(),
            })
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetStateProgress {} => to_binary(&query_state_progress(deps)?),
        QueryMsg::GetChannels {} => to_binary(&query_channels(deps)?),
        QueryMsg::GetTest {} => to_binary(&query_test(deps)?),
        QueryMsg::GetHighestReq {} => to_binary(&query_highest_request(deps)?),
        QueryMsg::GetReceivedSuggest {} => to_binary(&query_received_suggest(deps)?),
        QueryMsg::GetSendAllUpon {} => to_binary(&query_send_all_upon(deps)?),
        QueryMsg::GetTestQueue {} => to_binary(&query_test_queue(deps)?),
        QueryMsg::GetEcho {} => to_binary(&query_echo(deps)?),
        QueryMsg::GetKey1 {} => to_binary(&query_key1(deps)?),
        QueryMsg::GetKey2 {} => to_binary(&query_key2(deps)?),
        QueryMsg::GetKey3 {} => to_binary(&query_key3(deps)?),
        QueryMsg::GetLock {} => to_binary(&query_lock(deps)?),
        QueryMsg::GetDone {} => to_binary(&query_done(deps)?),
        QueryMsg::GetAbortInfo {} => to_binary(&query_abort_info(deps, env)?),
        QueryMsg::GetDebug {} => to_binary(&query_debug(deps)?),
        QueryMsg::GetHighestAbort {} => to_binary(&query_highest_abort(deps)?),
        QueryMsg::GetIbcDebug {} => to_binary(&query_ibc_debug(deps)?),
        QueryMsg::GetDebugReceive{} => to_binary(&query_debug_receive(deps)?),
        QueryMsg::CheckSignature { val } => to_binary(&check_signature(deps, val)?),
        QueryMsg::GetAddress { val }  => to_binary(&get_address(deps, val)?),
     }
}

fn query_echo(deps: Deps) -> StdResult<EchoQueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_ECHO
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(EchoQueryResponse { echo: query? })
}
fn query_key1(deps: Deps) -> StdResult<Key1QueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_KEY1
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(Key1QueryResponse { key1: query? })
}
fn query_key2(deps: Deps) -> StdResult<Key2QueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_KEY2
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(Key2QueryResponse { key2: query? })
}
fn query_key3(deps: Deps) -> StdResult<Key3QueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_KEY3
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(Key3QueryResponse { key3: query? })
}
fn query_lock(deps: Deps) -> StdResult<LockQueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_LOCK
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(LockQueryResponse { lock: query? })
}
fn query_done(deps: Deps) -> StdResult<DoneQueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_DONE
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(DoneQueryResponse { done: query? })
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(
        match state.done {           
            Some(val) => {
                let duration = match state.done_timestamp {
                    Some(val) => { 
                        Some(get_seconds_diff(&state.start_time, &state.done_timestamp.unwrap()))
                    },
                    None => {
                        None
                    }
                };   
                
                let minutes_duration = match duration {
                    Some(val) => { 
                        Some(val/60)
                    },
                    None => {
                        None
                    }
                };

                StateResponse::Done { 
                decided_val: val.binary,
                decided_timestamp: state.done_timestamp,
                block_height: state.done_block_height,
                start_time: state.start_time,
                seconds_duration: duration,
                minutes_duration: minutes_duration,
            }
        },
        None => StateResponse::InProgress { state },
    })
}

fn query_state_progress(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    return Ok(StateResponse::InProgress { state });
}

fn query_test_queue(deps: Deps) -> StdResult<TestQueueResponse> {
    let req: StdResult<Vec<_>> = TEST_QUEUE
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(TestQueueResponse { test_queue: req? })
}

fn query_send_all_upon(deps: Deps) -> StdResult<SendAllUponResponse> {
    let req: StdResult<Vec<_>> = SEND_ALL_UPON
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(SendAllUponResponse {
        send_all_upon: req?,
    })
}

fn query_received_suggest(deps: Deps) -> StdResult<ReceivedSuggestResponse> {
    // let req: StdResult<Vec<_>> = RECEIVED_SUGGEST
    //     .range(deps.storage, None, None, Order::Ascending)
    //     .collect();
    let req: StdResult<HashSet<_>> = RECEIVED.load(deps.storage, "Suggest".to_string());
    Ok(ReceivedSuggestResponse {
        received_suggest: req?,
    })
}

fn query_highest_request(deps: Deps) -> StdResult<HighestReqResponse> {
    let req: StdResult<Vec<_>> = HIGHEST_REQ
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(HighestReqResponse {
        highest_request: req?,
    })
}

fn query_highest_abort(deps: Deps) -> StdResult<HighestAbortResponse> {
    let req: StdResult<Vec<_>> = HIGHEST_ABORT
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(HighestAbortResponse {
        highest_abort: req?,
    })
}

fn query_test(deps: Deps) -> StdResult<Vec<(u32, Vec<IbcMsg>)>> {
    let test: StdResult<Vec<_>> = TEST
        .range(deps.storage, None, None, Order::Ascending)
        .collect();

    Ok(test?)
}

fn query_channels(deps: Deps) -> StdResult<ChannelsResponse> {
    let channels: StdResult<Vec<_>> = CHANNELS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    // let channels = channels?;
    Ok(ChannelsResponse {
        port_chan_pair: channels?,
    })
}

fn query_abort_info(deps: Deps, env: Env) -> StdResult<AbortResponse> {
    let state = STATE.load(deps.storage)?;
    // let channels = channels?;

    let end_time = state.start_time.plus_seconds(VIEW_TIMEOUT_SECONDS);
    let timeout = match env.block.time.cmp(&end_time) {
        Ordering::Greater => true,
        _ => false,
    };

    let is_input_finished = match state.done {
        Some(_) => true,
        _ => false,
    };

    Ok(AbortResponse {
        start_time: state.start_time,
        end_time: state.start_time.plus_seconds(60),
        current_time: env.block.time,
        is_timeout: timeout,
        done: is_input_finished,
        should_abort: (timeout && is_input_finished),
    })
}

fn query_debug(deps: Deps) -> StdResult<Vec<(u32, String)>> {
    let test: StdResult<Vec<_>> = DEBUG
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(test?)
}

fn query_ibc_debug(deps: Deps) -> StdResult<Vec<(String, Vec<SubMsg>)>> {
    let test: StdResult<Vec<_>> = IBC_MSG_SEND_DEBUG
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(test?)
}

fn query_debug_receive(deps: Deps) -> StdResult<Vec<(String, Vec<String>)>> {
    let test: StdResult<Vec<_>> = DEBUG_RECEIVE_MSG
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(test?)
}


// https://github.com/CosmWasm/cosmwasm/blob/main/contracts/crypto-verify/src/contract.rs#L90-L107
fn check_signature(deps: Deps, val: InputType) -> StdResult<Vec<bool>> {
    let mut result: Vec<bool> = Vec::new();

    // Hashing
    let hash = Sha256::digest(val.binary);

    // Verification
    let verify_result = deps
        .api
        .secp256k1_verify(hash.as_ref(), &val.signature, &val.public_key)?;

    result.push(verify_result);
    Ok(result)
}

// https://github.com/CosmWasm/cosmwasm/blob/main/contracts/crypto-verify/src/contract.rs#L90-L107
fn get_address(deps: Deps, val: InputType) -> StdResult<Addr> {
   let result = derive_addr_from_pubkey(&val.public_key);
    Ok(result.unwrap())
}


// entry_point for sub-messages
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        // REQUEST_REPLY_ID => handle_request_reply(deps, get_timeout(env), msg),
        REQUEST_REPLY_ID => Ok(Response::new()),
        SUGGEST_REPLY_ID => Ok(Response::new()),
        1234 => handle_wasm_exec(deps, msg),
        id => Err(StdError::generic_err(format!("Unknown reply id: {}", id))),
    }
}

fn handle_wasm_exec(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    // Upon sucessfully delivered the Suggest Message
    // Load the state
    // let _state = STATE.load(deps.storage)?;
    
    
    let res: Response = Response::new();

    let subMsgResult = msg.result;
    if subMsgResult.is_err() {
        let str = subMsgResult.unwrap_err();
        DEBUG.save(deps.storage, 12341234, &str.clone())?;
        Err(StdError::generic_err(&str.clone()))
    } else {
        DEBUG.save(deps.storage, 12341111, &"OK".to_string())?;
        Ok(res)
    }
    // Add consecutive submessages
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coins, from_binary, OwnedDeps};
}
