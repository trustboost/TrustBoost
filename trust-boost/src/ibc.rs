use std::convert::TryInto;

use cosmwasm_std::{
    entry_point, from_slice, to_binary, Binary, DepsMut, Env, Event, StdResult,
};
use cosmwasm_std::{
    IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse,
};

use crate::ibc_msg::{
    AcknowledgementMsg, PacketMsg, WhoAmIResponse, ProofResponse, EchoResponse, Key1Response, Key2Response, Key3Response, LockResponse, DoneResponse, Msg,
};

use crate::state::{
    CHANNELS, STATE, HIGHEST_ABORT, IBC_MSG_SEND_DEBUG, InputType,
};
use crate::utils::{get_timeout};
use crate::queue_handler::{receive_queue};

#[entry_point]
/// enforces ordering and versioing constraints
pub fn ibc_channel_open(_deps: DepsMut, _env: Env, _msg: IbcChannelOpenMsg) -> StdResult<()> {
    // verify_channel(msg)?;
    Ok(())
}

#[entry_point]
/// once it's established, we send a WhoAmI message
pub fn ibc_channel_connect(
    deps: DepsMut,
    env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    // Retrieve the connecting channel_id
    let channel_id = &channel.endpoint.channel_id;

    // Keep a record of connected channels
    let mut state = STATE.load(deps.storage)?;
    state.channel_ids.push(channel_id.to_string());
    // increment the total no of chains
    state.n += 1;
    STATE.save(deps.storage, &state)?;
    // let dst_port =  &channel.counterparty_endpoint.port_id;


    // let action = | mut state: State | -> StdResult<_> {
    //     state.channel_ids.insert(dst_port.to_string(), channel_id.to_string());
    //     Ok(state)
    // };
    // Storing channel_id info to state
    // STATE.update(deps.storage, action)?;

    // let action = |_| -> StdResult<String> {
    //     Ok(channel_id.to_string())
    // };
    // CHANNELS.update(deps.storage, dst_port.to_string(), action)?;

    // construct a packet to send, using the WhoAmI specification
    let packet = PacketMsg::WhoAmI {
        chain_id: state.chain_id,
    };
    let msg = IbcMsg::SendPacket {
        channel_id: channel_id.clone(),
        data: to_binary(&packet)?,
        timeout: get_timeout(&env)
    };

    Ok(IbcBasicResponse::new()
        .add_message(msg)
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", channel_id))
}

#[entry_point]
/// On closed channel, simply delete the channel_id local state
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    // fetch the connected channel_id
    let channel = msg.channel();
    let channel_id = &channel.endpoint.channel_id;
    // Remove the channel_ids stored in CHANNELS
    // CHANNELS.remove(deps.storage, dst_port.to_string());

    // let action = | mut state: State | -> StdResult<_> {
    //     state.channel_ids.retain(|_, v| !(v==channel_id));
    //     Ok(state)
    // };
    // STATE.update(deps.storage, action)?;

    // let action = |_| -> StdResult<String> {
    //     Ok(channel_id.to_string())
    // };
    // CHANNELS.update(deps.storage, dst_port.to_string(), action)?;

    // remove the channel
    // let mut state = STATE.load(deps.storage)?;
    // state.channel_ids.retain(|e| !(e==channel_id));
    // STATE.save(deps.storage, &state)?;
    // accounts(deps.storage).remove(channel_id.as_bytes());

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))
}

// This encode an error or error message into a proper acknowledgement to the recevier
fn encode_ibc_error(msg: impl Into<String>) -> Binary {
    // this cannot error, unwrap to keep the interface simple
    to_binary(&AcknowledgementMsg::<()>::Err(msg.into())).unwrap()
}

