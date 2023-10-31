# Asset-ext pallet supported RPC


## Supported RPCs

- `assetBalance` - single place to get balance for ROOT (Staked token) and other assets

## RPCs

---

### `assetBalance`

Returns the free balance of asset and user.

#### Parameters

- `assetId`: The asset id.
- `account`: The account in query.

#### Returns

- `Balance`: The amount of free input token with the account.

#### Payload

```json
{
  "jsonrpc": "2.0",
  "method": "assets-ext_assetBalance",
  "params": [2, "0x25451A4de12dcCc2D166922fA938E900fCc4ED24"],
  "id": 1
}
```

**_Curl example:_**

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method":"assets-ext_assetBalance", "params":[2, "0x25451A4de12dcCc2D166922fA938E900fCc4ED24"]}' \
  http://localhost:9933
```

**_Response (successful)_**

```json
{
  "jsonrpc": "2.0",
  "result": 1000000000000,
  "id": 1
}
```

For ROOT

```sh
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method":"assets-ext_assetBalance", "params":[1, "0xE04CC55ebEE1cBCE552f250e85c57B70B2E2625b"]}' \
  http://localhost:9933
```

**_Response (successful)_**

```json
{
  "jsonrpc": "2.0",
  "result": 900000000000,
  "id": 1
}
```

---

