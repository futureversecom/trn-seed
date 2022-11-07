# DEX pallet supported RPCs

Pallet Dex is based on UniswapV2; a subset of the [router functions](https://docs.uniswap.org/protocol/V2/reference/smart-contracts/library#pairfor) are supported as RPCs in the pallet.

## Supported RPCs

- [`quote`](https://docs.uniswap.org/protocol/V2/reference/smart-contracts/library#quote)
- [`getAmountsOut`](https://docs.uniswap.org/protocol/V2/reference/smart-contracts/library#getamountsout)
- [`getAmountsIn`](https://docs.uniswap.org/protocol/V2/reference/smart-contracts/library#getamountsin)

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
  "params": [
    "0x0000000000000000000000000000000000000000000000000000000000000001",
    1124,
    2145
  ],
  "id": 1
}
```

***Curl example:***

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method":"dex_quote", "params":[ "0x0000000000000000000000000000000000000000000000000000000000000001", 2, 10 ]}' \
  http://localhost:9933
```

***Response (successful)***

```json
{
  "jsonrpc": "2.0",
  "result": { "Ok": "0x5" },
  "id": 1
}
```

---

### `getAmountsOut`

Returns the amount of output tokens that you would receive if you sent an amount of input tokens to the DEX.

#### Parameters

- `amountIn`: The amount of input token to be sent to the DEX.
- `path`: The path of tokens to be traded.

#### Returns

- `amounts`: The amount of output token that you would receive if you sent an amount of input token to the DEX.

#### Example

***Payload:***

```json
{
  "jsonrpc": "2.0",
  "method": "dex_getAmountsOut",
  "params": [
    1000000000000,
    [
      2,
      1124
    ]
  ],
  "id": 1
}
```

***Curl:***

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "dex_getAmountsOut", "params": [1000000000000, [2, 1124]]}' \
  http://localhost:9933
```

***Response (error)***

```json
{
  "jsonrpc": "2.0",
  "result": {
    "Err": {
      "Module": {
        "index": 16,
        "error": [0,0,0,0],
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

#### Parameters

- `amountOut`: The amount of output token to be received from the DEX.
- `path`: The path of tokens to be traded.

#### Returns

- `amounts`: The amount of input token that you would need to send to the DEX in order to receive an amount of output token.

#### Example

***Payload:***

```json
{
  "jsonrpc": "2.0",
  "method": "dex_getAmountsIn",
  "params": [
    1000000000000,
    [
      2,
      1124
    ]
  ],
  "id": 1
}
```

***Curl:***

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "dex_getAmountsIn", "params": [1000000000000, [2, 1124]]}' \
  http://localhost:9933
```

***Response (error)***

```json
{
  "jsonrpc": "2.0",
  "result": {
    "Err": {
      "Module": {
        "index": 16,
        "error": [0,0,0,0],
        "message": "MustBeEnabled"
      }
    }
  },
  "id": 1
}
```
