use cosmwasm_std::{
    StdResult, Order, StdError, Storage, IbcTimeout, Env, Api
};

use crate::utils::{get_id_channel_pair_from_storage};
use crate::state::{
    STATE, HIGHEST_ABORT, DEBUG
};


use crate::utils::{
    reset_view_specific_maps
};

use crate::view_change::{
    append_queue_view_change
};

use crate::ibc_msg::Msg;

pub fn handle_abort(storage: &mut dyn Storage, 
                    queue: &mut Vec<Vec<Msg>>, view: u32, 
                    sender_chain_id: u32, timeout: IbcTimeout,
                    env: &Env,
                    api: &dyn Api,
                    ) -> Result<(), StdError> {
    let mut state = STATE.load(storage)?;
    
    let mut loaded_val: i32 = 0;
    let option = HIGHEST_ABORT.load(storage, sender_chain_id);
    match option {
        Ok(val) => loaded_val = val,
        Err(_) => return Err(StdError::GenericErr { msg: "handle_abort cannot find loadedVal".to_string()} ), 
    }

    if ((loaded_val + 1) as u32)< (view+1) {
        HIGHEST_ABORT.update(storage, sender_chain_id, |option| -> StdResult<i32> {
            match option {
                Some(_val) => Ok(view as i32),
                None => Ok(view as i32),
            }
        })?;

        let highest_abort_vector_pair: StdResult<Vec<_>> = HIGHEST_ABORT
            .range(storage, None, None, Order::Ascending)
            .collect();
        let mut vector_values = match highest_abort_vector_pair {
            Ok(vec) => { 
                let temp = vec.iter().map(|(_key, value)| value.clone()).collect::<Vec<i32>>();
                temp
            }
            Err(_) => return Err(StdError::GenericErr { msg: "Error nth".to_string()}),
        };
        vector_values.sort();

        // Sort will sort the array ascendingly... [-1,0,-1,-1] --> [-1,-1,-1,0]..
        // F+1 highest meaning n-F+1
        let u = vector_values[ (vector_values.len()-(state.F+1) as usize)]; 
        let mut loaded_val: i32 = 0;
        match HIGHEST_ABORT.load(storage, sender_chain_id) {
            Ok(val) => loaded_val = val,
            Err(_) => return Err(StdError::GenericErr { msg: "handle_abort cannot find loaded_val part 2".to_string()} ), 
        }

        if u > loaded_val {
            if u > -1 {
                let abort_packet = Msg::Abort { view: u as u32, chain_id: state.chain_id};
                let channel_ids = get_id_channel_pair_from_storage(storage)?;
                DEBUG.save(storage, 1200, &"CLONE_ABORT_PACKET".to_string())?;
                for (chain_id, _channel_id) in &channel_ids {
                    queue[*chain_id as usize].push(abort_packet.clone());
                }
                HIGHEST_ABORT.update(storage, sender_chain_id, |option| -> StdResult<i32> {
                    match option {
                        Some(_val) => Ok(u),
                        None => Ok(u),
                    }
                })?;
            }
        }

        let highest_abort_vector_pair: StdResult<Vec<_>> = HIGHEST_ABORT
            .range(storage, None, None, Order::Ascending)
            .collect();
        let mut vector_values = match highest_abort_vector_pair {
            Ok(vecs) => { 
                let temp = vecs.iter().map(|(_key, value)| value.clone()).collect::<Vec<i32>>();
                temp
            }
            Err(msg) => return Err(StdError::GenericErr { msg: msg.to_string()} ),
        };
        vector_values.sort();        

        // Sort will sort the array ascendingly... [-1,0,-1,-1] --> [-1,-1,-1,0]
        let idx = vector_values.len()-(state.n-state.F) as usize;
        // println!("state.n is {} F is {} vector_values size is {} idx is {}", state.n, F, vector_values.len(), idx);
        let w = vector_values[idx as usize];
        if (w+1) as u32 >= state.view {
            let previous_view = state.view;
            state.view = (w + 1) as u32;
            state.primary = (state.view % state.n) + 1;
            state.start_time = env.block.time;
            STATE.save(storage, &state)?;
            if previous_view != state.view {
                DEBUG.save(storage, 1300, &"TRIGGER_VIEW_CHANGE_NEW".to_string())?;
                match reset_view_specific_maps(storage) {
                    Ok(_) => {
                        
                    }
                    Err(_) => {
                        // println!("Error when reseting view maps in handle_abort");
                        return Err(StdError::GenericErr { msg: "Error when reseting view maps in handle_abort".to_string()} )
                    }
                }         
                
                let result = append_queue_view_change(storage, queue, timeout, env, api);
                match result {
                    Ok(_) => {

                    }
                    Err(msg) => {
                        //println!("Error when doing view_change in handle_abort ");
                        return Err(StdError::GenericErr { msg: msg.to_string()} )
                        // return Ok(())
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use std::error::Error;

    // https://docs.cosmwasm.com/tutorials/trust-boost/testing/
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coins, from_binary, OwnedDeps};
    use crate::state::{HIGHEST_ABORT, STATE, State};

    // #[test]
    // fn test_abort() {
    //     let mut deps = mock_dependencies();
    //     let mut env = mock_env();
    //     let mut _info = mock_info(&"test".to_string(), 
    //     &coins(2, "token"));

    //     let storage = deps.as_mut().storage;
    //     let mut queue: Vec<Vec<Msg>> = vec!(vec![]; 4);

    //     //init..
    //     let mut mock_state = State::new(0, "test_abort".to_string(), "".into(), env.block.time);
    //     mock_state.n = 4;
    //     STATE.save(storage, &mock_state);
    //     HIGHEST_ABORT.update(storage, 0, |_| -> StdResult<_> {Ok(-1)});
    //     HIGHEST_ABORT.update(storage, 1, |_| -> StdResult<_> {Ok(-1)});
    //     HIGHEST_ABORT.update(storage, 2, |_| -> StdResult<_> {Ok(-1)});
    //     HIGHEST_ABORT.update(storage, 3, |_| -> StdResult<_> {Ok(-1)});
    //     //init..
    //     assert_eq!(mock_state.view, 0);


    //     let result = handle_abort(storage, & mut queue, 0, 1, env.block.time.into(), &env);
    //     match result {
    //         Ok(_) => (),
    //         Err(msg) => {
    //             let cause = msg.source().unwrap();
    //             panic!(msg.to_string())
    //         }
    //     }
    //     let mut state = STATE.load(storage).unwrap();
    //     let mut abort0 = HIGHEST_ABORT.load(storage, 0).unwrap();
    //     let mut abort1 = HIGHEST_ABORT.load(storage, 1).unwrap();
    //     let mut abort2 = HIGHEST_ABORT.load(storage,2).unwrap();
    //     let mut abort3 = HIGHEST_ABORT.load(storage,3).unwrap();
    //     assert_eq!(state.view, 0);
    //     assert_eq!(abort0, -1);
    //     assert_eq!(abort1, 0);
    //     assert_eq!(abort2, -1);
    //     assert_eq!(abort3, -1);

    //     let result = handle_abort(storage, & mut queue ,0, 0, env.block.time.into(), &env);
    //     match result {
    //         Ok(_) => (),
    //         Err(msg) => {
    //             let cause = msg.source();
    //             match cause {
    //                 Some(error_cause) => panic!(error_cause.to_string()),
    //                 None => panic!("None when returning handle_abort"),
    //             }
    //         }
    //     }
        
    //     let mut state = STATE.load(storage).unwrap();
    //     let mut abort0 = HIGHEST_ABORT.load(storage, 0).unwrap();
    //     let mut abort1 = HIGHEST_ABORT.load(storage, 1).unwrap();
    //     let mut abort2 = HIGHEST_ABORT.load(storage,2).unwrap();
    //     let mut abort3 = HIGHEST_ABORT.load(storage,3).unwrap();
    //     assert_eq!(state.view, 1);
    //     assert_eq!(abort0, 0);
    //     assert_eq!(abort1, 0);
    //     assert_eq!(abort2, -1);
    //     assert_eq!(abort3, -1);
        
    // }


}