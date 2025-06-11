import { JsonRpcProvider } from "@ethersproject/providers";
import { ApiPromise, Keyring } from "@polkadot/api";
import { SignerOptions, SubmittableExtrinsic } from "@polkadot/api/types";
import { KeyringPair } from "@polkadot/keyring/types";
import { AnyJson } from "@polkadot/types/types";
import { BigNumber } from "ethers";
import { writeFileSync } from "fs";
import { CliPrettify } from "markdown-table-prettify";
import { join } from "path";
import web3 from "web3";

export * from "./node";

/** TYPEDEFS */
export const rpcs = {
  assetsExt: {
    freeBalance: {
      description: "Given asset id and address, returns free balance",
      params: [
        {
          name: "asset",
          type: "AssetId",
        },
        {
          name: "who",
          type: "AccountId",
        },
      ],
      type: "Text",
    },
  },
  dex: {
    quote: {
      description: "Given some amount of an asset and pair reserves, returns an equivalent amount of the other asset",
      params: [
        {
          name: "amountA",
          type: "u128",
        },
        {
          name: "reserveA",
          type: "u128",
        },
        {
          name: "reserveB",
          type: "u128",
        },
      ],
      type: "Json",
    },
    getAmountsOut: {
      description: "Given an array of AssetIds, return amounts out for an amount in",
      params: [
        {
          name: "amountIn",
          type: "Balance",
        },
        {
          name: "path",
          type: "Vec<AssetId>",
        },
      ],
      type: "Json",
    },
    getAmountsIn: {
      description: "Given an array of AssetIds, return amounts in for an amount out",
      params: [
        {
          name: "amountOut",
          type: "Balance",
        },
        {
          name: "path",
          type: "Vec<AssetId>",
        },
      ],
      type: "Json",
    },
    getLPTokenID: {
      description: "Given two AssetIds, return liquidity token created for the pair",
      params: [
        {
          name: "assetA",
          type: "AssetId",
        },
        {
          name: "assetB",
          type: "AssetId",
        },
      ],
      type: "Json",
    },
    getLiquidity: {
      description: "Given two AssetIds, return liquidity",
      params: [
        {
          name: "assetA",
          type: "AssetId",
        },
        {
          name: "assetB",
          type: "AssetId",
        },
      ],
      type: "Json",
    },
    getTradingPairStatus: {
      description: "Given two AssetIds, return whether trading pair is enabled or disabled",
      params: [
        {
          name: "assetA",
          type: "AssetId",
        },
        {
          name: "assetB",
          type: "AssetId",
        },
      ],
      type: "Text",
    },
  },
  ethy: {
    getEventProof: {
      description: "Get ETH event proof for event Id",
      params: [
        {
          name: "eventId",
          type: "EventProofId",
        },
      ],
      type: "Option<EthEventProofResponse>",
    },
    getXrplTxProof: {
      description: "Get XRPL event proof for event Id",
      params: [
        {
          name: "eventId",
          type: "EventProofId",
        },
      ],
      type: "Option<XrplEventProofResponse>",
    },
  },
  nft: {
    ownedTokens: {
      description: "Get all NFTs owned by an account",
      params: [
        {
          name: "collectionId",
          type: "CollectionUuid",
        },
        {
          name: "who",
          type: "AccountId",
        },
        { name: "cursor", type: "SerialNumber" },
        { name: "limit", type: "u16" },
      ],
      type: "Json",
    },
    tokenUri: {
      description: "Get the URI of a token",
      params: [
        {
          name: "tokenId",
          type: "TokenId",
        },
      ],
      type: "Json",
    },
    collectionDetails: {
      description: "Returns the collection info for a NFT collection",
      params: [
        {
          name: "collectionId",
          type: "u32",
        },
      ],
      type: "CollectionDetail",
    },
  },
  syloDataPermissions: {
    getPermissions: {
      description: "Get all permissions granted to a grantee by a data author for a given set of data ids",
      params: [
        {
          name: "dataAuthor",
          type: "AccountId",
        },
        {
          name: "grantee",
          type: "AccountId",
        },
        {
          name: "dataIds",
          type: "Vec<Text>",
        },
      ],
      type: "Json",
    },
  },
};
export const typedefs = {
  AccountId: "EthereumAccountId",
  AccountId20: "EthereumAccountId",
  AccountId32: "EthereumAccountId",
  Address: "AccountId",
  LookupSource: "AccountId",
  Lookup0: "AccountId",
  DataPermission: "Text",
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
  CollectionDetail: {
    owner: "AccountId",
    name: "Vec<u8>",
    metadataScheme: "Vec<u8>",
    royaltiesSchedule: "Option<Vec<(T::AccountId, Permill)>>",
    maxIssuance: "Option<u32>",
    originChain: "Text",
    nextSerialNumber: "u32",
    collectionIssuance: "u32",
    crossChainCompatibility: "CrossChainCompatibility",
  },
  CrossChainCompatibility: {
    xrpl: "bool",
  },
};

