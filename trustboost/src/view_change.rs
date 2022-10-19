use std::convert::TryInto;
use std::vec;

use cosmwasm_std::{
    IbcTimeout, Response, IbcMsg, Storage, StdResult, Env, Api
};

use crate::ibc_msg::{PacketMsg, Msg};
use crate::queue_handler::{receive_queue, send_all_party, send_all_upon_join_queue};
use crate::state::{
    HIGHEST_REQ, STATE, TEST_QUEUE, CHANNELS, IBC_MSG_SEND_DEBUG
};

use crate::ContractError;
use crate::utils::{convert_send_ibc_msg};

pub fn view_change(storage: &mut dyn Storage, timeout: IbcTimeout, env: &Env, api: &dyn Api) -> Result<Response, ContractError> {

    let state = STATE.load(storage)?;
    let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());

    append_queue_view_change(storage, & mut queue, timeout.clone(), env, api)?;
    let msgs = convert_queue_to_ibc_msgs(storage, &queue, timeout.clone())?;


    let response = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "execute")
        .add_attribute("msg_type", "input");

    IBC_MSG_SEND_DEBUG.save(storage, "view_change".to_string(),&response.messages)?;    

    Ok(response)
}

pub fn append_queue_view_change(
    storage: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    timeout: IbcTimeout,
    env: &Env,
    api: &dyn Api,
) -> Result<(), ContractError> {
    // load the state
    let state = STATE.load(storage)?;
    // Add Request message to packets_to_be_broadcasted
    let request_packet = Msg::Request {
        view: state.view,
        chain_id: state.chain_id,
    };

    // Send Request to all parties
    send_all_party(storage, queue, request_packet, timeout.clone(), env, api)?;

    
    let suggest_packet = Msg::Suggest {
        chain_id: state.chain_id,
        view: state.view,
        key2: state.key2,
        key2_val: state.key2_val.clone(),
        prev_key2: state.prev_key2,
        key3: state.key3,
        key3_val: state.key3_val.clone(),
    };
    // Upon highest_request[primary] == view
    if state.chain_id != state.primary {
        if state.view == HIGHEST_REQ.load(storage, state.primary)? {
            queue[state.primary as usize].push(suggest_packet);
        }
    } else {
        receive_queue(storage, timeout.clone(), None, vec![suggest_packet], queue, env, api)?;
    }


    // Contruct Request messages to be broadcasted
    let proof_packet = Msg::Proof {
        key1: state.key1,
        key1_val: state.key1_val.clone(),
        prev_key1: state.prev_key1,
        view: state.view,
    };
    // send_all_upon_join(Proof)
    send_all_upon_join_queue(storage, queue, proof_packet, timeout.clone(), env, api)?;
    Ok(())
}

pub fn testing_add2queue(
    store: &mut dyn Storage,
    chain_id: u32,
    msg_queue: Vec<Msg>
) -> Result<(), ContractError> {
    //// TESTING /////
    let chain_msg_pair = (chain_id, msg_queue);
    let action = |packets: Option<Vec<_>>| -> StdResult<Vec<_>> {
        match packets {
            Some(mut p) => {
                p.push(chain_msg_pair.clone());
                Ok(p)
            },
            None => Ok(vec!(chain_msg_pair.clone())),
        }
    };
    let state = STATE.load(store)?;
    TEST_QUEUE.update(store, state.current_tx_id, action)?;
    // TEST_QUEUE.save(storage, state.current_tx_id, &(chain_id as u32, msg_queue.to_vec()))?;
    Ok(())
}
//// TESTING /////
        

pub fn convert_queue_to_ibc_msgs(
    storage: &mut dyn Storage,
    queue: &Vec<Vec<Msg>>,
    timeout: IbcTimeout,
) -> Result<Vec<IbcMsg>, ContractError>{
    let state = STATE.load(storage)?;
    let mut msgs = Vec::new();
    for (chain_id, msg_queue) in queue.iter().enumerate() {
        //// TESTING ////
        testing_add2queue(storage, chain_id.try_into().unwrap(), msg_queue.to_vec())?;
        //// TESTING ////

        if chain_id != state.chain_id as usize {
            // When chain wishes to send some msgs to dest chain
            if msg_queue.len() > 0 {
                let channel_id = CHANNELS.load(storage, chain_id.try_into().unwrap())?;
                let msg = convert_send_ibc_msg(channel_id, PacketMsg::MsgQueue ( msg_queue.to_vec() ), timeout.clone());
                // let msg = IbcMsg::SendPacket {
                //     channel_id,
                //     data: to_binary(&PacketMsg::MsgQueue ( msg_queue.to_vec() ) )?,
                //     timeout: timeout.clone(),
                // };
                msgs.push(msg);
            }
        }
    }
    //// TESTING /////
    let mut state = STATE.load(storage)?;
    state.current_tx_id += 1;
    STATE.save(storage, &state)?;
    //// TESTING /////

    Ok(msgs)
}
