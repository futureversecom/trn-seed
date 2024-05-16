# Fee Proxy Pallet
## Description
This pallet is contains all features and implementations related to Multicurrency on The Root
Network. These include multicurrency through extrinsic and EVM based entry points. 
In both situations, we allow the user to submit a transaction alongside their preferred
payment asset, which then gets exchanged for the Fee Asset (XRP) internally before the call is executed

Note: In both cases, we rely on liquidity to be provided between the payment asset and XRP within our DEX
If there is no liquidity, the transaction will fail.

## Extrinsic based Multicurrency
This pallet has one extrinsic called `call_with_fee_preferences`. This extrinsic takes in 3 parameters:
 - payment_asset: The payment asset to be used for paying fees
 - max_payment: The specified max amount of the payment asset that will be accepted for exchange. 
   - If the exchange rate exceeds this value, the transaction will fail
 - call: The inner call to be executed once fees have been swapped

This method works by intercepting the implementation of `OnChargeTransaction` in `transaction-payment`. 
This impl can be found in `pallet-fee-proxy/src/impls` and re routes to the `transaction-payment` implementation
after the exchange has occurred inside `withdraw_fee`

Note: An extra step is taken here to check whether the inner_call is `pallet_evm.call` this is due to the fact
that the gas fees associated with an evm call are calculated outside of the weight of the extrinsic
itself. So in this case, the gas fees are calculated through `get_fee_preferences_data` (based on the
gas_limit and max_fee_per_gas).

## EVM based Multicurrency - DEPRECATED; documentation requires update

Multicurrency is also enabled for calls directly through our EVM. This is possible by wrapping an abi encoded input
and setting the target to `FEE_PROXY_ADDRESS`. This is made possible by intercepting the implementation of `Runner` 
defined in pallet_evm. By creating our own implementation with `FeePreferencesRunner` we are able to check the target 
and decode the input to extract the fee preference parameters.

To achieve this, the input and target must be changed to the following:
#### target:
The target must be set to the `FEE_PROXY_ADDRESS` which is equal to `0x00000000000000000000000000000000000004bb`
#### Input:
The input is the abi encoded function call `callWithFeePreferences(address,uint128,address,bytes)`
 - Function Selector: FEE_FUNCTION_SELECTOR 
   - As bytes: `0x255a3432`
 - Payment_asset `address`: The

| Parameter         |  Type   | Description                                                | example                                                                    |
|-------------------|:-------:|------------------------------------------------------------|----------------------------------------------------------------------------|
| Function Selector |  bytes  | FEE_FUNCTION_SELECTOR                                      | 0x255a3432                                                                 |
| Payment Asset     | address | The precompile address of the payment asset                | 0xCCCCCCCC00001864000000000000000000000000                                 |
| Max Payment       | Uint128 | The max amount to exchange when swapping for the fee asset | 1,000,000                                                                  |
| New target        | address | The target address that the inner call will be routed to   | 0xAAAAAAAA00001864000000000000000000000000                                 |
| New Input         |  Bytes  | The abi encoded input for the inner call                   | 0xe475949300000000000000000000000025451a4de12dccc2d166922fa938e900fcc4ed24 |

### Example:
Say we want to call transferFrom on some collection to transfer an NFT from Bob to Alice, traditionally we would
encode this data as follows:

```solidity
abi.encodeWithSignature(
    "transferFrom(address,address,uint256)",    // Function selector
    0x25451A4de12dcCc2D166922fA938E900fCc4ED24, // Bob
    0xE04CC55ebEE1cBCE552f250e85c57B70B2E2625b, // Alice
    9                                           // TokenId
)
```
This then produces the below output
```
0x23b872dd00000000000000000000000025451a4de12dccc2d166922fa938e900fcc4ed24000000000000000000000000e04cc55ebee1cbce552f250e85c57b70b2e2625b0000000000000000000000000000000000000000000000000000000000000009
```

We now want to wrap this with the fee preferences call by redirecting the target and specifying parameters
```solidity
abi.encodeWithSignature(
    "callWithFeePreferences(address,uint128,address,bytes)",
    0xCCCCCCCC00001864000000000000000000000000, // Precompile address for AssetId 4452
    1000000,                                    // Max Payment for internal exchange
    0xAAAAAAAA00000464000000000000000000000000, // Precompile address for CollectionId 1124, 
    0x23b872dd00000000000000000000000025451a4de12dccc2d166922fa938e900fcc4ed24000000000000000000000000e04cc55ebee1cbce552f250e85c57b70b2e2625b0000000000000000000000000000000000000000000000000000000000000009
    // ^ Input encoded from transferFrom
)
```

Note that the target is the precompile address for CollectionId 1124, this is the contract address that we want
to perform transferFrom

Now that that has been encoded, this call can be called as per usual and it will first be routed through
the `FeePreferencesRunner` then to the collection address specified