/** CONSTANTS */

export const NATIVE_TOKEN_ID = 1;
export const GAS_TOKEN_ID = 2;
export const ALITH_PRIVATE_KEY = "0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133";
export const ALICE_PRIVATE_KEY = "0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854";
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

// Precompile address for dex precompile
export const DEX_PRECOMPILE_ADDRESS = "0x000000000000000000000000000000000000DDDD";

// Precompile address for marketplace precompile
export const MARKETPLACE_PRECOMPILE_ADDRESS = "0x00000000000000000000000000000000000006CD";

// Futurepass delegate reserve amount
export const FP_DELEGATE_RESERVE = 126 * 1; // ProxyDepositFactor * 1(num of delegates)

// XRP Precompile contract address
export const XRP_PRECOMPILE_ADDRESS = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000");

// ROOT Precompile contract address
export const ROOT_PRECOMPILE_ADDRESS = web3.utils.toChecksumAddress("0xCCCCCCCC00000001000000000000000000000000");

// Futurepass creation reserve amount
export const FP_CREATION_RESERVE = 148 + FP_DELEGATE_RESERVE; // ProxyDepositBase + ProxyDepositFactor * 1(num of delegates)

export type TxCosts = {
  Contract: BigNumber;
  Precompile: BigNumber;
  Extrinsic: BigNumber;
};

/** ABIs */

const OWNABLE_ABI = [
  "event OwnershipTransferred(address indexed previousOwner, address indexed newOwner)",

  "function owner() public view returns (address)",
  "function renounceOwnership()",
  "function transferOwnership(address owner)",
];

const ERC165_ABI = ["function supportsInterface(bytes4 interfaceId) public view returns (bool)"];

export const FEE_PROXY_ABI_DEPRECATED = [
  "function callWithFeePreferences(address asset, uint128 maxPayment, address target, bytes input)",
];
export const FEE_PROXY_ABI = ["function callWithFeePreferences(address asset, address target, bytes input)"];

