# Ethy

Is a protocol for generating collaborative message proofs using network validators (based on beefy-gadget).
Validators receive proof requests via a consensus log in blocks (i.e. from the runtime) and sign a witness.
The witness is broadcast on a dedicated p2p channel, once a configurable threshold of validators have signed and
broadcast their witnesses, each participant may construct a local proof.

The proof is simply an ordered list of signatures from all validators over a given message.
This could be advanced to use threshold signing scheme in the future.
The proof is portable and useful for submitting to an accompanying Ethereum contract.

It differs from BEEFY in 2 main ways
1) Ethy witnesses arbitrary messages vs. finalized blocks
2) The Ethy proof structure is less compact and less complex (BEEFY uses probabilistic sampling of a Merkle root signature)
