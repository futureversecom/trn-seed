# DEX pallet supported RPCs

Pallet Dex is based on UniswapV2; a subset of the [router functions](https://docs.uniswap.org/protocol/V2/reference/smart-contracts/library#pairfor) are supported as RPCs in the pallet.

## Supported RPCs

- [`quote`](https://docs.uniswap.org/protocol/V2/reference/smart-contracts/library#quote)
- [`getAmountsOut`](https://docs.uniswap.org/protocol/V2/reference/smart-contracts/library#getamountsout)
- [`getAmountsIn`](https://docs.uniswap.org/protocol/V2/reference/smart-contracts/library#getamountsin)
- `getLPTokenID`
- `getLiquidity`
- `getTradingPairStatus`

## RPCs

---

### `quote`

Returns the amount of output token that can be obtained by swapping an amount of input token.

#### Parameters

- `amountIn`: The amount of input token to be swapped.
- `reserveIn`: The amount of input token in the pool.
- `reserveOut`: The amount of output token in the pool.

#### Returns

- `amountOut`: The amount of output token that can be obtained by swapping `amountIn` of input token.

#### Payload

```json
{
  "jsonrpc": "2.0",
  "method": "dex_quote",
  "params": [1, 5, 10],
  "id": 1
}
```

**_Curl example:_**

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method":"dex_quote", "params":[ 1, 5, 10 ]}' \
  http://127.0.0.1:9944
```

**_Response (successful)_**

```json
{
  "jsonrpc": "2.0",
  "result": { "Ok": 2 },
  "id": 1
}
```

---

### `getAmountsOut`

Returns the amount of output tokens that you would receive if you sent an amount of input tokens to the DEX.

**\*Note**: This RPC requires liquidity for the given pair in the `path` param to be present in the DEX.\*

#### Parameters

- `amountIn`: The amount of input token to be sent to the DEX.
- `path`: The path of tokens to be traded.

#### Returns

- `amounts`: The amount of output token that you would receive if you sent an amount of input token to the DEX.

#### Example

**_Payload:_**

```json
{
  "jsonrpc": "2.0",
  "method": "dex_getAmountsOut",
  "params": [1000000000000, [2, 1124]],
  "id": 1
}
```

**_Curl:_**

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "dex_getAmountsOut", "params": [1000000000000, [2, 1124]]}' \
  http://127.0.0.1:9944
```

**_Response (error)_**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "Err": {
      "Module": {
        "index": 16,
        "error": [0, 0, 0, 0],
        "message": "MustBeEnabled"
      }
    }
  },
  "id": 1
}
```

---

### `getAmountsIn`

Returns the amount of input token that you would need to send to the DEX in order to receive an amount of output token.

**\*Note**: This RPC requires liquidity for the given pair in the `path` param to be present in the DEX.\*

#### Parameters

- `amountOut`: The amount of output token to be received from the DEX.
- `path`: The path of tokens to be traded.

#### Returns

- `amounts`: The amount of input token that you would need to send to the DEX in order to receive an amount of output token.

#### Example

**_Payload:_**

```json
{
  "jsonrpc": "2.0",
  "method": "dex_getAmountsIn",
  "params": [1000000000000, [2, 1124]],
  "id": 1
}
```

**_Curl:_**

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "dex_getAmountsIn", "params": [1000000000000, [2, 1124]]}' \
  http://127.0.0.1:9944
```

**_Response (error)_**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "Err": {
      "Module": {
        "index": 16,
        "error": [0, 0, 0, 0],
        "message": "MustBeEnabled"
      }
    }
  },
  "id": 1
}
```

---

### `getLPTokenID`

Returns the LP token ID from the given trading pair.

#### Parameters

- `assetIdA`: The first asset ID of the trading pair.
- `assetIdB`: The second asset ID of the trading pair.

#### Returns

- `lpTokenId`: The LP token ID of the trading pair (assetIdA, assetIdB).

#### Example

**_Payload:_**

```json
{
  "jsonrpc": "2.0",
  "method": "dex_getLPTokenID",
  "params": [2, 1124],
  "id": 1
}
```

**_Curl:_**

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "dex_getLPTokenID", "params": [2, 1124]}' \
  http://127.0.0.1:9944
```

**_Response (successful)_**

```json
{
  "jsonrpc": "2.0",
  "result": { "Ok": 2148 },
  "id": 1
}
```

---

### `getLiquidity`

Returns the liquidity balances of the given trading pair.

#### Parameters

- `assetIdA`: The first asset ID of the trading pair.
- `assetIdB`: The second asset ID of the trading pair.

#### Returns

- `balances`: The corresponding balances of each asset in the given trading pair (assetIdA, assetIdB).

#### Example

**_Payload:_**

```json
{
  "jsonrpc": "2.0",
  "method": "dex_getLiquidity",
  "params": [2, 1124],
  "id": 1
}
```

**_Curl:_**

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "dex_getLiquidity", "params": [2, 1124]}' \
  http://127.0.0.1:9944
```

**_Response (successful)_**

```json
{
  "jsonrpc": "2.0",
  "result": [10000000, 250],
  "id": 1
}
```

---

### `getTradingPairStatus`

Returns the status of the given trading pair.

#### Parameters

- `assetIdA`: The first asset ID of the trading pair.
- `assetIdB`: The second asset ID of the trading pair.

#### Returns

- `status`: The current status of the given trading pair (assetIdA, assetIdB).

#### Example

**_Payload:_**

```json
{
  "jsonrpc": "2.0",
  "method": "dex_getTradingPairStatus",
  "params": [2, 1124],
  "id": 1
}
```

**_Curl:_**

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "dex_getTradingPairStatus", "params": [2, 1124]}' \
  http://127.0.0.1:9944
```

**_Response (successful)_**

```json
{
  "jsonrpc": "2.0",
  "result": "Enabled",
  "id": 1
}
```
