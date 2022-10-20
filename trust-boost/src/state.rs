use std::collections::{HashSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};


use cosmwasm_std::{IbcMsg, Timestamp, SubMsg, Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map, PrimaryKey, Key};

use crate::{ibc_msg::Msg};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Hash)]
#[serde(rename_all = "snake_case")]
pub struct TBInput {
    pub binary: String,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

impl TBInput {

    pub fn calculate_hash(self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }
}


pub type InputType = TBInput;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    // pub role: String,
    pub n: u32,
    pub chain_id: u32,
    pub channel_ids: Vec<String>,
    pub current_tx_id: u32,
    pub view: u32,
    pub primary: u32,
    pub key1: u32,
    pub key2: u32,
    pub key3: u32,
    pub lock: u32,
    pub key1_val: InputType,
    pub key2_val: InputType,
    pub key3_val: InputType,
    pub lock_val: InputType,

    pub prev_key1: i32,
    pub prev_key2: i32,

    pub suggestions: Vec<(u32, InputType)>,
    pub key2_proofs: Vec<(u32, InputType, i32)>,
    pub proofs: Vec<(u32, InputType, i32)>,
    pub received_propose: bool,
    // pub is_first_req_ack: bool,
    // pub sent_suggest: bool,
    // pub sent_done: bool,
    pub sent: HashSet<String>,
    pub done: Option<InputType>,
    pub start_time: Timestamp,
    pub contract_addr: Addr,
    pub done_executed:bool,
    pub done_timestamp: Option<Timestamp>,
    pub done_block_height: Option<u64>,
    pub F: u32,
}

impl State {
    // Another associated function, taking two arguments:
    pub(crate) fn new(chain_id: u32, input: InputType, contract_addr: Addr, start_time: Timestamp) -> Self {
        Self {
            n: 1,
            chain_id,
            channel_ids: Vec::new(),
            current_tx_id: 0,
            view: 0,
            primary: 1,
            key1: 0,
            key2: 0,
            key3: 0,
            lock: 0,
            key1_val: input.clone(),
            key2_val: input.clone(),
            key3_val: input.clone(),
            lock_val: input.clone(),
            prev_key1: -1,
            prev_key2: -1,
            suggestions: Vec::new(),
            key2_proofs: Vec::new(),
            proofs: Vec::new(),
            received_propose: false,
            // is_first_req_ack: true,
            // sent_suggest: false,
            // sent_done: false,
            sent: HashSet::new(),
            done: None,
            start_time,
            contract_addr,
            done_executed: false,
            done_timestamp: None,
            done_block_height: None,
            F: 0,
        }
    }
    pub(crate) fn re_init(&mut self, input: InputType, start_time: Timestamp) -> () {
        self.sent = HashSet::new();
        self.done = None;
        self.view = 0;
        self.key1 = 0;
        self.key2 = 0;
        self.key3 = 0;
        self.lock = 0;
        self.prev_key1 = -1;
        self.prev_key2 = -1;
        self.key1_val = input.clone();
        self.key2_val = input.clone();
        self.key3_val = input.clone();
        self.lock_val = input.clone();
        // Set suggestions and key2_proofs to empty set
        self.suggestions = Vec::new();
        self.key2_proofs = Vec::new();

        // Use block time..
        self.start_time = start_time;

        // Set the primary to be (view mod n) + 1
        self.primary = self.view % self.n + 1;

        ////    process_messages() part     ////
        // initialize proofs to an empty set
        self.proofs = Vec::new();

        // reset values
        self.received_propose = false;
        self.done_executed = false;
        self.done_timestamp = None;
        self.done_block_height = None;
        if (self.n == 3) {
            self.F = 1;
        } else {
            self.F = (self.n-1)/3;
        }
        ()

    }


}


pub const STATE: Item<State> = Item::new("state");
pub const CHANNELS: Map<u32, String> = Map::new("channels");

pub const HIGHEST_REQ: Map<u32, u32> = Map::new("highest_req");
pub const HIGHEST_ABORT: Map<u32, i32> = Map::new("highest_abort");

pub const SEND_ALL_UPON: Map<u32, Vec<Msg>> = Map::new("send_all_upon");

// FOR DEDUPING MESSAGES <Channel_Id, has_received_the_message_before>
pub const RECEIVED: Map<String, HashSet<u32>> = Map::new("received");
// pub const RECEIVED_SUGGEST: Map<String, HashSet<u32>> = Map::new("received_suggest");
// pub const RECEIVED_PROOF: Map<String, HashSet<u32>> = Map::new("received_proof");
pub const RECEIVED_ECHO: Map<u64, HashSet<u32>> = Map::new("received_echo");
pub const RECEIVED_KEY1: Map<u64, HashSet<u32>> = Map::new("received_key1");
pub const RECEIVED_KEY2: Map<u64, HashSet<u32>> = Map::new("received_key2");
pub const RECEIVED_KEY3: Map<u64, HashSet<u32>> = Map::new("received_key3");
pub const RECEIVED_LOCK: Map<u64, HashSet<u32>> = Map::new("received_lock");
pub const RECEIVED_DONE: Map<u64, HashSet<u32>> = Map::new("received_done");


//// TESTING.. ////
pub const TEST: Map<u32, Vec<IbcMsg>> = Map::new("test");
pub const TEST_QUEUE: Map<u32, Vec<(u32, Vec<Msg>)> > = Map::new("test_queue");
pub const DEBUG: Map<u32, String> = Map::new("debug");
pub const IBC_MSG_SEND_DEBUG: Map<String, Vec<SubMsg>> = Map::new("ibc_msg_send_debug");
pub const DEBUG_CTR: Item<u32> = Item::new("DEBUG_CTR");
pub const DEBUG_RECEIVE_MSG: Map<String, Vec<String>> = Map::new("DEBUG_RECEIVE_MSG");


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Test {
    pub src_port: String,
    pub src_chan_id: String,
    pub dest_port: String,
    pub dest_chan_id: String,
}