export const ERC20_ABI = [
  // ERC20
  "event Transfer(address indexed from, address indexed to, uint256 value)",
  "event Approval(address indexed owner, address indexed spender, uint256 value)",
  "function approve(address spender, uint256 amount) public returns (bool)",
  "function allowance(address owner, address spender) public view returns (uint256)",
  "function balanceOf(address who) public view returns (uint256)",
  "function totalSupply() external view returns (uint256)",
  "function transfer(address who, uint256 amount)",
  "function transferFrom(address from, address to, uint256 amount)",

  // ERC20 Metadata
  "function name() public view returns (string memory)",
  "function symbol() public view returns (string memory)",
  "function decimals() public view returns (uint8)",

  // ERC165
  ...ERC165_ABI,
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

  // ERC721 Burnable
  "function burn(uint256 tokenId)",

  // Root specific precompiles
  "event MaxSupplyUpdated(uint32 maxSupply)",
  "event BaseURIUpdated(string baseURI)",
  "event PublicMintToggled(bool enabled)",
  "event MintFeeUpdated(address indexed paymentAsset, uint128 indexed mintFee)",

  "function totalSupply() external view returns (uint256)",
  "function mint(address owner, uint32 quantity)",
  "function setMaxSupply(uint32 maxSupply)",
  "function setBaseURI(bytes baseURI)",
  "function ownedTokens(address who, uint16 limit, uint32 cursor) public view returns (uint32, uint32, uint32[] memory)",
  "function togglePublicMint(bool enabled)",
  "function setMintFee(address paymentAsset, uint128 mintFee)",

  // Ownable
  ...OWNABLE_ABI,

  // ERC165
  ...ERC165_ABI,

  // ERC5484
  "event PendingIssuanceCreated(address indexed to, uint256 issuanceId, uint256 quantity, uint8 burnAuth)",
  "event Issued(address indexed from, address indexed to, uint256 indexed tokenId, uint8 burnAuth)",

  "function issueSoulbound(address,uint32,uint8)",
  "function acceptSoulboundIssuance(uint32)",
  "function pendingIssuances(address) external view returns (uint256[] memory, (uint256 quantity, uint8)[] memory)",
  "function burnAuth(uint256) external view returns (uint8)",
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

  // ERC1155Burnable
  "function burn(address account, uint256 id, uint256 value) external",
  "function burnBatch(address account, uint256[] ids, uint256[] values) external",

  // ERC1155Supply
  "function totalSupply(uint256 id) external view returns (uint256)",
  "function exists(uint256 id) external view returns (bool)",

  // ERC1155MetadataURI
  "function uri(uint256 id) external view returns (string memory)",

  // TRN
  "event TokenCreated(uint32 indexed serialNumber)",
  "event MaxSupplyUpdated(uint128 indexed maxSupply)",
  "event BaseURIUpdated(string baseURI)",
  "event PublicMintToggled(uint32 indexed id, bool enabled)",
  "event MintFeeUpdated(uint32 indexed id, address indexed paymentAsset, uint128 indexed mintFee)",

  "function createToken(bytes name, uint128 initialIssuance, uint128 maxIssuance, address tokenOwner) external returns (uint32)",
  "function mint(address owner, uint256 id, uint256 amount) external",
  "function mintBatch(address owner, uint256[] ids, uint256[] amounts) external",
  "function setMaxSupply(uint256 id, uint32 maxSupply) external",
  "function setBaseURI(bytes baseURI) external",
  "function togglePublicMint(uint256 id, bool enabled)",
  "function setMintFee(uint256 id, address paymentAsset, uint128 mintFee)",

  // Ownable
  ...OWNABLE_ABI,

  // ERC165
  ...ERC165_ABI,

  // ERC5484
  "event PendingIssuanceCreated(address indexed to, uint256 issuanceId, uint256[] tokenIds, uint256[] balances)",
  "event Issued(address indexed from, address indexed to, uint256 indexed tokenId, uint8 burnAuth)",

  "function setBurnAuth(uint256,uint8)",
  "function issueSoulbound(address,uint256[],uint256[])",
  "function acceptSoulboundIssuance(uint32)",
  "function pendingIssuances(address) external view returns (uint256[] memory,(uint256[] memory,uint256[] memory,uint8[] memory)[] memory)",
  "function burnAuth(uint256) external view returns (uint8)",
  "function burnAsCollectionOwner(address,uint256[],uint256[])",
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
  "function registerDelegateWithSignature(address delegate, uint8 proxyType, uint32 deadline, bytes memory signature) external",
  "function unregisterDelegate(address delegate) external",
  "function proxyCall(uint8 callType, address callTo, uint256 value, bytes memory callData) external payable",

  // Ownable
  ...OWNABLE_ABI,
];

