// https://github.com/cosmos/cosmjs/blob/main/packages/crypto/src/secp256k1.spec.ts#L94
// https://gist.github.com/webmaster128/8444d42a7eceeda2544c8a59fbd7e1d9
// https://discord.com/channels/737637324434833438/737640557400293416/995734411607801926

// {"name":"user","type":"local","address":"wasm1pfu4h0g9ye26nlm2vay8m747pwc3quhsh5c94p","pubkey":"{\"@type\":\"/cosmos.crypto.secp256k1.PubKey\",\"key\":\"A3Xa2cxsCqe0bTV21H1ZmYlrwOCJzkrNwM59SZaz+kbz\"}","mnemonic":"leave unfold dance spread blast auto gadget sing shield silk garlic away mean type memory town someone language bronze whale nut among praise bone"}

import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { Secp256k1, Slip10, Slip10Curve, Secp256k1Signature, ExtendedSecp256k1Signature, sha256} from "@cosmjs/crypto"; 

const mnemonic = "leave unfold dance spread blast auto gadget sing shield silk garlic away mean type memory town someone language bronze whale nut among praise bone";

const wallet_options = {
  bip39Password: "",
  prefix: "wasm",
};

const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, wallet_options);


console.log("hello world!");

console.log(wallet);

let senderAddress = (await wallet.getAccounts())[0].address;
console.log(senderAddress);
let account_detail = (await wallet.getAccounts())[0];
console.log(account_detail);

const { privkey } = Slip10.derivePath(Slip10Curve.Secp256k1, wallet.seed, wallet.accounts[0].hdPath);
const keyPair = await Secp256k1.makeKeypair(privkey);

console.log("keyPair");
console.log(keyPair);

const compressedPubKey = Secp256k1.compressPubkey(keyPair.pubkey);
console.log("compressedPubKey");
console.log(compressedPubKey);
console.log("PRIVATE_KEY");
console.log(privkey);

// https://github.com/CosmWasm/cosmwasm/tree/main/contracts/crypto-verify
// Signature: Serialized signature, in "compact" Cosmos format (64 bytes). Ethereum DER needs to be converted.

// Need to figure out hash in smart contract https://github.com/CosmWasm/cosmwasm/blob/main/packages/crypto/src/secp256k1.rs#L28-L64
// SHA256 hashing inside a smart contract https://github.com/CosmWasm/cosmwasm/blob/main/contracts/crypto-verify/src/contract.rs#L90-L107

// https://github.com/cosmos/cosmjs/blob/720d3b211e824f42a6a41a46e7a1d5c6a085ca00/packages/crypto/src/sha.ts#L29 sha256 expects u8 array

let binary_string = "eyJyZWdpc3RlciI6eyJuYW1lIjoidGVzdF9mcm9tX3RydXN0Ym9vc3Rfc2VwdCJ9fQ==";
binary_string = "eyJyZWdpc3Rlcl90YiI6eyJuYW1lIjoidGVzdF9mcm9tX3RydXN0Ym9vc3Rfc2VwdCJ9fQ==";

const len = binary_string.length;
let bytes = new Uint8Array(len);
for (var i = 0; i < len; i++) {
   bytes[i] = binary_string.charCodeAt(i);
}
let messageHash = sha256(bytes);

console.log("Message Hash");
console.log(messageHash);

const signature = await Secp256k1.createSignature(messageHash, keyPair.privkey);

// https://github.com/cosmos/cosmjs/blob/720d3b211e824f42a6a41a46e7a1d5c6a085ca00/packages/crypto/src/secp256k1signature.ts#L16-L31
console.log("SIGNATURE FIXED_LENGTH: ");
console.log(signature.toFixedLength());

// Length is 65 because it includes recovery parameter, need to remove it since it expects 64 bytes
const check = Secp256k1Signature.fromFixedLength(signature.toFixedLength().subarray(0,64));
console.log("PASS CHECK!!!");

console.log("SIGNATURE DER: " + signature.toDer());