#[entry_point]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    (|| {
        let packet = msg.packet;
        // which local channel did this packet come on
        let dest_channel_id = packet.dest.channel_id;
        let msg: PacketMsg = from_slice(&packet.data)?;
        match msg {
            PacketMsg::MsgQueue(q) => 
            {
                let state = STATE.load(deps.storage)?;
                let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
                let result = receive_queue(deps.storage, get_timeout(&env), Some(dest_channel_id), q, &mut queue, &env, deps.api);
                IBC_MSG_SEND_DEBUG.save(deps.storage, "ibc_packet_receive".to_string(), &result.as_ref().unwrap().messages)?;
                return result;
            },
            PacketMsg::WhoAmI { chain_id } => receive_who_am_i(deps, dest_channel_id, chain_id),
        }
    })()
    .or_else(|e| {
        // we try to capture all app-level errors and convert them into
        // acknowledgement packets that contain an error code.
        let acknowledgement = encode_ibc_error(format!("invalid packet: {}", e));
        Ok(IbcReceiveResponse::new()
            .set_ack(acknowledgement)
            .add_event(Event::new("ibc").add_attribute("packet", "receive")))
    })
}


// processes PacketMsg::WhoAmI
fn receive_who_am_i(
    deps: DepsMut,
    channel_id: String,
    chain_id: u32,
) -> StdResult<IbcReceiveResponse> {
    let action = |_| -> StdResult<String> { Ok(channel_id.to_string()) };
    CHANNELS.update(deps.storage, chain_id, action)?;

    // initialize the highest_request of that chain
    // let action = |_| -> StdResult<u32> { Ok(0) };
    // HIGHEST_REQ.update(deps.storage, chain_id, action)?;
    // initialize the highest_request of that chain
    HIGHEST_ABORT.save(deps.storage, chain_id, &-1)?;

    let response = WhoAmIResponse {};
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;
    // and we are golden
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_who_am_i")
        .add_attribute("chain_id", chain_id.to_string()))
}


#[entry_point]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {
    let packet: PacketMsg = from_slice(&msg.original_packet.data)?;
    match packet {
        PacketMsg::MsgQueue(_q) => Ok(IbcBasicResponse::new()),
        PacketMsg::WhoAmI { chain_id: _ } => Ok(IbcBasicResponse::new()),
    }
}

#[entry_point]
/// This will never be called
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_packet_timeout"))
}



// pub fn receive_done(
//     _deps: DepsMut,
//     _val: String,
// ) -> StdResult<IbcReceiveResponse> {
//     let acknowledgement = to_binary(&AcknowledgementMsg::Ok(DoneResponse { }))?;
//     Ok(IbcReceiveResponse::new()
//         .set_ack(acknowledgement)
//         .add_attribute("action", "receive_done"))  
// }

// pub fn receive_lock(
//     _deps: DepsMut,
//     _val: String,
//     _view: u32,
// ) -> StdResult<IbcReceiveResponse> {
//     let acknowledgement = to_binary(&AcknowledgementMsg::Ok(LockResponse { }))?;
//     Ok(IbcReceiveResponse::new()
//         .set_ack(acknowledgement)
//         .add_attribute("action", "receive_lock"))  
// }

// pub fn receive_key3(
//     _deps: DepsMut,
//     _val: String,
//     _view: u32,
// ) -> StdResult<IbcReceiveResponse> {
//     let acknowledgement = to_binary(&AcknowledgementMsg::Ok(Key3Response { }))?;
//     Ok(IbcReceiveResponse::new()
//         .set_ack(acknowledgement)
//         .add_attribute("action", "receive_key3"))  
// }

// pub fn receive_key2(
//     _deps: DepsMut,
//     _val: String,
//     _view: u32,
// ) -> StdResult<IbcReceiveResponse> {
//     let acknowledgement = to_binary(&AcknowledgementMsg::Ok(Key2Response { }))?;
//     Ok(IbcReceiveResponse::new()
//         .set_ack(acknowledgement)
//         .add_attribute("action", "receive_key2"))  
// }

// pub fn receive_key1(
//     _deps: DepsMut,
//     _val: String,
//     _view: u32,
// ) -> StdResult<IbcReceiveResponse> {

//     let acknowledgement = to_binary(&AcknowledgementMsg::Ok(Key1Response { }))?;
//     Ok(IbcReceiveResponse::new()
//         .set_ack(acknowledgement)
//         .add_attribute("action", "receive_key1"))   
// }

