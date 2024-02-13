# Doughnut Pallet
This pallet handles Doughnut transactions. An account(doughnut holder) can send a doughnut transaction with a valid
doughnut issued by another account(doughnut issuer) and the inner call will be dispatched on behalf of the doughnut issuer account.

The pallet only contains a single extrinsic, `transact` call. a self-contained call that validates the transaction
and dispatches the inner call to the chain.
