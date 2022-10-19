use std::{collections::HashSet, fmt, str};

use cosmwasm_std::{Timestamp, to_binary, Binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ibc_msg::Msg, state::{State, InputType}};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub chain_id: u32,
    pub input: InputType,
    pub contract_addr: String,
    // pub msg: ContractExecuteMsg
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Input { value: InputType },
    PreInput { value: InputType},
    ForceAbort {},
    Abort {},
    Trigger { behavior: String },
    Key3 {val: InputType,view: u32,local_channel_id: String},
    Lock {val: InputType,view: u32,local_channel_id: String},
    Done {val: InputType,view: u32,local_channel_id: String},
    SetContractAddr {addr: String},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// GetValue querys value for given key, GetState returns the current state, GetTx returns tx with tx_id
    GetState { },
    GetStateProgress { },
    GetChannels { },
    GetTest { },
    GetHighestReq { },
    GetHighestAbort { },
    GetReceivedSuggest { },
    GetSendAllUpon { },
    GetTestQueue { },
    GetEcho { },
    GetKey1 { },
    GetKey2 { },
    GetKey3 { },
    GetLock { },
    GetDone { },
    GetAbortInfo { },
    GetDebug { },
    GetIbcDebug {},
    GetDebugReceive {},
    CheckSignature {
        val: InputType
    },
    GetAddress {
        val: InputType
    }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ValueResponse {
    KeyFound {
        key: String,
        value: String
    },
    KeyNotFound {
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum StateResponse {
    InProgress {
        state: State
    },
    Done {
        decided_val: String,
        decided_timestamp: Option<Timestamp>,
        block_height: Option<u64>,
        start_time: Timestamp,
        seconds_duration: Option<u64>,
        minutes_duration: Option<u64>,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ChannelsResponse {
    pub port_chan_pair: Vec<(u32,String)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HighestReqResponse {
    pub highest_request: Vec<(u32, u32)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReceivedSuggestResponse {
    pub received_suggest: HashSet<u32>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SendAllUponResponse {
    pub send_all_upon: Vec<(u32, Vec<Msg>)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TestQueueResponse {
    pub test_queue: Vec<(u32, Vec<(u32, Vec<Msg>)>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EchoQueryResponse { 
    pub echo: Vec<(u64, HashSet<u32>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key1QueryResponse { 
    pub key1: Vec<(u64, HashSet<u32>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key2QueryResponse { 
    pub key2: Vec<(u64, HashSet<u32>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key3QueryResponse { 
    pub key3: Vec<(u64, HashSet<u32>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockQueryResponse { 
    pub lock: Vec<(u64, HashSet<u32>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DoneQueryResponse { 
    pub done: Vec<(u64, HashSet<u32>)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AbortResponse { 
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub current_time: Timestamp,
    pub is_timeout: bool,
    pub done: bool,
    pub should_abort: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DebugResponse { 
    pub debug: Vec<(u32, String)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HighestAbortResponse {
    pub highest_abort: Vec<(u32, i32)>
}
