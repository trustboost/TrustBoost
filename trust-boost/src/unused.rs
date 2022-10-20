fn get_action<T: 'static>(to_push: T) -> Box<dyn Fn(Option<Vec<T>>) -> StdResult<Vec<T>> >{
    Box::new(|ele: Option<Vec<T>>| -> StdResult<Vec<T>> {
        match ele {
            Some(mut p) => {
                p.push(to_push);
                Ok(p)
            },
            None => Ok(vec!(to_push)),
        }
        
    })
}

// execute entry_point is used for beginning new instance of IT-HS consensus
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // let channel_ids = state.channel_ids.clone();
    // let channel_ids = state.channel_ids.values().cloned().collect();
    // let channel_ids: StdResult<Vec<_>> = CHANNELS
    //         .range(deps.storage, None, None, Order::Ascending)
    //         .collect();

    // let mut state = STATE.load(deps.storage)?;
    // let tx_id = state.current_tx_id.clone();

    // // Initialize tx info and store in local state(TXS)
    // TXS.save(
    //     deps.storage,
    //     tx_id.clone(),
    //     &Tx {
    //         msg: msg.clone(),
    //         no_of_votes: 1,
    //     },
    // )?;
    // // Update the tx_id to assign and save current state
    // state.current_tx_id += 1;

    // STATE.save(deps.storage, &state)?;

    // broadcast_response(timeout.clone(), channel_ids, packet, "broadcast_propose".to_string())
}


// macro_rules! enum_to_str {
//     (enum $name:ident {
//         $($variant:ident = $val:expr),*,
//     }) => {
//         enum $name {
//             $($variant = $val),*
//         }

//         impl $name {
//             fn name(&self) -> &'static str {
//                 match self {
//                     $($name::$variant => stringify!($variant)),*
//                 }
//             }
//         }
//     };
// }

pub fn broadcast_submsgs(
    attrib: String,
    timeout: IbcTimeout,
    channel_ids: Vec<String>,
    packet_to_broadcast: PacketMsg,
) -> Result<Response, ContractError> {
    let mut msgs = Vec::new();
    for channel_id in channel_ids {
        let msg = IbcMsg::SendPacket {
            channel_id: channel_id.clone(),
            data: to_binary(&packet_to_broadcast)?,
            timeout: timeout.clone()
        };
        let submsg = SubMsg::reply_on_success(msg, REQUEST_REPLY_ID);
        // let submsg = msg;
        msgs.push(submsg);
    }
    let res = Response::new()
        .add_submessages(msgs)
        .add_attribute("action", attrib);
    Ok(res)
}

pub fn add_broadcast_submsgs(
    mut res: Response,
    timeout: IbcTimeout,
    channel_ids: Vec<String>,
    packet_to_broadcast: PacketMsg,
    submsg_id: u64
) -> Result<Response, ContractError> {
        for channel_id in channel_ids {
            let msg = IbcMsg::SendPacket {
                channel_id: channel_id.clone(),
                data: to_binary(&packet_to_broadcast)?,
                timeout: timeout.clone()
            };
            let submsg = SubMsg::reply_on_success(msg, submsg_id);
            res = res.add_submessage(submsg);
        }
    Ok(res)
}


pub fn add_broadcast_msgs(
    mut res: Response,
    timeout: IbcTimeout,
    channel_ids: Vec<String>,
    packet_to_broadcast: PacketMsg,
) -> Result<Response, ContractError> {
    // let mut res = res;
        for channel_id in channel_ids {
            let msg = IbcMsg::SendPacket {
                channel_id: channel_id.clone(),
                data: to_binary(&packet_to_broadcast)?,
                timeout: timeout.clone()
            };
            res = res.add_message(msg);
        }
    // }
    Ok(res)
}

