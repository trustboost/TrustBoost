
use cosmwasm_std::{
    StdResult, IbcReceiveResponse, to_binary, IbcMsg, StdError, Storage, IbcTimeout, Env, wasm_execute, WasmMsg, Binary, SubMsg, Api
};
use serde_json::to_string;

use std::collections::HashSet;
use std::convert::TryInto;
use std::hash::Hash;

use crate::ContractError;
use crate::state::{RECEIVED_DONE, InputType, TBInput};
use crate::utils::{get_id_channel_pair_from_storage, get_chain_id, check_signature, append_binary_string, derive_addr_from_pubkey};
use crate::ibc_msg::{Msg,AcknowledgementMsg, MsgQueueResponse, PacketMsg};
use crate::{state::{
    HIGHEST_REQ, STATE, SEND_ALL_UPON, CHANNELS, TEST_QUEUE, TEST, RECEIVED, RECEIVED_ECHO, RECEIVED_KEY1, RECEIVED_KEY2, RECEIVED_KEY3,
    DEBUG, RECEIVED_LOCK, DEBUG_RECEIVE_MSG
}, abort::handle_abort};

// Handle Propose
fn handle_propose(
    store: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    timeout: IbcTimeout,
    _local_channel_id: Option<String>,
    chain_id: u32,
    k: u32, 
    v: InputType,
    view: u32 ,
    env: &Env,
    api: &dyn Api,
) -> StdResult<()> {
    let mut state = STATE.load(store)?;

    // ignore messages from other views, other than abort, done and request messages
    if view != state.view {
    } else {
        // upon receiving the first propose message from a chain
        if !state.received_propose && chain_id == state.primary {
            // RECEIVED_PROPOSE.save(store, chain_id, &true)?;
            let mut broadcast = false;
            state.received_propose = true;
            STATE.save(store, &state)?;
            
            // First case we should broadcast Echo message
            if state.lock == 0 || v == state.lock_val {
                broadcast = true;
            } else if view > k && k >= state.lock {
                // upon open_lock(proofs) == true
                // Second case we should broadcast Echo message
                if open_lock(store, state.proofs)? {
                    broadcast = true;
                }
            }
            // send_all_upon_join_queue(<echo, k, v, view>)
            if broadcast {
                let echo_packet = Msg::Echo { val: v, view };
                send_all_upon_join_queue(store, queue, echo_packet, timeout, env, api)?;
            }
            // send_all_upon_join_queue(<echo, k, v, view>)/

        }
    }
    Ok(())
}

// Handle Request
fn handle_request(
    store: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    view: u32,
    chain_id: u32,
    api: &dyn Api,
) -> StdResult<()> {
    let mut state = STATE.load(store)?;

    // state.key2_proofs.push((state.current_tx_id,"received_request".to_string(), chain_id as i32));
    // STATE.save(store, &state)?;
    // Update stored highest_request for that blockchain accordingly
    let highest_request = HIGHEST_REQ.load(store, chain_id)?;
    if highest_request < view {
        HIGHEST_REQ.save(store, chain_id, &view)?;
            
        if view == state.view {
            let packet = Msg::Suggest {
                chain_id: state.chain_id,
                view: state.view,
                key2: state.key2,
                key2_val: state.key2_val.clone(),
                prev_key2: state.prev_key2,
                key3: state.key3,
                key3_val: state.key3_val.clone(),
            };
            // Check if we are ready to send Suggest to Primary
            if chain_id == state.primary && !state.sent.contains(packet.name()) {
                
                state.sent.insert(packet.name().to_string());
                STATE.save(store, &state)?;
                queue[chain_id as usize].push(packet);
            }

            // Check if any pending send_all_upon_join
            let packets = SEND_ALL_UPON.may_load(store, chain_id)?;
            match packets {
                Some(p) => {
                    // Add to queue and remove from the buffer
                    queue[chain_id as usize].extend(p);
                    SEND_ALL_UPON.remove(store, chain_id);

                },
                None => (),
            };
        }
    }
    Ok(())
}