export const MARKETPLACE_ABI = [
  "event MarketplaceRegister(address indexed sender, uint256 indexed marketplaceId, address marketplaceAccount)",
  "event FixedPriceSaleListNFT(address indexed seller, uint256 indexed listingId, uint256 indexed fixedPrice, uint256[] serialNumbers, address collectionAddress, uint128 marketplaceId)",
  "event FixedPriceSaleListSFT(address indexed seller, uint256 indexed listingId, uint256 indexed fixedPrice, uint256[] serialNumbers, address collectionAddress, uint128 marketplaceId, uint256[] quantities)",
  "event FixedPriceSaleUpdate(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed newPrice, address sender, uint256[] serialNumbers, uint128 marketplaceId)",
  "event FixedPriceSaleComplete(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed fixedPrice, address sender, uint256[] serialNumbers, uint128 marketplaceId)",
  "event AuctionOpenNFT(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed reservePrice, address sender, uint256[] serialNumbers, uint128 marketplaceId)",
  "event AuctionOpenSFT(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed reservePrice, address sender, uint256[] serialNumbers, uint128 marketplaceId)",
  "event Bid(address indexed bidder, uint256 indexed listingId, uint256 indexed amount, uint128 marketplaceId)",
  "event FixedPriceSaleClose(uint256 indexed collectionId, uint256 indexed listingId, address sender, uint256[] serialNumbers, uint128 marketplaceId)",
  "event AuctionClose(uint256 indexed collectionId, uint256 indexed listingId, address sender, uint256[] serialNumbers, uint128 marketplaceId)",
  "event Offer(uint256 indexed offerId, address indexed sender, uint256 collectionId, uint256 seriesId, uint128 marketplaceId)",
  "event OfferCancel(uint256 indexed offerId, address indexed sender, uint256 collectionId, uint256 seriesId, uint128 marketplaceId)",
  "event OfferAccept(uint256 indexed offerId, uint256 indexed amount, address indexed sender, uint256 collectionId, uint256 seriesId, uint128 marketplaceId)",

  "function registerMarketplace(address marketplaceAccount, uint256 entitlement) external returns (uint marketplaceId)",
  "function sellNftWithMarketplaceId(address collectionAddress, uint32[] calldata serialNumberIds, address buyer, address paymentAsset, uint128 fixedPrice, uint128 duration, uint32 marketplaceId) external returns (uint listingId)",
  "function sellNft(address collectionAddress, uint32[] calldata serialNumberIds, address buyer, address paymentAsset, uint128 fixedPrice, uint128 duration, uint32 marketplaceId) external returns (uint listingId)",
  "function sellSft(address collectionAddress, uint32[] calldata serialNumberIds, uint128[] calldata quantities, address buyer, address paymentAsset, uint128 fixedPrice, uint128 duration, uint32 marketplaceId) external returns (uint listingId)",
  "function updateFixedPrice(uint128 listingId, uint128 newPrice) external",
  "function buy(uint128 listingId) external",
  "function auctionNftWithMarketplaceId(address collectionAddress, uint32[] calldata serialNumberIds, address paymentAsset, uint128 reservePrice, uint128 duration, uint32 marketplaceId) external",
  "function auctionNft(address collectionAddress, uint32[] calldata serialNumberIds, address paymentAsset, uint128 reservePrice, uint128 duration, uint32 marketplaceId) external",
  "function auctionSft(address collectionAddress, uint32[] calldata serialNumberIds, uint128[] calldata quantities, address paymentAsset, uint128 reservePrice, uint128 duration, uint32 marketplaceId) external",
  "function bid(uint128 listingId, uint128 amount) external",
  "function cancelSale(uint128 listingId) external",
  "function makeSimpleOfferWithMarketplaceId(address collectionAddress, uint32 serialNumber, uint128 amount, address assetId, uint32 marketplaceId) external returns (uint offerId)",
  "function makeSimpleOffer(address collectionAddress, uint32 serialNumber, uint128 amount, address assetId, uint32 marketplaceId) external returns (uint offerId)",
  "function cancelOffer(uint64 offerId) external",
  "function acceptOffer(uint64 offerId) external",

  // read
  "function getMarketplaceAccount(uint32 marketplaceId) external view returns(address marketplaceAccount)",
  "function getListingFromId(uint128 listingId) external view returns (bytes memory listingType, uint32 collectionId, uint32[] calldata serialNumbers, uint128 reservePrice, uint32 paymentAsset)",
  "function getOfferFromId(uint64 offerId) external view returns (uint32 collectionId, uint32 serialNumber, uint128 amount, address buyer)",
];

