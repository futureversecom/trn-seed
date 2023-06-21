import { ApiPromise } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { AnyJson } from "@polkadot/types/types";
import { writeFileSync } from "fs";
import { CliPrettify } from "markdown-table-prettify";
import { join } from "path";
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
  CollectionInformation: {
    owner: "AccountId",
    name: "Vec<u8>",
    metadataScheme: "MetadataScheme",
    royaltiesSchedule: "Option<RoyaltiesSchedule>",
    maxIssuance: "Option<TokenCount>",
    originChain: "OriginChain",
    nextSerialNumber: "SerialNumber",
    collectionIssuance: "TokenCount",
    crossChainCompatibility: "CrossChainCompatibility",
    ownedTokens: "Vec<TokenOwnership>",
  },
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

// Precompile address for nft precompile is 1721
export const NFT_PRECOMPILE_ADDRESS = "0x00000000000000000000000000000000000006b9";
// Precompile address for sft precompile is 1731
export const SFT_PRECOMPILE_ADDRESS = "0x00000000000000000000000000000000000006c3";
// Precompile address for futurepass registrar precompile is 65535
export const FUTUREPASS_REGISTRAR_PRECOMPILE_ADDRESS = "0x000000000000000000000000000000000000FFFF";

// Precompile address for peg precompile is 1939
export const PEG_PRECOMPILE_ADDRESS = "0x0000000000000000000000000000000000000793";

// Futurepass delegate reserve amount
export const FP_DELEGATE_RESERVE = 126 * 1; // ProxyDepositFactor * 1(num of delegates)

// Futurepass creation reserve amount
export const FP_CREATION_RESERVE = 148 + FP_DELEGATE_RESERVE; // ProxyDepositBase + ProxyDepositFactor * 1(num of delegates)

export type GasCosts = {
  Contract: number;
  Precompile: number;
  Extrinsic: number;
};

/** ABIs */

const OWNABLE_ABI = [
  "event OwnershipTransferred(address indexed previousOwner, address indexed newOwner)",

  "function owner() public view returns (address)",
  "function renounceOwnership()",
  "function transferOwnership(address owner)",
];

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
  "function totalSupply() external view returns (uint256)",
  "function transfer(address who, uint256 amount)",
  "function transferFrom(address from, address to, uint256 amount)",
];

export const NFT_PRECOMPILE_ABI = [
  "event InitializeCollection(address indexed collectionOwner, address precompileAddress)",
  "function initializeCollection(address owner, bytes name, uint32 maxIssuance, bytes metadataPath, address[] royaltyAddresses, uint32[] royaltyEntitlements) returns (address, uint32)",
];

export const SFT_PRECOMPILE_ABI = [
  "event InitializeSftCollection(address indexed collectionOwner, address indexed precompileAddress)",
  "function initializeCollection(address owner, bytes name, bytes metadataPath, address[] royaltyAddresses, uint32[] royaltyEntitlements) returns (address, uint32)",
];

export const PEG_PRECOMPILE_ABI = [
  "event Erc20Withdrawal(uint64 indexed eventProofId, address indexed beneficiary, address indexed tokenAddress, uint128 balance)",
  "event Erc721Withdrawal(uint64 indexed eventProofId, address indexed beneficiary, address indexed tokenAddress, uint32[] serialNumbers)",
  "function erc20Withdraw(address beneficiary, address asset, uint128 balance) returns (uint64)",
  "function erc721Withdraw(address beneficiary, address[] tokenAddresses, uint32[][] serialNumbers) returns (uint64)",
];

export const ERC721_PRECOMPILE_ABI = [
  // ERC721
  "event Transfer(address indexed from, address indexed to, uint256 indexed tokenId)",
  "event Approval(address indexed owner, address indexed approved, uint256 indexed tokenId)",
  "event ApprovalForAll(address indexed owner, address indexed operator, bool approved)",

  "function balanceOf(address who) public view returns (uint256)",
  "function ownerOf(uint256 tokenId) public view returns (address)",
  "function safeTransferFrom(address from, address to, uint256 tokenId)",
  "function transferFrom(address from, address to, uint256 tokenId)",
  "function approve(address to, uint256 tokenId)",
  "function getApproved(uint256 tokenId) public view returns (address)",
  "function setApprovalForAll(address operator, bool _approved)",
  "function isApprovedForAll(address owner, address operator) public view returns (bool)",

  // ERC721 Metadata
  "function name() public view returns (string memory)",
  "function symbol() public view returns (string memory)",
  "function tokenURI(uint256 tokenId) public view returns (string memory)",

  // Root specific precompiles
  "event MaxSupplyUpdated(uint32 maxSupply)",
  "event BaseURIUpdated(string baseURI)",

  "function totalSupply() external view returns (uint256)",
  "function mint(address owner, uint32 quantity)",
  "function setMaxSupply(uint32 maxSupply)",
  "function setBaseURI(bytes baseURI)",
  "function ownedTokens(address who, uint16 limit, uint32 cursor) public view returns (uint32, uint32, uint32[] memory)",

  // Ownable
  ...OWNABLE_ABI,
];