// Handle Suggest msg within MsgQueue
fn handle_suggest(
    store: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    timeout: IbcTimeout,
    chain_id: u32,
    view: u32,
    key2: u32,
    key2_val: InputType,
    prev_key2: i32,
    key3: u32,
    key3_val: InputType,
    env: &Env,
    api: &dyn Api,
) -> StdResult<()> {
    let mut state = STATE.load(store)?;

    // When I'm the primary
    if state.primary == state.chain_id {


        let mut receive_set= RECEIVED.load(store, "Suggest".to_string())?;
        // upon receiving the first suggest message from a chain
        if !receive_set.contains(&chain_id) {
            // Update the state
            receive_set.insert(chain_id);
            RECEIVED.save(store, "Suggest".to_string(), &receive_set)?;
            // Check if the following conditions hold
            if prev_key2 < key2 as i32 && key2 < view {
                state.key2_proofs.push((key2, key2_val, prev_key2));
                STATE.save(store, &state)?;
            }
            if key3 == 0 {
                state.suggestions.push((key3, key3_val));
                STATE.save(store, &state)?;
            } else if key3 < view {
                // Upon accept_key = true
                if accept_key(key3, key3_val.clone(), state.key2_proofs.clone()) {
                    state.suggestions.push((key3, key3_val.clone()));
                    STATE.save(store, &state)?;
                }
            }

            // Check if |suggestions| >= n - f
            if !state.sent.contains("Propose") && state.suggestions.len() >= (state.n - state.F) as usize {
                state.sent.insert("Propose".to_string());
                STATE.save(store, &state)?;
                // Retrive the entry with the largest k
                let (k, v) = state.suggestions.iter().max_by(|x, y| y.0.cmp(&x.0)).unwrap();
                let propose_packet = Msg::Propose {
                    chain_id: state.chain_id,
                    k: k.clone(),
                    v: v.clone(),
                    view: state.view,
                };
                
                send_all_upon_join_queue(store, queue, propose_packet, timeout, env, api)?;
                /*

                // send_all_upon_join_queue(<propose, k, v, view>)
                let channel_ids = get_id_channel_pair(store)?;
                // let state = STATE.load(store)?;
                for (chain_id, _channel_id) in &channel_ids {
                    let highest_request = HIGHEST_REQ.load(store, chain_id.clone())?;
                    if highest_request == state.view {
                        queue[*chain_id as usize].push(propose_packet.clone());

                    }
                    // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
                    else {
                        let action = |packets: Option<Vec<Msg>>| -> StdResult<Vec<Msg>> {
                            match packets {
                                Some(mut p) => {
                                    p.push(propose_packet.clone());
                                    Ok(p)
                                },
                                None => Ok(vec!(propose_packet.clone())),
                            }
                            
                        };
                        SEND_ALL_UPON.update(store, *chain_id, action)?;
                    }
                }
                */

            }
            
        }
    }

    Ok(())

}

// Handle Proof
fn handle_proof(
    store: &mut dyn Storage,
    local_channel_id: Option<String>,
    key1: u32,
    key1_val: InputType,
    prev_key1: i32,
    view: u32,
    _env: &Env,
    api: &dyn Api,
) -> StdResult<()> {
    let state = STATE.load(store)?;
    // detect if self-send
    let chain_id = match local_channel_id.clone() {
        Some(id) => {
            // Get the chain_id of the sender
            get_chain_id(store, id)
        },
        None => state.chain_id,
    };

    // let received_proof = RECEIVED_PROOF.load(store, chain_id)?;
    let mut receive_set= RECEIVED.load(store, "Proof".to_string())?;
    if !receive_set.contains(&chain_id) {
        // Update the state
        receive_set.insert(chain_id);
        RECEIVED.save(store, "Proof".to_string(), &receive_set)?;
        
        if view > key1 && key1 as i32 > prev_key1 {
            let mut state = STATE.load(store)?;
            state.proofs.push((key1, key1_val, prev_key1));
            STATE.save(store, &state)?;
        } 
        // if condition is met, update the proofs accordingly
        
    }

    Ok(())
}

// Handle Echo
fn handle_echo(
    store: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    timeout: IbcTimeout,
    local_channel_id: Option<String>,
    val: InputType,
    view: u32,
    env: &Env,
    api: &dyn Api,
) -> StdResult<()> {
    let key1_packet = Msg::Key1 { val: val.clone(), view };

    // ignore messages from other views, other than abort, done and request messages
    // if this condition holds, we have received Echo from n - f parties on same val
    if message_transfer_hop(store, val.clone(), view, queue, RECEIVED_ECHO, key1_packet.clone(), timeout.clone(), local_channel_id.clone(), env, api)? {
        let mut state = STATE.load(store)?;
        if state.key1_val != val {
            state.prev_key1 = state.key1 as i32;
            state.key1_val = val;                    
        }
        state.key1 = view;
        STATE.save(store, &state)?; 
    }
    
    Ok(())
}