pub fn broadcast_response(
    timeout: IbcTimeout,
    channel_ids: Vec<(u32, String)>,
    packets_to_broadcast: Vec<PacketMsg>,
    attrib: String,
) -> Result<Response, ContractError> {
    // broadcast Propose message
    let mut msgs: Vec<IbcMsg> = Vec::new();
    for packet in packets_to_broadcast {
        for (_, channel_id) in &channel_ids {
            let msg = IbcMsg::SendPacket {
                channel_id: channel_id.clone(),
                data: to_binary(&packet)?,
                timeout: timeout.clone(),
            };
            msgs.push(msg);
        }
    }

    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", attrib);
    Ok(res)
}

pub fn _create_queue_view_change_backup(
    deps: DepsMut,
    timeout: IbcTimeout,
) -> Result<Vec<IbcMsg>, ContractError> {
    // load the state
    let mut state = STATE.load(deps.storage)?;
    // Add Request message to packets_to_be_broadcasted
    let request_packet = Msg::Request {
        view: state.view,
        chain_id: state.chain_id,
    };

    // Contruct Request messages to be broadcasted
    let channels = get_id_channel_pair(deps.storage)?;
    let proof_packet = Msg::Proof {
        key1: state.key1,
        key1_val: state.key1_val.clone(),
        prev_key1: state.prev_key1,
        view: state.view,
    };
    // let mut msgs: Vec<IbcMsg> = Vec::new();
    let mut queue: Vec<Vec<Msg>> = vec!(vec![request_packet.clone()]; state.n.try_into().unwrap());

    for (chain_id, _channel_id) in &channels {
        // construct the msg queue to send
        // let mut queue = vec![request_packet.clone()];
        let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;

        if *chain_id == state.chain_id {
            if *chain_id == state.primary {
                // Contruct Suggest message
                let suggest_packet = Msg::Suggest {
                    chain_id: state.chain_id,
                    view: state.view,
                    key2: state.key2,
                    key2_val: state.key2_val.clone(),
                    prev_key2: state.prev_key2,
                    key3: state.key3,
                    key3_val: state.key3_val.clone(),
                };
                // self-send Suggest, Proof
                receive_queue(deps.storage, timeout.clone(), None, vec![suggest_packet, proof_packet.clone()], &mut queue)?;
            }
            else{
                // self-send Proof
                receive_queue(deps.storage, timeout.clone(), None, vec![proof_packet.clone()], &mut queue)?;
            }
            
        } else {
            // If dest chain is primary, check if satisfiy condition
            if chain_id.clone() == state.primary {
                // Contruct Suggest message to delivery to primary
                let suggest_packet = Msg::Suggest {
                    chain_id: state.chain_id,
                    view: state.view,
                    key2: state.key2,
                    key2_val: state.key2_val.clone(),
                    prev_key2: state.prev_key2,
                    key3: state.key3,
                    key3_val: state.key3_val.clone(),
                };
                
                // if state.chain_id != state.primary {
                // Upon highest_request[primary] = view
                if highest_request == state.view {
                    // queue.push(suggest_packet);
                    queue[state.primary as usize].push(suggest_packet);
                    
                }
                // } 
                state.sent.insert("Suggest".to_string());
                STATE.save(deps.storage, &state)?;
            }
            // send_all_upon_join(proof)
            if highest_request == state.view {
                queue[*chain_id as usize].push(proof_packet.clone());
            }
        }
        

        // msgs.push(msg);
    }
    let mut msgs = Vec::new();
    for (chain_id, msg_queue) in queue.iter().enumerate() {
        //// TESTING /////
        TEST_QUEUE.save(deps.storage, state.current_tx_id, &(chain_id as u32, msg_queue.to_vec()))?;
        state.current_tx_id += 1;
        STATE.save(deps.storage, &state)?;
        if chain_id != state.chain_id as usize {
            // When chain wish to send some msgs to dest chain
            if msg_queue.len() > 0 {
                let channel_id = CHANNELS.load(deps.storage, chain_id.try_into().unwrap())?;
                let msg = IbcMsg::SendPacket {
                    channel_id,
                    data: to_binary(&PacketMsg::MsgQueue ( msg_queue.to_vec() ) )?,
                    timeout: timeout.clone(),
                };
                msgs.push(msg);
            }
        }
    }
    
    Ok(msgs)
    
}