// pub fn receive_echo(
//     _deps: DepsMut,
//     _val: String,
//     _view: u32,
// ) -> StdResult<IbcReceiveResponse> {

//     let acknowledgement = to_binary(&AcknowledgementMsg::Ok(EchoResponse { }))?;
//     Ok(IbcReceiveResponse::new()
//         .set_ack(acknowledgement)
//         .add_attribute("action", "receive_echo"))
// }

// pub fn receive_proof(
//     deps: DepsMut,
//     _k1: u32,
//     _key1_val: String,
//     _pk1: i32,
//     _view: u32,
// ) -> StdResult<IbcReceiveResponse> {
//     let mut state = STATE.load(deps.storage)?;
//     state.current_tx_id += 10;
//     STATE.save(deps.storage, &state)?;
//     // if view > k1 && k1 as i32 > pk1 && RECEIVED_PROOF.load(deps.storage, k)? {
//         // Get the chain_id of the sender
//         // let chain_id = CHANNELS.range(&deps.storage, min, max, order)

//     // } 
//     let response = ProofResponse {};
//     let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;
//     Ok(IbcReceiveResponse::new()
//         .set_ack(acknowledgement)
//         .add_attribute("action", "receive_proof"))
// }


/*
pub fn queue_receive_suggest(
    _queue_to_process: Vec<Vec<PacketMsg>>,
    deps: DepsMut,
    env: Env,
    from_chain_id: u32,
    view: u32,
    key2: u32,
    key2_val: String,
    prev_key2: i32,
    key3: u32,
    key3_val: String,
) -> StdResult<Vec<Vec<PacketMsg>>> {
    let mut state = STATE.load(deps.storage)?;
    let _acknowledgement = to_binary(&AcknowledgementMsg::Ok(SuggestResponse {}))?;

    // When I'm the primary
    if state.primary == state.chain_id {

        // upon receiving the first suggest message from a chain
        if !RECEIVED_SUGGEST.load(deps.storage, from_chain_id)? {
            RECEIVED_SUGGEST.save(deps.storage, from_chain_id, &true)?;
            // Check if the following conditions hold
            if prev_key2 < key2 as i32 && key2 < view {
                state.key2_proofs.push((key2, key2_val, prev_key2));
                STATE.save(deps.storage, &state)?;
            }
            if key3 == 0 {
                state.suggestions.push((key3, key3_val));
                STATE.save(deps.storage, &state)?;
            } else if key3 < view {
                // Upon accept_key = true
                if accept_key(key3, key3_val.clone(), state.key2_proofs.clone()) {
                    state.suggestions.push((key3, key3_val.clone()));
                    STATE.save(deps.storage, &state)?;
                }
            }

            // Check if |suggestions| >= n - f
            if state.suggestions.len() >= (state.n - F) as usize {
                let _timeout: IbcTimeout = get_timeout(env);
                // Retrive the entry with the largest k
                let (k, v) = state.suggestions.iter().max().unwrap();
                let _propose_packet = PacketMsg::Propose {
                    chain_id: state.chain_id,
                    k: k.clone(),
                    v: v.clone(),
                    view: state.view,
                };
            }
        }
    }
    Ok(Vec::new())

}

pub fn receive_suggest(
    deps: DepsMut,
    env: Env,
    from_chain_id: u32,
    view: u32,
    key2: u32,
    key2_val: String,
    prev_key2: i32,
    key3: u32,
    key3_val: String,
) -> StdResult<IbcReceiveResponse> {
    let mut state = STATE.load(deps.storage)?;
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(SuggestResponse {}))?;

    // When I'm the primary
    if state.primary == state.chain_id {

        // upon receiving the first suggest message from a chain
        if !RECEIVED_SUGGEST.load(deps.storage, from_chain_id)? {
            RECEIVED_SUGGEST.save(deps.storage, from_chain_id, &true)?;
            // Check if the following conditions hold
            if prev_key2 < key2 as i32 && key2 < view {
                state.key2_proofs.push((key2, key2_val, prev_key2));
                STATE.save(deps.storage, &state)?;
            }
            if key3 == 0 {
                state.suggestions.push((key3, key3_val));
                STATE.save(deps.storage, &state)?;
            } else if key3 < view {
                // Upon accept_key = true
                if accept_key(key3, key3_val.clone(), state.key2_proofs.clone()) {
                    state.suggestions.push((key3, key3_val.clone()));
                    STATE.save(deps.storage, &state)?;
                }
            }

            // Check if |suggestions| >= n - f
            if state.suggestions.len() >= (state.n - F).try_into().unwrap() {
                let timeout = get_timeout(env);
                // Retrive the entry with the largest k
                let (k, v) = state.suggestions.iter().max().unwrap();
                let propose_packet = PacketMsg::Propose {
                    chain_id: state.chain_id,
                    k: k.clone(),
                    v: v.clone(),
                    view: state.view,
                };

                return Ok(IbcReceiveResponse::new()
                    .set_ack(acknowledgement)
                    .add_submessages(send_all_upon_join(&deps, timeout, propose_packet).unwrap())
                    .add_attribute("action", "receive_suggest")
                    .add_attribute("suggest_sender_chain_id", from_chain_id.to_string()));
            }
            
        }
        
    }
    // let acknowledgement = to_binary(&AcknowledgementMsg::Ok(SuggestResponse {}))?;
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_suggest")
        .add_attribute("suggest_sender_chain_id", from_chain_id.to_string()))
}
*/