// Handle Key1
fn handle_key1(
    store: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    timeout: IbcTimeout,
    local_channel_id: Option<String>,
    val: InputType,
    view: u32,
    env: &Env,
    api: &dyn Api,
) -> StdResult<()> {

 
    // ignore messages from other views, other than abort, done and request messages
    let key2_packet = Msg::Key2 { val: val.clone(), view };
    if message_transfer_hop(store, val.clone(), view, queue, RECEIVED_KEY1, key2_packet.clone(), timeout.clone(), local_channel_id.clone(), env, api)? {
        let mut state = STATE.load(store)?;
        if state.key2_val != val {
            state.prev_key2 = state.key2 as i32;
            state.key2_val = val;                    
        }
        state.key2 = view;
        STATE.save(store, &state)?; 
    }
    
    Ok(())
}

// Handle Key2
fn handle_key2(
    store: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    timeout: IbcTimeout,
    local_channel_id: Option<String>,
    val: InputType,
    view: u32,
    env: &Env,
    api: &dyn Api,
) -> StdResult<()> {
    let key3_packet = Msg::Key3 { val: val.clone(), view };
    if message_transfer_hop(store, val.clone(), view, queue, RECEIVED_KEY2, key3_packet.clone(),timeout.clone(), local_channel_id.clone(), env, api)? {
        let mut state = STATE.load(store)?;
        state.key3 = view;
        state.key3_val = val.clone();
        STATE.save(store, &state)?;    
    }
    
    Ok(())
}

// Handle Key3
fn handle_key3(
    store: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    timeout: IbcTimeout,
    local_channel_id: Option<String>,
    val: InputType,
    view: u32,
    env: &Env,
    api: &dyn Api,
) -> StdResult<()> {
    let lock_packet = Msg::Lock { val: val.clone(), view }; 

    DEBUG.save(store, 33330, &queue.len().to_string())?;
    if message_transfer_hop(store, val.clone(), view, queue, RECEIVED_KEY3, lock_packet.clone(), timeout.clone(), local_channel_id.clone(),env, api)? {
        let mut state = STATE.load(store)?;
        state.lock = view;
        state.lock_val = val;
        STATE.save(store, &state)?;    
        DEBUG.save(store, 33333, &"HANDLE_KEY_3_TRUE".to_string())?;
    } else {
        DEBUG.save(store, 3333, &"HANDLE_KEY_3_FALSE".to_string())?;
    }
    Ok(())
}

// Handle Lock
fn handle_lock(
    store: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    timeout: IbcTimeout,
    local_channel_id: Option<String>,
    val: InputType,
    view: u32,
    env: &Env,
    api: &dyn Api,
) -> Result<Vec<SubMsg>, ContractError> {        
    let done_packet = Msg::Done { val: val.clone() };
    // ignore messages from other views, other than abort, done and request messages
    // upon receiving from n - f parties with the same val
    let result = message_transfer_hop(store, val.clone(), view, queue, RECEIVED_LOCK, 
                                            done_packet.clone(), timeout.clone(), local_channel_id.clone(), env, api)?;

    // handle self-execute done
    // Since 
    if result {
        let mut vec_msgs:Vec<SubMsg> = Vec::new();
        let mut state = STATE.load(store).unwrap();
        if state.done.is_some() && !state.done_executed && check_signature(api, val.clone()){
            let address = derive_addr_from_pubkey(&val.public_key).unwrap();
            let appended_binary = append_binary_string(val.binary, &"tb_user".to_string(), &address.to_string());
            let stringified_binary = appended_binary.to_string();
            let wasm_msg = WasmMsg::Execute{
                contract_addr: state.contract_addr.to_string(),
                msg: appended_binary,
                funds: vec![]
            };
            let sub_msg: SubMsg = SubMsg::reply_always(wasm_msg, 1234);    
            state.done_executed = true;
            STATE.save(store, &state)?;
            vec_msgs.push(sub_msg);
            return Ok(vec_msgs);    
        }
    }

    // send_all_party(store, queue, done_packet, timeout.clone())?;
    return Ok(Vec::new());
}