fn _handle_request_reply(deps: DepsMut, timeout: IbcTimeout, _msg: Reply) -> StdResult<Response> {
    // Upon sucessfully called the broadcast of Request Messages
    // Load the state
    let state = STATE.load(deps.storage)?;
    if state.chain_id != state.primary {
        // Upon highest_request[primary] = view
        let prim_highest_req = HIGHEST_REQ.load(deps.storage, state.primary)?;
        if prim_highest_req == state.view {
            // Contruct Suggest message to delivery to primary
            let packet = Msg::Suggest {
                chain_id: state.chain_id,
                view: state.view,
                key2: state.key2,
                key2_val: state.key2_val.clone(),
                prev_key2: state.prev_key2,
                key3: state.key3,
                key3_val: state.key3_val.clone(),
            };

            let channel_id = CHANNELS.load(deps.storage, state.primary)?;
            let msg = IbcMsg::SendPacket {
                channel_id,
                data: to_binary(&packet)?,
                timeout: timeout.clone(),
            };
            let submsg: SubMsg = SubMsg::reply_on_success(msg, SUGGEST_REPLY_ID);

            // construct Response and put Suggest message in the query on the fly
            return Ok(Response::new()
                .add_submessage(submsg)
                .add_attribute("action", "send_suggest2primary".to_string()));
        }
    }

    // TODO: Add ops for reply of Request message
    Ok(Response::new())
    // Add consecutive submessages
}



/*
fn _acknowledge_request(
    deps: DepsMut,
    env: Env,
) -> StdResult<IbcBasicResponse> {
    
    // Upon sucessfully called the broadcast of Request Messages
    // Load the state 
    let mut state = STATE.load(deps.storage)?;
    if !state.is_first_req_ack {
        return Ok(IbcBasicResponse::new());
    }
    state.is_first_req_ack = false;
    STATE.save(deps.storage, &state)?;
    if state.chain_id != state.primary {
        // Upon highest_request[primary] = view
        let prim_highest_req = HIGHEST_REQ.load(deps.storage, state.primary)?;
        if prim_highest_req == state.view {
            // Contruct Suggest message to delivery to primary
            let packet = PacketMsg::Suggest {
                chain_id: state.chain_id,
                view: state.view,
                key2: state.key2,
                key2_val: state.key2_val.clone(),
                prev_key2: state.prev_key2,
                key3: state.key3,
                key3_val: state.key3_val.clone(),
            };
            // let timeout: IbcTimeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();
            let channel_id = CHANNELS.load(deps.storage, state.primary)?;
            let timeout = get_timeout(env);
            let msg = IbcMsg::SendPacket {
                channel_id,
                data: to_binary(&packet)?,
                timeout: timeout.clone(),
            };
            let submsg = SubMsg::reply_on_success(msg, SUGGEST_REPLY_ID);
            // let submsg = msg;
            // construct Response and put Suggest message in the query on the fly
            return Ok(IbcBasicResponse::new()
                .add_submessage(submsg)
                .add_attribute("action", "send_suggest2primary".to_string()))
        }
    }
    Ok(IbcBasicResponse::new())
}

fn acknowledge_propose(
    _deps: DepsMut,
    env: Env,
    ack: AcknowledgementMsg<ProposeResponse>,
) -> StdResult<IbcBasicResponse> {
    let _timeout: IbcTimeout = get_timeout(env);
    // retrive tx_id from acknowledge message
    let _tx_id = match ack {
        AcknowledgementMsg::Ok(res) => res,
        AcknowledgementMsg::Err(e) => {
            return Ok(IbcBasicResponse::new()
                .add_attribute("action", "acknowledge_propose")
                .add_attribute("error", e))
        }
    };
    // let action = |tx: Option<Tx>| -> StdResult<Tx> {
    //     let mut tx = tx.unwrap();
    //     tx.no_of_votes += 1;
    //     Ok(tx)
    // };

    // let tx = TXS.update(deps.storage, tx_id.clone(), action)?;

    // broadcast Commit message
    // if tx.no_of_votes >= 2 {
    //     // let state: State = STATE.load(deps.storage)?;
    //     // let channel_ids = state.channel_ids.clone();
    //     let channel_ids: StdResult<Vec<_>> = CHANNELS
    //         .range(deps.storage, None, None, Order::Ascending)
    //         .collect();
    //     let channel_ids = channel_ids?;
    //     let packet = PacketMsg::Commit {
    //         msg: tx.msg.clone(),
    //         tx_id: tx_id.clone(),
    //     };

    //     receive_commit(deps, "self".to_string(), tx.msg.clone(), tx_id.clone())?;

    //     // Broadcast Commit messages
    //     let mut commit_msgs: Vec<IbcMsg> = Vec::new();
    //     for (_, channel_id) in channel_ids {
    //         let msg = IbcMsg::SendPacket {
    //             channel_id: channel_id.clone(),
    //             data: to_binary(&packet)?,
    //             timeout: timeout.clone(),
    //         };
    //         commit_msgs.push(msg);
    //     }

    // let msg0 = IbcMsg::SendPacket {
    //     channel_id: channel_ids[0].clone(),
    //     data: to_binary(&packet)?,
    //     timeout: timeout.clone()
    // };
    // let msg1 = IbcMsg::SendPacket {
    //     channel_id: channel_ids[1].clone(),
    //     data: to_binary(&packet)?,
    //     timeout: timeout.clone()
    // };

    //     Ok(IbcBasicResponse::new()
    //         // .add_message(msg0)
    //         // .add_message(msg1)
    //         .add_messages(commit_msgs)
    //         .add_attribute("action", "acknowledge_propose_response")
    //         .add_attribute("commit", "true"))
    // } else {
    Ok(IbcBasicResponse::new()
        .add_attribute("action", "acknowledge_propose_response")
        .add_attribute("commit", "false"))
    // }
}
*/