// fn _accept_key(key: u32, value: String, proofs: Vec<(u32, String, i32)>) -> bool {
//     let mut supporting = 0;
//     for (k, v, pk) in proofs {
//         if (key as i32) < pk {
//             supporting += 1;
//         } else if key <= k && value == v {
//             supporting += 1;
//         }
//     }
//     if supporting >= 1 + 1 {
//         return true;
//     }
//     false
// }

// pub fn receive_request(
//     deps: DepsMut,
//     _caller: String,
//     view: u32,
//     chain_id: u32,
// ) -> StdResult<IbcReceiveResponse> {
//     let mut state = STATE.load(deps.storage)?;
//     state.key2_proofs.push((state.current_tx_id,"received_request".to_string(), chain_id as i32));
//     state.current_tx_id += 1;
//     STATE.save(deps.storage, &state)?;
//     // Update stored highest_request for that blockchain accordingly
//     let highest_request = HIGHEST_REQ.load(deps.storage, chain_id)?;
//     if highest_request < view {
//         HIGHEST_REQ.save(deps.storage, chain_id, &view)?;
//     }

//     let acknowledgement = to_binary(&AcknowledgementMsg::Ok(RequestResponse {}))?;

//     Ok(IbcReceiveResponse::new()
//         .set_ack(acknowledgement)
//         .add_attribute("action", "receive_request")
//         .add_attribute("chain_id", chain_id.to_string()))
// }

// fn _open_lock(deps: &DepsMut, proofs: Vec<(u32, InputType, i32)>) -> StdResult<bool> {
//     let mut supporting: u32 = 0;
//     let state = STATE.load(deps.storage)?;
//     for (k, v, pk) in proofs {
//         if (state.lock as i32) <= pk {
//             supporting += 1;
//         } else if state.lock <= k && v != state.lock_val {
//             supporting += 1;
//         }
//     }
//     if supporting >= (state.F + 1) {
//         Ok(true)
//     } else {
//         Ok(false)
//     }
// }