export const DEX_PRECOMPILE_ABI = [
  // IUniswapV2Pair
  "event Mint(address indexed sender, uint256 amount0, uint256 amount1)",
  "event Burn(address indexed sender, uint256 amount0, uint256 amount1, address indexed to)",
  "event Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to)",

  // IUniswapV2Router01
  "function addLiquidity(address tokenA, address tokenB, uint amountADesired, uint amountBDesired, uint amountAMin, uint amountBMin, address to, uint deadline) external returns (uint amountA, uint amountB, uint liquidity)",
  "function addLiquidityETH(address token, uint amountTokenDesired, uint amountTokenMin, uint amountETHMin, address to, uint deadline) external payable returns (uint amountToken, uint amountETH, uint liquidity)",
  "function removeLiquidity(address tokenA, address tokenB, uint liquidity, uint amountAMin, uint amountBMin, address to, uint deadline) external returns (uint amountA, uint amountB)",
  "function removeLiquidityETH(address token, uint liquidity, uint amountTokenMin, uint amountETHMin, address to, uint deadline) external returns (uint amountToken, uint amountETH)",
  "function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)",
  "function swapTokensForExactTokens(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)",
  "function swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline) external payable returns (uint[] memory amounts)",
  "function swapTokensForExactETH(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)",
  "function swapExactTokensForETH(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)",
  "function swapETHForExactTokens(uint amountOut, address[] calldata path, address to, uint deadline) external payable returns (uint[] memory amounts)",

  "function quote(uint amountA, uint reserveA, uint reserveB) external pure returns (uint amountB)",
  "function getAmountOut(uint amountIn, uint reserveIn, uint reserveOut) external pure returns (uint amountOut)",
  "function getAmountIn(uint amountOut, uint reserveIn, uint reserveOut) external pure returns (uint amountIn)",
  "function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)",
  "function getAmountsIn(uint amountOut, address[] calldata path) external view returns (uint[] memory amounts)",
];

/** Functions */

export const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export const assetIdToERC20ContractAddress = (assetId: string | number): string => {
  const asset_id_hex = (+assetId).toString(16).padStart(8, "0");
  return web3.utils.toChecksumAddress(`0xCCCCCCCC${asset_id_hex}000000000000000000000000`);
};

export const collectionIdToERC721Address = (collectionId: string | number): string => {
  const collection_id_hex = (+collectionId).toString(16).padStart(8, "0");
  return web3.utils.toChecksumAddress(`0xAAAAAAAA${collection_id_hex}000000000000000000000000`);
};

// Combine the nextId and parachainId into a single 32-bit value
export const nftCollectionIdToCollectionUUID = (nextId: number | string): number => {
  return (+nextId << 10) | 100; // parachainId = 100
};

// Generate futurepass address given next futurepass id
export const futurepassAddress = (nextId: number | string): string => {
  // address prefix: 0xFFFFFFFF + 32 byte hex
  const fpIdHex = (+nextId).toString(16).padStart(32, "0");
  return web3.utils.toChecksumAddress(`0xFFFFFFFF${fpIdHex}`);
};