export const ERC1155_PRECOMPILE_ABI = [
  // ERC1155
  "event TransferSingle(address indexed operator, address indexed from, address indexed to, uint256 id, uint256 value)",
  "event TransferBatch(address indexed operator, address indexed from, address indexed to, uint256[] ids, uint256[] balances)",
  "event ApprovalForAll(address indexed account, address indexed operator, bool approved)",

  "function balanceOf(address owner, uint256 id) external view returns (uint256)",
  "function balanceOfBatch(address[] owners, uint256[] ids) external view returns (uint256[] memory)",
  "function setApprovalForAll(address operator, bool approved) external",
  "function isApprovedForAll(address account, address operator) external view returns (bool)",
  "function safeTransferFrom(address from, address to, uint256 id, uint256 amount, bytes calldata data) external",
  "function safeBatchTransferFrom(address from, address to, uint256[] calldata ids, uint256[] calldata amounts, bytes calldata data) external",

  // Burnable
  "function burn(address account, uint256 id, uint256 value) external",
  "function burnBatch(address account, uint256[] ids, uint256[] values) external",

  // Supply
  "function totalSupply(uint256 id) external view returns (uint256)",
  "function exists(uint256 id) external view returns (bool)",

  // Metadata
  "function uri(uint256 id) external view returns (string memory)",

  // TRN
  "event TokenCreated(uint32 indexed serialNumber)",
  "event MaxSupplyUpdated(uint128 indexed maxSupply)",
  "event BaseURIUpdated(string baseURI)",

  "function createToken(bytes name, uint128 initialIssuance, uint128 maxIssuance, address tokenOwner) external returns (uint32)",
  "function mint(address owner, uint256 id, uint256 amount) external",
  "function mintBatch(address owner, uint256[] ids, uint256[] amounts) external",
  "function setMaxSupply(uint256 id, uint32 maxSupply) external",
  "function setBaseURI(bytes baseURI) external",

  // Ownable
  ...OWNABLE_ABI,
];

export const FUTUREPASS_REGISTRAR_PRECOMPILE_ABI = [
  "event FuturepassCreated(address indexed futurepass, address owner)",

  "function futurepassOf(address owner) external view returns (address)",
  "function create(address owner) external returns (address)",
];

export const FUTUREPASS_PRECOMPILE_ABI = [
  "event FuturepassDelegateRegistered(address indexed futurepass, address indexed delegate, uint8 proxyType)",
  "event FuturepassDelegateUnregistered(address indexed futurepass, address delegate)",
  "event Executed(uint8 indexed callType, address indexed target, uint256 indexed value, bytes4 data)",
  "event ContractCreated(uint8 indexed callType, address indexed contractAddress, uint256 indexed value, bytes32 salt)",

  "function delegateType(address delegate) external view returns (uint8)",
  "function registerDelegate(address delegate, uint8 proxyType) external",
  "function unregisterDelegate(address delegate) external",
  "function proxyCall(uint8 callType, address callTo, uint256 value, bytes memory callData) external payable",

  // Ownable
  ...OWNABLE_ABI,
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

/**
 *
 * @param collectionId Converts collection id to precompile address (without parachain id)
 * @returns
 */
export const getCollectionPrecompileAddress = (collectionId: number) => {
  const collectionIdBin = (+collectionId).toString(2).padStart(22, "0");
  const parachainIdBin = (100).toString(2).padStart(10, "0");
  const collectionUuid = parseInt(collectionIdBin + parachainIdBin, 2);
  const collectionIdHex = (+collectionUuid).toString(16).padStart(8, "0");
  return web3.utils.toChecksumAddress(`0xAAAAAAAA${collectionIdHex}000000000000000000000000`);
};

/**
 *
 * @param collectionId Converts collection id to precompile address (without parachain id)
 * @returns
 */
export const getSftCollectionPrecompileAddress = (collectionId: number) => {
  const collectionIdBin = (+collectionId).toString(2).padStart(22, "0");
  const parachainIdBin = (100).toString(2).padStart(10, "0");
  const collectionUuid = parseInt(collectionIdBin + parachainIdBin, 2);
  const collectionIdHex = (+collectionUuid).toString(16).padStart(8, "0");
  return web3.utils.toChecksumAddress(`0xBBBBBBBB${collectionIdHex}000000000000000000000000`);
};

/**
 * Saves gas cost to a markdown file
 * @returns
 * @param costs Dictionary of gas costs for different function calls
 * @param filePath The file path to save the output
 * @param header The header for the generated output, i.e. "ERC1155 Precompiles"
 */
export const saveGasCosts = (costs: { [key: string]: GasCosts }, filePath: string, header: string) => {
  // Set string headers
  let data: string = `## Generated gas prices for ${header}\n\n`;
  data += "| Function Call | Contract gas | Precompile gas | Extrinsic gas |\n";
  data += "| :--- | :---: | :---: | :---: |\n";

  // Iterate through functions and add gas prices
  for (const key in costs) {
    const value = costs[key];
    data += `| ${key} | ${value.Contract} | ${value.Precompile} | ${value.Extrinsic} |\n`;
  }

  // Prettify data
  data = CliPrettify.prettify(data);

  // Save data to specified file path
  writeFileSync(join("./test", filePath), data, {
    flag: "w",
  });
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