/* 
pub fn receive_wrapper(
    msgs: Vec<SubMsg>,
    receive_type: String
) -> StdResult<IbcReceiveResponse> {
    let res = IbcReceiveResponse::new();
    Ok(res)
}

pub fn queue_receive_propose(
    deps: DepsMut,
    _caller: String,
    timeout: IbcTimeout,
    chain_id: u32,
    k: u32,
    v: String,
    view: u32,
) -> StdResult<Vec<SubMsg>> {
    let mut state = STATE.load(deps.storage)?;
    // let mut send_msg = false;
    let mut msgs: Vec<SubMsg> = Vec::new();
    // ignore messages from other views, other than abort, done and request messages
    if view != state.view {
    } else {
        // upon receiving the first propose message from a chain
        if chain_id == state.primary && !state.received_propose {
            // RECEIVED_PROPOSE.save(deps.storage, chain_id, &true)?;
            state.received_propose = true;
            STATE.save(deps.storage, &state)?;
            
            // First case we should broadcast Echo message
            if state.lock == 0 || v == state.lock_val {
                let echo_packet = PacketMsg::Echo { val: v, view };
                msgs.extend(send_all_upon_join(&deps, timeout.clone(), echo_packet).unwrap());
            

            } else if view > k && k >= state.lock {
                // upon open_lock(proofs) == true
                // Second case we should broadcast Echo message
                if open_lock(&deps, state.proofs)? {
                    let echo_packet = PacketMsg::Echo { val: v, view };
                    msgs.extend(send_all_upon_join(&deps, timeout.clone(), echo_packet).unwrap());
                }
            }
        }
    }

    // specify the type of AcknowledgementMsg to be ProposeResponse
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(ProposeResponse {}))?;
    let _res: IbcReceiveResponse = IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_propose");
    // send back acknowledgement, containing the response info
    Ok(msgs)
}

pub fn receive_propose(
    deps: DepsMut,
    _caller: String,
    timeout: IbcTimeout,
    chain_id: u32,
    k: u32,
    v: String,
    view: u32,
) -> StdResult<IbcReceiveResponse> {
    let mut state = STATE.load(deps.storage)?;
    // let mut send_msg = false;
    let mut msgs: Vec<SubMsg> = Vec::new();
    // ignore messages from other views, other than abort, done and request messages
    if view != state.view {
    } else {
        // upon receiving the first propose message from a chain
        if chain_id == state.primary && !state.received_propose {
            // RECEIVED_PROPOSE.save(deps.storage, chain_id, &true)?;
            state.received_propose = true;
            STATE.save(deps.storage, &state)?;
            
            // First case we should broadcast Echo message
            if state.lock == 0 || v == state.lock_val {
                let echo_packet = PacketMsg::Echo { val: v, view };
                msgs.extend(send_all_upon_join(&deps, timeout.clone(), echo_packet).unwrap());
            

            } else if view > k && k >= state.lock {
                // upon open_lock(proofs) == true
                // Second case we should broadcast Echo message
                if open_lock(&deps, state.proofs)? {
                    let echo_packet = PacketMsg::Echo { val: v, view };
                    msgs.extend(send_all_upon_join(&deps, timeout.clone(), echo_packet).unwrap());
                }
            }
        }
    }

    // specify the type of AcknowledgementMsg to be ProposeResponse
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(ProposeResponse {}))?;
    let res = IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_propose");
    // send back acknowledgement, containing the response info
    if msgs.is_empty() {
        Ok(res)
    } else {
        Ok(res.add_submessages(msgs))
    }
}
*/

/* 

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
    use crate::utils::IBC_APP_VERSION;

    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init,
        mock_ibc_channel_open_try, mock_ibc_packet_ack, mock_info, MockApi, MockQuerier,
        MockStorage,
    };
    use cosmwasm_std::{coins, CosmosMsg, OwnedDeps, IbcOrder};

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            // role: "leader".to_string(),
            chain_id: 0,
            input: 0.to_string(),
            contract_addr: 0.to_string()
        };
        let info = mock_info("creator_V", &coins(100, "BTC"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    // connect will run through the entire handshake to set up a proper connect and
    // save the account (tested in detail in `proper_handshake_flow`)
    fn connect(mut deps: DepsMut, channel_id: &str) {
        let handshake_open =
            mock_ibc_channel_open_init(channel_id, IbcOrder::Ordered, IBC_APP_VERSION);
        // first we try to open with a valid handshake
        ibc_channel_open(deps.branch(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_ack(channel_id, IbcOrder::Ordered, IBC_APP_VERSION);
        let res = ibc_channel_connect(deps.branch(), mock_env(), handshake_connect).unwrap();

        // this should send a WhoAmI request, which is received some blocks later
        assert_eq!(1, res.messages.len());
    }
}

*/
