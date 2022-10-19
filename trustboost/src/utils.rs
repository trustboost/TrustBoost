use std::collections::HashSet;

use cosmwasm_std::{
    StdResult, Order, IbcTimeout, Env, IbcOrder, StdError, IbcChannelOpenMsg, Storage, IbcMsg, to_binary, Addr, Binary, Deps, Api, Timestamp
};

use crate::ibc_msg::{
    Msg, PacketMsg
};

use sha2::{Digest, Sha256};
use bech32::ToBase32;
use ripemd::{Digest as RipDigest, Ripemd160};

use cw_storage_plus::{Map};
use crate::state::{
    CHANNELS, SEND_ALL_UPON, STATE, HIGHEST_REQ, HIGHEST_ABORT, RECEIVED, RECEIVED_ECHO, RECEIVED_KEY1, RECEIVED_KEY2, RECEIVED_KEY3, RECEIVED_LOCK, TEST_QUEUE,RECEIVED_DONE, InputType
};

/// Setting the lifetime of packets to be one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;
/// Setting up constant
pub const IBC_APP_VERSION: &str = "simple_storage";


use crate::ContractError;

pub fn get_chain_id(store: &mut dyn Storage, channel_id_to_get: String) -> u32 {
    // Get the chain_id of the sender
    CHANNELS
    .range(store, None, None, Order::Ascending)
    .find_map(|res| { 
        let (chain_id,channel_id) = res.unwrap(); 
        if channel_id == channel_id_to_get { 
            Some(chain_id) 
        } 
        else { None }
    }).unwrap()
}

// reset views for a new "Instance" of the IT-HS algorithm
pub fn init_receive_map(store: &mut dyn Storage) -> StdResult<()> {
    let state = STATE.load(store)?;
    // Initialize highest_request (all to the max of u32 to differentiate between the initial state)
    let all_chain_ids: StdResult<Vec<_>> = CHANNELS
        .keys(store, None, None, Order::Ascending)
        .collect();
    let all_chain_ids = all_chain_ids?;
    // initialize the highest_request of oneself
    HIGHEST_REQ.save(store, state.chain_id, &0)?;
    // initialize the highest_abort of oneself
    for chain_id in all_chain_ids {
        HIGHEST_REQ.save(store, chain_id, &0)?;
        // Resetting highest_abort
        // RECEIVED_SUGGEST.save(store, chain_id, &false)?;
        // RECEIVED_PROOF.save(store, chain_id, &false)?;
    }
    
    reset_view_specific_maps(store)?;
    reset_aborts(store)?;
    Ok(())
}

// Reset maps that are specific to views...
pub fn reset_view_specific_maps(store: &mut dyn Storage) -> StdResult<()> {

        // remove all records for previous values
    let msg_types: StdResult<Vec<_>> = RECEIVED
        .keys(store, None, None, Order::Ascending)
        .collect();
    for msg_type in msg_types? {
        RECEIVED.save(store, msg_type, &HashSet::new())?;
    }

    delete_map(store, RECEIVED_ECHO)?;
    delete_map(store, RECEIVED_KEY1)?;
    delete_map(store, RECEIVED_KEY2)?;
    delete_map(store, RECEIVED_KEY3)?;
    delete_map(store, RECEIVED_LOCK)?;
    delete_map(store, RECEIVED_DONE)?;

    //// TESTING ////
    let keys: StdResult<Vec<_>> = TEST_QUEUE
        .keys(store, None, None, Order::Ascending)
        .collect();
    for v in keys? {
        TEST_QUEUE.remove(store, v);
    }       
    //// TESTING ////

    Ok(())
}

fn reset_aborts(store: &mut dyn Storage) -> StdResult<()> {
    let state = STATE.load(store)?;
    // Initialize highest_request (all to the max of u32 to differentiate between the initial state)
    let all_chain_ids: StdResult<Vec<_>> = CHANNELS
    .keys(store, None, None, Order::Ascending)
    .collect();
    let all_chain_ids = all_chain_ids?;
    HIGHEST_ABORT.save(store, state.chain_id, &-1)?;
    for chain_id in all_chain_ids {
        // Resetting highest_abort
        HIGHEST_ABORT.save(store, chain_id, &-1)?;
    }
    Ok(())
}

fn delete_map(store: &mut dyn Storage, map: Map<u64, HashSet<u32>>)  -> StdResult<()> {
    let vals: StdResult<Vec<_>> = map
        .keys(store, None, None, Order::Ascending)
        .collect();
    for v in vals? {
        map.remove(store, v);
    }       
    Ok(())
}


