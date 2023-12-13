# XRPL Transaction pallet

This pallet allows for the submission of signed XRPL transactions of the [`SignIn`](https://docs.xumm.dev/concepts/special-transaction-types) type by the XUMM mobile wallet to be submitted to the chain.

The signed transaction is a hex encoded message that contains an inner call (extrinsic) to be dispatched by the chain.

The transaction must contain specific [memo data](https://xrpl.org/transaction-common-fields.html#memos-field) of the `extrinsic` `memo type` - which encodes a `nonce`, `max_block_number` and a `scale_encoded_extrinsic` to be dispatched by the mapped ethereum address (derived from the provided pub key in the signed transaction).
The scale_encoded_extrinsic is a SCALE encoded `Vec<u8>` of the extrinsic to be dispatched by the account.

The pallet only contains a single extrinsic, `submit_encoded_xumm_transaction` call; a self contained call that validates the signed transaction and dispatches the encoded extrinsic to the chain.

A signature must be provided along with the encoded message (signed transaction) in the `submit_encoded_xumm_transaction` call - which validates that the signed transaction was signed by the public key provided in the signed transaction.
