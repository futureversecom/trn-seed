
# Eth Bridge

The eth-bridge pallet defines a notarization protocol for CENNZnet validators to agree on values from the bridged Ethereum chain,
and additionally provide proofs of events having occurred on CENNZnet.
The proofs are a collection of signatures which can be verified by the bridge contract on Ethereum.

There are types of Ethereum values the bridge can verify:
1) verify a transaction hash exists that called a specific contract producing a specific event log
2) verify the `returndata` of calling a contract at some time _t_