// Handle Done
fn handle_done(
    store: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    timeout: IbcTimeout,
    local_channel_id: Option<String>,
    val: InputType,
    env: &Env,
    api: &dyn Api,
) -> Result<Vec<SubMsg>, ContractError> {   
    let mut state = STATE.load(store).unwrap();

    // upon receiving from n - f parties with the same val
    if message_transfer_hop(store, val.clone(), state.view, queue, RECEIVED_DONE, Msg::Done { val: val.clone() }, timeout.clone(), local_channel_id.clone(), env, api).unwrap() {
        // decide and terminate
        state.done = Some(val.clone());
        let mut vec_msgs:Vec<SubMsg> = Vec::new();

        // Only handle if it is not self send..., self send case is handled in handle_lock...
        if !local_channel_id.is_none() && !state.done_executed && check_signature(api, val.clone()){
            DEBUG.save(store, 7777777, &"EXECUTED ME HELLO!!!!!".to_string())?;
            state.done_executed = true;
            let address = derive_addr_from_pubkey(&val.public_key).unwrap();
            let appended_binary = append_binary_string(val.binary, &"tb_user".to_string(), &address.to_string());
            let stringified_binary = appended_binary.to_string();
            state.done_timestamp = Some(env.block.time);
            state.done_block_height = Some(env.block.height);

            let wasm_msg = WasmMsg::Execute{
                contract_addr: state.contract_addr.to_string(),
                msg: appended_binary,
                funds: vec![]
            };
            DEBUG.save(store, 777777777, &stringified_binary)?;
            let sub_msg = SubMsg::reply_always(wasm_msg, 1234);    
            vec_msgs.push(sub_msg)
        }
        STATE.save(store, &state)?;
        return Ok(vec_msgs);
    }
    return Ok(Vec::new());
}

