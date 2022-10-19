// Malicious Trigger tests

use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, DepsMut, Env, IbcMsg, Response,
};

use std::convert::TryInto;

use crate::error::ContractError;
use crate::ibc_msg::{Msg, PacketMsg};
use crate::queue_handler::{send_all_party};
use crate::utils::{get_timeout, get_id_channel_pair_from_storage, convert_send_ibc_msg};
use crate::view_change::{convert_queue_to_ibc_msgs, testing_add2queue};
// use crate::ibc_msg::PacketMsg;
use crate::state::{
    CHANNELS, STATE, InputType, TBInput,
};



pub fn trigger_done(
    deps: DepsMut,
    env: Env
) -> Result<Response, ContractError> {
    let res = 
    Response::new()
        .add_attribute("action", "trigger")
        .add_attribute("trigger_behavior", "done");
    let state = STATE.load(deps.storage)?;

    let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
    // self-send msg
    // receive_queue(store, timeout, None, vec![packet.clone()], queue)?;
    let done_packet = Msg::Done {
        // val: "MALICIOUS_VAL".to_string()
        val: TBInput { binary: "TODO".to_string(), public_key: Vec::new(), signature: Vec::new()}
    };
    send_all_party(deps.storage, &mut queue, done_packet, get_timeout(&env), &env, deps.api)?;
    let msgs = convert_queue_to_ibc_msgs(deps.storage, &mut queue, get_timeout(&env))?;

    Ok(res
        .add_messages(msgs))
}

pub fn trigger_done_2(
    deps: DepsMut,
    env: Env
) -> Result<Response, ContractError> {
    let res = 
    Response::new()
        .add_attribute("action", "trigger")
        .add_attribute("trigger_behavior", "done");
    let state = STATE.load(deps.storage)?;

    let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
    // self-send msg
    // receive_queue(store, timeout, None, vec![packet.clone()], queue)?;
    let packet_1 = Msg::Done {
        // val: "PACKET_A".to_string()
        val: TBInput { binary: "TODO".to_string(), public_key: Vec::new(), signature: Vec::new()}
    };

    let packet_2 = Msg::Done {
        // val: "PACKET_B".to_string()
        val: TBInput { binary: "TODO".to_string(), public_key: Vec::new(), signature: Vec::new() }
    };

    let channel_id_1 = CHANNELS.load(deps.storage, 1)?;
    let channel_id_2 = CHANNELS.load(deps.storage, 2)?;

    let mut vec_1:Vec<Msg> = Vec::new();
    let mut vec_2:Vec<Msg> = Vec::new();
    vec_1.push(packet_1);
    vec_2.push(packet_2);

    let packet_queue_1 = PacketMsg::MsgQueue(vec_1);
    let packet_queue_2 = PacketMsg::MsgQueue(vec_2);

    let ibc_packet_1 = IbcMsg::SendPacket { channel_id: channel_id_1, data: to_binary(&packet_queue_1)?, timeout: get_timeout(&env) };
    let ibc_packet_2 = IbcMsg::SendPacket { channel_id: channel_id_2, data: to_binary(&packet_queue_2)?, timeout: get_timeout(&env) };

    Ok(res
        .add_message(ibc_packet_1)
        .add_message(ibc_packet_2))
}


pub fn trigger_abort(
    deps: DepsMut,
    env: &Env
) -> Result<Response, ContractError> {
    let res = 
    Response::new()
        .add_attribute("action", "trigger")
        .add_attribute("trigger_behavior", "abort");
    let state = STATE.load(deps.storage)?;

    if state.chain_id == state.primary {
        return Ok(res
        .add_attribute("error", "is primary"));
    }
    // let mut msgs = Vec::new();
    let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
    // self-send msg
    // receive_queue(store, timeout, None, vec![packet.clone()], queue)?;
    let abort_packet = Msg::Abort {
        view: state.view,
        chain_id: state.chain_id,
    };
    send_all_party(deps.storage, &mut queue, abort_packet, get_timeout(&env), env, deps.api)?;
    let msgs = convert_queue_to_ibc_msgs(deps.storage, &mut queue, get_timeout(&env))?;

    Ok(res
        .add_messages(msgs))
}

pub fn trigger_key1_diff_val(
    deps: DepsMut,
    env: Env
) -> Result<Response, ContractError> {
    let res = 
        Response::new()
            .add_attribute("action", "trigger");
    let state = STATE.load(deps.storage)?;
    
    if state.chain_id == state.primary {
        return Ok(res
        .add_attribute("trigger_behavior", "key1_diff_val")
        .add_attribute("error", "is primary"));
    }
    let mut msgs = Vec::new();
    // self-send msg
    // receive_queue(store, timeout, None, vec![packet.clone()], queue)?;

    let channel_ids = get_id_channel_pair_from_storage(deps.storage)?;

    for (chain_id, channel_id) in &channel_ids {
        let val = ["TRIGGER_", &chain_id.to_string()].join("");
        let val = TBInput { binary: "TODO".to_string(), public_key: Vec::new(), signature: Vec::new()};
        let msg_queue = vec![Msg::Key1 { val, view: state.view }];
        testing_add2queue(deps.storage, *chain_id, msg_queue.clone())?;
        let packet = PacketMsg::MsgQueue(msg_queue);
    
        let msg = convert_send_ibc_msg(channel_id.to_string(), packet, get_timeout(&env));
        msgs.push(msg);
    }

    return Ok(res
    .add_messages(msgs)
    .add_attribute("trigger_behavior", "key1_diff_val"));
}

pub fn trigger_multi_propose(
    deps: DepsMut,
    env: Env
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // check if this chain is the primary of current view
    if state.chain_id != state.primary {
        return Ok(Response::new()
        .add_attribute("action", "trigger")
        .add_attribute("trigger_behavior", "multi_propose")
        .add_attribute("error", "not primary"));
    }
    // let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
    let mut msgs = Vec::new();

    // Send different Propose to other parties
    let channel_ids = get_id_channel_pair_from_storage(deps.storage)?;

    for (chain_id, channel_id) in &channel_ids {
        let v = ["TRIGGER_", &chain_id.to_string()].join("");
        let v = TBInput { binary: "TODO".to_string(), public_key: Vec::new(), signature: Vec::new() };
        let msg_queue = vec![Msg::Propose {chain_id: state.chain_id, k: state.view, v, view: state.view}];
        testing_add2queue(deps.storage, *chain_id, msg_queue.clone())?;

        let packet = PacketMsg::MsgQueue(msg_queue);
        let msg = convert_send_ibc_msg(channel_id.to_string(), packet, get_timeout(&env));
        msgs.push(msg);
    }
    /* 
    for k in 0..3 {
        let v = ["TESTING", &k.to_string()].join("");
        let propose_packet = Msg::Propose {
            chain_id: state.chain_id,
            k,
            v,
            view: state.view,
        };
        
        send_all_party(deps.storage, &mut queue, propose_packet, get_timeout(&env))?;
    }
    let msgs = convert_queue_to_ibc_msgs(deps.storage, &mut queue, get_timeout(&env))?;
    */
    let mut state = STATE.load(deps.storage)?;
    state.current_tx_id += 1;
    STATE.save(deps.storage, &state)?;

    return Ok(Response::new()
    .add_messages(msgs)
    .add_attribute("action", "trigger")
    .add_attribute("trigger_behavior", "multi-propose"));
}