// pub fn send_all_upon_join_sub(
//     deps: &DepsMut,
//     timeout: IbcTimeout,
//     mut res: Response,
//     packet_to_broadcast: PacketMsg,
//     reply_id: u64
// ) -> Result<Response, ContractError> {
//     let channel_ids = get_id_channel_pair(deps.storage)?;
//     // let mut res = res;
//     let state = STATE.load(deps.storage)?;
//     for (chain_id, channel_id) in &channel_ids {
//         let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
//         if highest_request == state.view {
//             let msg = IbcMsg::SendPacket {
//                 channel_id: channel_id.clone(),
//                 data: to_binary(&packet_to_broadcast)?,
//                 timeout: timeout.clone(),
//             };
//             let submsg = SubMsg::reply_on_success(msg, reply_id);
//             res = res.add_submessage(submsg);
//         }
//     }

//     Ok(res)
// }

// pub fn send_all_upon_join(
//     deps: &DepsMut,
//     timeout: IbcTimeout,
//     packet_to_broadcast: PacketMsg,
// ) -> Result<Vec<SubMsg>, ContractError> {
//     let channel_ids = get_id_channel_pair(deps.storage)?;

//     let mut msgs = Vec::new();
//     let state = STATE.load(deps.storage)?;
//     for (chain_id, channel_id) in &channel_ids {
//         let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
//         if highest_request == state.view {
//             let msg = IbcMsg::SendPacket {
//                 channel_id: channel_id.clone(),
//                 data: to_binary(&packet_to_broadcast)?,
//                 timeout: timeout.clone(),
//             };
//             let submsg: SubMsg = SubMsg::reply_on_success(msg, PROPOSE_REPLY_ID);
//             msgs.push(submsg);
//         }
//     }

//     Ok(msgs)
// }