pub fn receive_queue(
    store: &mut dyn Storage,
    timeout: IbcTimeout,
    local_channel_id: Option<String>,
    queue_to_process: Vec<Msg>,
    queue: &mut Vec<Vec<Msg>>,
    env: &Env,
    api: &dyn Api,
) -> StdResult<IbcReceiveResponse> {
    // let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
    if let Some(_) = STATE.load(store)?.done {
        return Ok(IbcReceiveResponse::new())
    }
    let mut wasm_exec_messages: Vec<SubMsg> = Vec::new();

    for msg in queue_to_process {
        let msg_string = msg.name();
        // TODO skip...
        // let key = msg.name().to_string();
        // if(RECEIVED.load(store,key)?.contains(local_channel_id.unwrap()?)) {
        //     continue;
        // }
        let result: StdResult<()> = match msg {
            Msg::Propose {
                chain_id,
                k,
                v,
                view,
            } => { 
                handle_propose(store, queue, timeout.clone(), local_channel_id.clone(), chain_id, k, v, view, env, api) 
            },
            Msg::Request { 
                view, 
                chain_id 
            } => {
                handle_request(store, queue, view, chain_id,api)
            },
            Msg::Suggest {
                chain_id,
                view,
                key2,
                key2_val,
                prev_key2,
                key3,
                key3_val,
            } => { 
                handle_suggest(store, queue, timeout.clone(), chain_id,view, key2, key2_val, prev_key2, key3, key3_val, env, api)
            },
            Msg::Proof {
                key1,
                key1_val,
                prev_key1,
                view,
            } => { 
                handle_proof(store, local_channel_id.clone(), key1, key1_val, prev_key1, view,env,api)
            },
            Msg::Echo { val, view } => { 
                handle_echo(store, queue, timeout.clone(), local_channel_id.clone(), val, view,env,api)
            },
            Msg::Key1 { val, view } => handle_key1(store, queue, timeout.clone(), local_channel_id.clone(), val, view,env,api),
            Msg::Key2 { val, view } => handle_key2(store, queue, timeout.clone(), local_channel_id.clone(), val, view,env,api),
            Msg::Key3 { val, view } => {
                handle_key3(
                    store, queue, timeout.clone(), local_channel_id.clone(), val, view,env, api
            )},
            Msg::Lock { val, view } => {
                // DEBUG_RECEIVE_MSG.update(store, "handle_lock".to_string(), | mut state| -> Result<_, ContractError> {
                //     match state {
                //         Some(mut vec) => {
                //             vec.push(val.clone().binary);
                //             Ok(vec)
                //         },
                //         None => {
                //             Ok(vec![val.clone().binary])
                //         }
                //     }
                // });                            
                wasm_exec_messages = handle_lock(store, queue, timeout.clone(), local_channel_id.clone(), val, view,env,api).unwrap();
                Ok(())
            },
            Msg::Done { val } => { 
                wasm_exec_messages = handle_done(store, queue, timeout.clone(), local_channel_id.clone(), val,env,api).unwrap();
                Ok(())
            }
            Msg::Abort { view, chain_id } => 
            {
                DEBUG.save(store, 200+chain_id, &"RECEIVED_ABORT".to_string())?;
                handle_abort(store, queue, view, chain_id, timeout.clone(), env, api)
            },
        };
        
        // unwrap the result to handle any errors
        result?
    }


    let mut res = IbcReceiveResponse::new();
    let mut state = STATE.load(store)?;
    if let Some(val) = state.done {
        if wasm_exec_messages.len() > 0 {
            DEBUG.save(store, 88888888, &"EXECUTED ME HELLO OUTSIDE!!!!!".to_string())?;
            res = res.add_submessages(wasm_exec_messages);
        }
    }   

    match local_channel_id {
        Some(_) => {
            // After handling all msgs in queue sucessfully
            // Generate msg queue to send
            let mut msgs = Vec::new();
            // let timeout = get_timeout(env);
            DEBUG.save(store, 300, &"LOCAL_CHANNEL_ID".to_string())?;

            //// TESTING /////
            let state = STATE.load(store)?;
            let mut i = 0;
            for (chain_id, msg_queue) in queue.iter().enumerate() {
                //// TESTING /////
                let chain_msg_pair = (chain_id as u32, msg_queue.to_vec());
                let action = |packets: Option<Vec<_>>| -> StdResult<Vec<_>> {
                    match packets {
                        Some(mut p) => {
                            p.push(chain_msg_pair.clone());
                            Ok(p)
                        },
                        None => Ok(vec!(chain_msg_pair.clone())),
                    }
                };
                TEST_QUEUE.update(store, state.current_tx_id, action)?;
                //// TESTING /////

                if chain_id != state.chain_id as usize {
                    // When chain wish to send some msgs to dest chain
                    if msg_queue.len() > 0 {
                        let channel_id = CHANNELS.load(store, chain_id.try_into().unwrap())?;
                        i = i+1;
                        let first_msg_name = msg_queue[0].name();
                        let debug_str = format!("{} {} FIRST MESSAGE LEN {} TO CHAIN_ID: {}" , 
                                                        "SEND_PACKET QUEUE SIZE", msg_queue.len(), first_msg_name, chain_id);   
                        DEBUG.save(store, 400+i, &debug_str)?;
                        let msg = IbcMsg::SendPacket {
                            channel_id,
                            data: to_binary(&PacketMsg::MsgQueue ( msg_queue.to_vec() ) )?,
                            timeout: timeout.clone(),
                        };
                        msgs.push(msg);
                    }
                }
            }
            //// TESTING ////
            let mut state = STATE.load(store)?;
            state.current_tx_id += 1;
            STATE.save(store, &state)?;
            //// TESTING ////

            let acknowledgement = to_binary(&AcknowledgementMsg::Ok(MsgQueueResponse { }))?;            
            // Add to Response if there are pending messages
            if msgs.len() > 0 {
                TEST.save(store, state.current_tx_id, &msgs)?;
                // state.current_tx_id += 1;
                STATE.save(store, &state)?;
                res = res.add_messages(msgs);
            }
                    
            Ok(res
                .set_ack(acknowledgement)
                .add_attribute("action", "receive_msg_queue"))
        },
        None => { 
            Ok(res.set_ack(b"{}")
                .add_attribute("action", "ibc_packet_ack"))
        }
    }
}


fn accept_key(key: u32, value: InputType, proofs: Vec<(u32, InputType, i32)>) -> bool {
    let mut supporting = 0;
    for (k, v, pk) in proofs {
        if (key as i32) < pk {
            supporting += 1;
        } else if key <= k && value == v {
            supporting += 1;
        }
    }
    if supporting >= 1 + 1 {
        return true;
    }
    false
}


fn open_lock(store: &mut dyn Storage, proofs: Vec<(u32, InputType, i32)>) -> StdResult<bool> {
    let mut supporting: u32 = 0;
    let state = STATE.load(store)?;
    for (k, v, pk) in proofs {
        if (state.lock as i32) <= pk {
            supporting += 1;
        } else if state.lock <= k && v != state.lock_val {
            supporting += 1;
        }
    }
    if supporting >= (state.F + 1) {
        Ok(true)
    } else {
        Ok(false)
    }
}

