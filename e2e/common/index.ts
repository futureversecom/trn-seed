import { ApiPromise } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { AnyJson } from "@polkadot/types/types";
import web3 from "web3";

export * from "./node";

/** TYPEDEFS */

export const typedefs = {
  AccountId: "EthereumAccountId",
  AccountId20: "EthereumAccountId",
  AccountId32: "EthereumAccountId",
  Address: "AccountId",
  LookupSource: "AccountId",
  Lookup0: "AccountId",
  EthereumSignature: {
    r: "H256",
    s: "H256",
    v: "U8",
  },
  ExtrinsicSignature: "EthereumSignature",
  SessionKeys: "([u8; 32], [u8; 32])",
};

/** CONSTANTS */

export const NATIVE_TOKEN_ID = 1;
export const GAS_TOKEN_ID = 2;
export const ALITH_PRIVATE_KEY = "0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133";
export const BOB_PRIVATE_KEY = "0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf";
export const FEE_PROXY_ADDRESS = "0x00000000000000000000000000000000000004bb";
// The current index of the evm pallet. This might change between deployments, depending on the order/index in `construct_runtime`
export const EVM_PALLET_INDEX = "27";
// The current index of the pallet WithdrawFailed error
export const WITHDRAW_FAILED_ERROR_INDEX = "0x03000000";
export const DEAD_ADDRESS = "0x000000000000000000000000000000000000DEAD";

/** ABIs */

export const FEE_PROXY_ABI = [
  "function callWithFeePreferences(address asset, uint128 maxPayment, address target, bytes input)",
];

export const ERC20_ABI = [
  "event Transfer(address indexed from, address indexed to, uint256 value)",
  "event Approval(address indexed owner, address indexed spender, uint256 value)",
  "function approve(address spender, uint256 amount) public returns (bool)",
  "function allowance(address owner, address spender) public view returns (uint256)",
  "function balanceOf(address who) public view returns (uint256)",
  "function name() public view returns (string memory)",
  "function symbol() public view returns (string memory)",
  "function decimals() public view returns (uint8)",
  "function transfer(address who, uint256 amount)",
];

/** Functions */

export const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export const assetIdToERC20ContractAddress = (assetId: string | number): string => {
  const asset_id_hex = (+assetId).toString(16).padStart(8, "0");
  return web3.utils.toChecksumAddress(`0xCCCCCCCC${asset_id_hex}000000000000000000000000`);
};

/**
 * Fields of a Polkadotjs event to match on
 */
interface EventMatchers {
  /**
   * Method of a pallet being matched on
   */
  method?: string;
  /**
   * Section, or pallet to match on
   */
  section?: string;
}

/**
 * gets the next asset id - to be created by `assetsExt.createAsset`
 */
export const getNextAssetId = async (api: ApiPromise): Promise<number> => {
  const nextAssetId = (await api.query.assetsExt.nextAssetId()).toString();
  const nextAssetIdBin = (+nextAssetId).toString(2).padStart(22, "0");
  const parachainIdBin = (100).toString(2).padStart(10, "0");
  const nextAssetUuid = parseInt(nextAssetIdBin + parachainIdBin, 2);
  return nextAssetUuid;
};

export const getCollectionPrecompileAddress = (collectionId: number) => {
  const collectionIdBin = (+collectionId).toString(2).padStart(22, "0");
  const parachainIdBin = (100).toString(2).padStart(10, "0");
  const collectionUuid = parseInt(collectionIdBin + parachainIdBin, 2);
  const collectionIdHex = (+collectionUuid).toString(16).padStart(8, "0");
  return web3.utils.toChecksumAddress(`0xAAAAAAAA${collectionIdHex}000000000000000000000000`);
};

/**
 * createAssetUntil continously creates assets until asset with `assetId` exists
 * throws error if `assetId` is less than next asset id and does not already exist
 */
export const getOrCreateAssetUntil = async (
  api: ApiPromise,
  keyring: KeyringPair,
  assetId: number,
): Promise<AnyJson> => {
  let gotAssetId = null;
  while (gotAssetId === null) {
    // check if assetId is already available
    gotAssetId = (await api.query.assets.asset(assetId)).toJSON();
    if (gotAssetId) {
      return gotAssetId;
    }

    // check next asset id is greater than provided id
    const nextAssetId = await getNextAssetId(api);
    if (nextAssetId > assetId) {
      throw new Error(`next asset id ${nextAssetId} is less than ${assetId}`);
    }

    // create new asset
    await new Promise<void>((resolve) => {
      console.log(`creating asset ${nextAssetId}...`);
      api.tx.assetsExt
        .createAsset("test", "TEST", 18, 1, keyring.address)
        .signAndSend(keyring, { nonce: -1 }, ({ status }) => {
          if (status.isInBlock) {
            console.log(`created asset ${nextAssetId}`);
            resolve();
          }
        });
    });
  }
};

/**
 * Match on some amount of previous polkadotjs events up to `previousBlocks` behind, executing `fn` on any event results
 * WARNING: use for tests only, as this makes use of the `events()` storage item
 */
export const executeForPreviousEvent = async (
  api: ApiPromise,
  /**
   * Matchers with fields that will match one or more event being queried for
   */
  matchers: EventMatchers,
  /**
   * Maximum number of blocks to check in history
   */
  previousBlocks: number,
  /**
   * Callback to execute on a found event, given the event data.
   */
  fn: (retrievedEventData: any) => any,
) => {
  const currentHash = await api.rpc.chain.getBlockHash();
  let parentHash: any = currentHash;

  let currentInHistory = 0;
  while (currentInHistory !== previousBlocks) {
    let events;
    if (parentHash === null) {
      events = await api.query.system.events();
      // Set initial parentHash
      parentHash = await api.query.system.parentHash();
    } else {
      events = await api.query.system.events.at(parentHash);
      // new parentHash for next iteration
      parentHash = await api.query.system.parentHash.at(parentHash);
    }

    (events as any).forEach(async ({ event }: { event: any }) => {
      // For any events, only match on combination of matchers, or single matcher
      if ("method" in matchers && "section" in matchers) {
        if (event.method === matchers.method && event.section === matchers.section) {
          await fn(event);
        }
      } else if ("method" in matchers && matchers.method === event.method) {
        await fn(event);
      } else if ("section" in matchers && matchers.section === event.section) {
        await fn(event);
      }
    });
    currentInHistory++;
  }
};
