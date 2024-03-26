# TRN Marketplace Precompile supported interfaces

Precompile address: `0x00000000000000000000000000000000000006CD`

### TODO add more details 

```solidity
interface Marketplace {
    event MarketplaceRegister(address indexed sender, uint256 indexed marketplaceId, address marketplace_account);
    event FixedPriceSaleListNFT(address indexed seller, uint256 indexed listingId, uint256 indexed fixedPrice, uint256[] serialNumbers, address collectionAddress, uint128 marketplace_id);
    event FixedPriceSaleListSFT(address indexed seller, uint256 indexed listingId, uint256 indexed fixedPrice, uint256[] serialNumbers, address collectionAddress, uint128 marketplace_id, uint256[] serialNumbers);
    event FixedPriceSaleUpdate(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed newPrice, address sender, uint256[] serialNumbers, uint128 marketplace_id);
    event FixedPriceSaleComplete(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed fixedPrice, address sender, uint256[] serialNumbers, uint128 marketplace_id);
    event AuctionOpenNFT(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed reservePrice, address sender, uint256[] serialNumbers, uint128 marketplace_id);
    event AuctionOpenSFT(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed reservePrice, address sender, uint256[] serialNumbers, uint128 marketplace_id);
    event Bid(address indexed bidder, uint256 indexed listingId, uint256 indexed amount, uint128 marketplace_id);
    event FixedPriceSaleClose(uint256 indexed collectionId, uint256 indexed listingId, address sender, uint256[] serialNumbers, uint128 marketplace_id); //uint256;uint256;address;uint256[]
    event AuctionClose(uint256 indexed collectionId, uint256 indexed listingId, address sender, uint256[] serialNumbers, uint128 marketplace_id);
    event Offer(uint256 indexed offerId, address indexed sender, uint256 collectionId, uint256 seriesId, uint128 marketplace_id);
    event OfferCancel(uint256 indexed offerId, address indexed sender, uint256 collectionId, uint256 seriesId, uint128 marketplace_id);
    event OfferAccept(uint256 indexed offerId, uint256 indexed amount, address indexed sender, uint256 collectionId, uint256 seriesId, uint128 marketplace_id); // uint256;uint256;address;uint256

    function registerMarketplace(address marketplaceAccount, uint256 entitlement) external returns (uint marketplaceId);
    // sellNftWithMarketplaceId is deprecated for sellNFT
    function sellNftWithMarketplaceId(address collectionAddress, uint256[] calldata serialNumberIds, address buyer, address paymentAsset, uint256 fixedPrice, uint256 duration, uint32 marketplaceId) external returns (uint listingId);
    function sellNft(address collectionAddress, uint256[] calldata serialNumberIds, address buyer, address paymentAsset, uint256 fixedPrice, uint256 duration, uint128 marketplace_id) external returns (uint listingId);
    function sellSft(address collectionAddress, uint256[] calldata serialNumberIds, uint256[] calldata quantities, address buyer, address paymentAsset, uint256 fixedPrice, uint256 duration, uint128 marketplace_id) external returns (uint listingId);
    function updateFixedPrice(uint128 listingId, uint256 newPrice) external;
    function buy(uint128 listingId) external payable";
    // auctionNftWithMarketplaceId is deprecated for auctionNft
    function auctionNftWithMarketplaceId(address collectionAddress, uint256[] calldata serialNumberIds, address paymentAsset, uint256 reservePrice, uint256 duration, uint256 marketplaceId);
    function auctionNft(address collectionAddress, uint256[] calldata serialNumberIds, address paymentAsset, uint256 reservePrice, uint256 duration, uint256 marketplaceId);
    function auctionSft(address collectionAddress, uint256[] calldata serialNumberIds, uint256[] calldata quantities, address paymentAsset, uint256 reservePrice, uint256 duration, uint256 marketplaceId);
    function bid(uint128 listingId, uint256 amount) external;
    function cancelSale(uint128 listingId) external;
    // makeSimpleOfferWithMarketplaceId is deprecated for makeSimpleOffer
    function makeSimpleOfferWithMarketplaceId(address collectionAddress, uint32 serialNumber, uint256 amount, address assetId, uint32 marketplaceId) external returns (uint offerId);
    function makeSimpleOffer(address collectionAddress, uint32 serialNumber, uint256 amount, address assetId, uint32 marketplaceId) external returns (uint offerId);
    function cancelOffer(uint64 offerId) external;
    function acceptOffer(uint64 offerId) external;

  // read
  function getMarketplaceAccount(uint32 marketplaceId) external view returns(address marketplaceAccount);
  function getListingFromId(uint128 listingId) external view returns (bytes type, uint32 collectionId, uint32[] calldata serial_numbers, uint128 price, uint32 paymentAsset);
  function getOfferFromId(uint64 offerId) external view returns (uint32 collectionId, uint32 serial_number, uint128 amount, address buyer);
}
```