// Retrieve the dex liquidity pool address for a given pair of assets
// `0xdddddddd` + <4-byte-asset_a-padded> + <4-byte-asset_b-padded> + `0000000000000000` (8 bytes)
export const poolAddress = (assetA: number | string, assetB: number | string): string => {
  const assetAHex = (+assetA).toString(16).padStart(8, "0");
  const assetBHex = (+assetB).toString(16).padStart(8, "0");

  // lower asset id comes first
  if (assetA < assetB) {
    return web3.utils.toChecksumAddress(`0xDdDddDdD${assetAHex}${assetBHex}0000000000000000`);
  }
  return web3.utils.toChecksumAddress(`0xDdDddDdD${assetBHex}${assetAHex}0000000000000000`);
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
export const getNextAssetId = async (api: ApiPromise, nextAssetId?: string | number): Promise<number> => {
  if (!nextAssetId) {
    nextAssetId = (await api.query.assetsExt.nextAssetId()).toString();
  }
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
 * Saves tx costs(gas to a markdown file
 * @returns
 * @param costs Dictionary of gas costs for different function calls
 * @param filePath The file path to save the output
 * @param header The header for the generated output, i.e. "ERC1155 Precompiles"
 */
export const saveTxGas = (costs: { [key: string]: TxCosts }, filePath: string, header: string) => {
  // Set string headers
  let data: string = `## Generated tx costs(Gas) for ${header}\n\n`;
  data += "| Function Call | Contract gas | Precompile gas | (Extrinsic fee/Gas price) |\n";
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

export const finalizeTx = (
  signer: KeyringPair,
  extrinsic: SubmittableExtrinsic<"promise">,
  opts?: Partial<SignerOptions>,
) => {
  const sendCb =
    (resolve: any, reject: any) =>
    ({ internalError, dispatchError, status, events = [] }: any) => {
      if (internalError) {
        return reject(internalError);
      }

      if (dispatchError && !dispatchError.isModule) {
        return reject(dispatchError.toJSON());
      }

      if (dispatchError && dispatchError.isModule) {
        const { section, name, docs } = dispatchError.registry.findMetaError(dispatchError.asModule);

        return reject({ section, name, docs });
      }

      if (status.isInBlock) resolve(events);
    };

  return new Promise<any[]>((resolve, reject) => {
    if (opts) {
      extrinsic.signAndSend(signer, opts, sendCb(resolve, reject)).catch(reject);
    } else {
      extrinsic.signAndSend(signer, sendCb(resolve, reject)).catch(reject);
    }
  });
};

export const stringToHex = (str: string) => {
  return str
    .split("")
    .map((c) => c.charCodeAt(0).toString(16))
    .join("");
};

/**
 * Saves tx costs(fees) to a markdown file
 * @returns
 * @param costs Dictionary of tx costs for different function calls
 * @param filePath The file path to save the output
 * @param header The header for the generated output, i.e. "ERC1155 Precompiles"
 */
export const saveTxFees = (costs: { [key: string]: TxCosts }, filePath: string, header: string) => {
  // Set string headers
  let data: string = `\n\n## Generated tx costs(fees) for ${header}\n\n`;
  data += "| Function Call | Contract cost (Drops) | Precompile cost (Drops) | Extrinsic cost (Drops) |\n";
  data += "| :--- | :---: | :---: | :---: |\n";

  // Iterate through functions and add tx fees
  for (const key in costs) {
    const value = costs[key];
    data += `| ${key} | ${value.Contract} | ${value.Precompile} | ${value.Extrinsic} |\n`;
  }

  // Prettify data
  data = CliPrettify.prettify(data);

  // Save data to specified file path
  writeFileSync(join("./test", filePath), data, {
    flag: "a",
  });
};

/**
 * Convert extrinsic fee to scaled gas
 * @param provider Provider to get fee data
 * @param fee Extrinsic fee
 */
export async function getScaledGasForExtrinsicFee(provider: JsonRpcProvider, fee: BigNumber) {
  // NOTE - What we do here is not exactly correct. If you want to get the actual equivalent gas for an extrinsic fee,
  // first need to get the weight by reversing substrate tx fee formula. Then use that weight to get the correct gas by
  // reversing runtime weight to gas mapping. But this is rather complex in ts context as the substrate tx formula
  // depends on many factors.
  const feeData = await provider.getFeeData();
  return fee.div(feeData.gasPrice!);
}

/**
 * Converts a value in wei to 6 decimal places
 * @param value
 */
export function weiTo6DP(value: BigNumber) {
  const quotient = value.div(1000000000000n);
  const remainder = value.mod(1000000000000n);

  if (remainder.isZero()) {
    return quotient;
  } else {
    return quotient.add(1n);
  }
}

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

export const generateTestUsers = (userAmount: number): KeyringPair[] => {
  const keyring = new Keyring({ type: "ethereum" });
  return Array.from({ length: userAmount }, () => keyring.addFromUri(`${Math.random().toString(36).substring(2)}`));
};

export const getPrefixLength = (encoded: SubmittableExtrinsic<any>): number => {
  if (encoded.encodedLength < 66) return 6;
  return 8;
};

export enum BurnAuth {
  TokenOwner = 0,
  CollectionOwner,
  Both,
  Neither,
}