fn message_transfer_hop(
    storage: &mut dyn Storage, 
    val: InputType, 
    view: u32,
    queue: &mut Vec<Vec<Msg>>, 
    message_type: cw_storage_plus::Map<u64, HashSet<u32>>, 
    msg_to_send: Msg, 
    timeout: IbcTimeout, 
    channel_id: Option<String>, 
    env: &Env,
    api: &dyn Api
) -> Result<bool, StdError> {
        let state = STATE.load(storage)?;
        // ignore messages from other views, other than abort, done and request messages
        if view != state.view && message_type.namespace() != "received_done".as_bytes(){
            return Ok(false);
        }
        // detect if self-send
        let chain_id = match channel_id {
            Some(id) => {
                // Get the chain_id of the sender
                get_chain_id(storage, id)
            },
            None => state.chain_id,
        };
        // Initialize local record of messages of type key
        let action = |set: Option<HashSet<u32>>| -> StdResult<HashSet<u32>> {
            match set {
                Some(set) => Ok(set),
                None => Ok(HashSet::new()),
            }
        };
        let val_hash = val.calculate_hash();
        let mut set = message_type.update(storage, val_hash, action)?;
        if !set.contains(&chain_id) {
            set.insert(chain_id);
            message_type.save(storage, val_hash, &set)?;

            // If received Done, operate accordingly
            if message_type.namespace() == "received_done".as_bytes() {
                // check if have not sent Done && received from f + 1 parties 
                if !state.sent.contains(msg_to_send.name()) && set.len() >= (state.F + 1).try_into().unwrap() {
                    let mut state = STATE.load(storage)?;
                    state.sent.insert(msg_to_send.name().to_string());
                    STATE.save(storage, &state)?;
                    send_all_party(storage, queue, msg_to_send, timeout.clone(), env, api)?;
                }
                // upon receiving from n - f parties with the same val
                if set.len() >= (state.n - state.F).try_into().unwrap() {
                    return Ok(true);
                }
                return Ok(false);
            } else {
                // upon receiving from n - f parties with the same val
                if !state.sent.contains(msg_to_send.name()) && set.len() >= (state.n - state.F).try_into().unwrap() {
                    let mut state = STATE.load(storage)?;
                    state.sent.insert(msg_to_send.name().to_string());
                    STATE.save(storage, &state)?;
                    // if received Lock, ensure we send <done, val> to every party
                    if message_type.namespace() == "received_lock".as_bytes() {
                        send_all_party(storage, queue, msg_to_send, timeout, env, api)?;
                        return Ok(true);
                    } else {
                        send_all_upon_join_queue(storage, queue, msg_to_send, timeout, env, api)?;
                        return Ok(true);    
                    }
                } else {
                    return Ok(false);
                }
            }
        }
        Ok(false)
    }

// send_all_upon_join_queue Operation
pub fn send_all_upon_join_queue(storage: &mut dyn Storage, queue: &mut Vec<Vec<Msg>>, packet_msg: Msg, timeout: IbcTimeout, env: &Env, api: &dyn Api) -> Result<(), StdError> {
    let state = STATE.load(storage)?;
    let channel_ids = get_id_channel_pair_from_storage(storage)?;
    // self-send msg
    receive_queue(storage, timeout, None, vec![packet_msg.clone()], queue, env, api)?;

    for (chain_id, _channel_id) in &channel_ids {
        let highest_request = HIGHEST_REQ.load(storage, chain_id.clone())?;
        if highest_request == state.view {
            //DEBUG.save(storage, 10000000+chain_id, &chain_id.to_string())?;
            queue[*chain_id as usize].push(packet_msg.clone());
        } else {
            // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
            let action = |packets: Option<Vec<Msg>>| -> StdResult<Vec<Msg>> {
                match packets {
                    Some(mut p) => {
                        p.push(packet_msg.clone());
                        Ok(p)
                    },
                    None => Ok(vec!(packet_msg.clone())),
                }
                
            };
            SEND_ALL_UPON.update(storage, *chain_id, action)?;
        }
    }
    Ok(())
}

pub fn send_all_party(store: &mut dyn Storage, queue: &mut Vec<Vec<Msg>>, packet: Msg, timeout: IbcTimeout, env: &Env, api: &dyn Api) -> Result<(), StdError> {
    let channel_ids = get_id_channel_pair_from_storage(store)?;
    // self-send msg
    receive_queue(store, timeout, None, vec![packet.clone()], queue, env, api)?;

    for (chain_id, _channel_id) in &channel_ids {
        queue[*chain_id as usize].push(packet.clone());
    }
    
    Ok(())
}