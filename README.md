# TrustBoost

Proof of concept demo for trustboost

## Installation

- Use the package manager [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) to install dependencies and crates required to run Rust code and unit tests
- Install wasmd (use version 0.28.0 https://github.com/CosmWasm/wasmd/tree/v0.28.0)
- install relayer follow this branch for the one with shorter relay delay (https://github.com/sdgs72/relayer)
    - just clone the repo and run <code> make install </code>

## Usage

<h2> Start Chains and Relayer </h2>

Note that chain/node indexes start at 0 and ends at n-1 (in the 7 chains case, the last chain will be ibc-6)

To start the system use the ./start script, and specify the number of chains(nodes), that you want to run. This will automatically setup all the relayers, deploy all the smartcontracts(Nameservice and trustboost) and also run the relayers. Right now only size 3,4,7 and 10 are supported
```bash
# start 3 chains
./start 3

# start 4 chains
./start 4

# start 7 chains
./start 7
```

<h2> Getting balances of Relayer before starting </h2>

To get the balances of the relayers use this command <code>./helper queryRelayerBalanceMany $(nodeCount) </code> 

use this for getting the relayer wallet in one chain <code> ./helper queryRelayerBalance $(chainIndex) </code>

Note that the relayer wallet in a chain will be used by many relayers IE: in a 3 chain setup we have 3 relayers connecting chains chain0-chain1, chain1-chain2 and chain0-chain2. The relayer wallet at chain-0 will be used by relayers that will be forwarding 0-1 and 0-2 and will be used to pay transactions to persist IBC messages from chain-1 and chain-2 to chain-0.

```bash
# will list out the balances of the relayer wallet from chain-0,1,2,3
./helper queryRelayerBalanceMany 4

# will list out the balances of the relayer wallet from chain-3
./helper queryRelayerBalance 3
```


<h2> Getting balances of User before starting </h2>

To get the balances of the relayers use this command <code>./helper queryUserBalanceMany $(nodeCount) </code> 

```bash
# will list out the balances of the relayer wallet from chain-0,1,2,3
./helper queryUserBalanceMany 3
```



<h2> Start Request </h2>


After starting the chains and the relayer, please wait for ~1-2 min, since after setup the relayer needs to forward some IBC setup messages between the trustboost contrast across diferent chains.

Next to start the input use the following ./helper inputMany (nodecount) command. This will loop over to the number of node count specified and start sending the request. There is a 15 second delay between each call to a chain to prevent account sequencing errors in the CLI.

The input will start, starting at 1 since this is usually the primary and we want the primary to start first otherwise some of the IBC message might get dropped and the process will be stuck if the primary started late.

```bash
# (3 chains 1 faulty, special case)
./helper inputMany 2

# (4 chains 1 faulty, so only input to 3 chains)
./helper inputMany 3

#  (7 chains 2 faulty, so only input to 5 chains)
./helper inputMany 5
```

Wait for some time (~ 5 minute) for the state to converge use the next commands to check. (for 7/10 chains might take more time then ~5 minute)

<h2> Getting Trustboost contract state </h2>

use the the command to print the state of many nodes. ./helper queryStateMany (nodeNumber). (use <code>./helper queryState $targetNode</code> to only execute for one chain)
```bash 
# will list out the state of chain-0,chain-1,chain-2. It will show the done timestamp if it is finished. But please use the resolveRecordMany instead to get the final contract time stamp
./helper queryStateMany 2

#To show the state for all 4 chains
./helper queryStateMany 4

#To show the state for all 7 chains
./helper queryStateMany 7

```

Example output for ONE chain(in queryStateMany it will show the result for many chains) after it is done (if it is not done, it will just show a big blob of progress), otherwise it will show a big blob of state

```bash

  Done:
    block_height: 473  # height when the target is mined
    decided_timestamp: "1665373301539366000" #when the target smart contract will be executed
    decided_val: eyJyZWdpc3Rlcl90YiI6eyJuYW1lIjoidGVzdF9mcm9tX3RydXN0Ym9vc3Rfc2VwdCJ9fQ== # decided value
    minutes_duration: 1 # DONT USE THIS CAN BE FAULTY DUE TO START TIME ERROR
    seconds_duration: 97 # DONT USE THIS CAN BE FAULTY DUE TO START TIME ERROR
    start_time: "1665373204826996000" # DONT USE THIS CAN BE FAULTY DUE TO START TIME ERROR

```

<h2> Getting State Name service state</h2>


use the following command <code>./helper resolveRecord $(targetNode)</code> to get the end time, the owner should be filled and the timestamp should be there if it is null
```bash
./helper resolveRecordMany 5
```

<h2> Resetting smart contract state </h2>

To reset use this command ./helper resetMany (nodeCount). This command will some time (60 seconds delay per chain). To prevent sequencing error. Please check the state using queryStateMany after the reset has finished and make sure that the value of the keys are "RESET_TB" and the signature fields are all empty, otherwise try resetting again. (Usually there are some messages in the relayer that are forwarded a bit late that cause the state to be dirty after being reset).
```bash
# if you start 4 chains use this to reset all of them
./helper resetMany 4

# if you start 7 chains use this to reset all of them
./helper resetMany 7
```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)