pub fn get_timeout(env: &Env) -> IbcTimeout {
    env.block.time.plus_seconds(PACKET_LIFETIME).into()
}


pub fn get_id_channel_pair(store: &mut dyn Storage) -> StdResult<Vec<(u32, String)>> {
    let channels: StdResult<Vec<_>> = CHANNELS
        .range(store, None, None, Order::Ascending)
        .collect();
    channels
}

pub fn get_id_channel_pair_from_storage(storage: &mut dyn Storage) -> StdResult<Vec<(u32, String)>> {
    let channels: StdResult<Vec<_>> = CHANNELS
        .range(storage, None, None, Order::Ascending)
        .collect();
    channels
}

pub fn send_all_upon_join_queue(storage: &mut dyn Storage, packet_to_broadcast: Msg, 
                                queue: &mut Vec<Vec<Msg>>) -> Result<(), ContractError> {
    let channel_ids = get_id_channel_pair_from_storage(storage)?;
    let state = STATE.load(storage)?;
    for (chain_id, _channel_id) in &channel_ids {
        let highest_request = HIGHEST_REQ.load(storage, chain_id.clone())?;
        if highest_request == state.view {
            queue[*chain_id as usize].push(packet_to_broadcast.clone());
        }
        else{
            let action = |packets: Option<Vec<Msg>>| -> StdResult<Vec<Msg>> {
                match packets {
                    Some(_) => todo!(),
                    None => Ok(vec!(packet_to_broadcast.clone())),
                }
                
            };
            SEND_ALL_UPON.update(storage, *chain_id, action)?;
        }
    }
    Ok(())
}

fn _verify_channel(msg: IbcChannelOpenMsg) -> StdResult<()> {
    let channel = msg.channel();

    if channel.order != IbcOrder::Ordered {
        return Err(StdError::generic_err("Only supports ordered channels"));
    }
    if channel.version.as_str() != IBC_APP_VERSION {
        return Err(StdError::generic_err(format!(
            "Must set version to `{}`",
            IBC_APP_VERSION
        )));
    }
    if let Some(counter_version) = msg.counterparty_version() {
        if counter_version != IBC_APP_VERSION {
            return Err(StdError::generic_err(format!(
                "Counterparty version must be `{}`",
                IBC_APP_VERSION
            )));
        }
    }

    Ok(())
}

pub fn convert_send_ibc_msg(channel_id: String, packet: PacketMsg, timeout: IbcTimeout) -> IbcMsg {
    IbcMsg::SendPacket {
        channel_id,
        data: to_binary(&packet).unwrap(),
        timeout,
    }
}

pub fn derive_addr_from_pubkey(pub_key_bytes: &[u8]) -> Result<Addr, ContractError> {

   // derive external address for merkle proof check
    let sha_hash = Sha256::digest(pub_key_bytes);

    if sha_hash.len() != 32 {
         return Err(ContractError::CustomError { val: "INVALID_LENGTH NOT 32".to_string() });
    }
    
    let rip_hash = Ripemd160::digest(sha_hash);
    let rip_slice: &[u8] = rip_hash.as_slice();

    let addr: String = bech32::encode("wasm", rip_slice.to_base32(), bech32::Variant::Bech32)
        .map_err(|err| ContractError::CustomError { val: err.to_string() })?;

    let addr = cosmwasm_std::Addr::unchecked(addr);
    Ok(addr)
}

pub fn append_binary_string(binaryString: String, key: &String, value: &String) -> Binary {
    let binary = Binary::from_base64(&binaryString).unwrap();;
    
    let mut binaryVector = binary.0.to_vec();

    // Pop last two brackets
    binaryVector.pop();
    binaryVector.pop();

    binaryVector.push(b',');
    binaryVector.push(b'"');
    for elem in key.chars() {
        binaryVector.push(elem as u8);
    }
    binaryVector.push(b'"');
    binaryVector.push(b':');
    binaryVector.push(b'"');
    for elem in value.chars() {
        binaryVector.push(elem as u8);
    }
    binaryVector.push(b'"');
    binaryVector.push(b'}');
    binaryVector.push(b'}');

    Binary(binaryVector)
}

pub fn check_signature(api: &dyn Api, val: InputType) -> bool {
    let mut result: Vec<bool> = Vec::new();

    // Hashing
    let hash = Sha256::digest(val.binary);

    // Verification
    let verify_result = api
        .secp256k1_verify(hash.as_ref(), &val.signature, &val.public_key);

    verify_result.unwrap()
}


pub fn get_seconds_diff(start: &Timestamp, end: &Timestamp) -> u64 {
    return end.seconds()-start.seconds();
} 



