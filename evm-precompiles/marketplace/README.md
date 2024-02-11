# TRN Marketplace Precompile supported interfaces

Precompile address: `0x00000000000000000000000000000000000006CD`

### TODO add more details 

```solidity
interface Marketplace {
    event MarketplaceRegister(address indexed sender, uint256 indexed marketplaceId, address marketplace_account);
    event FixedPriceSaleList(address indexed seller, uint256 indexed listingId, uint256 indexed fixedPrice, uint256[] serialNumbers, address collectionAddress);
    event FixedPriceSaleUpdate(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed newPrice, address sender, uint256[] serialNumbers);
    event FixedPriceSaleComplete(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed fixedPrice, address sender, uint256[] serialNumbers);
    event AuctionOpen(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed reservePrice, address sender, uint256[] serialNumbers),
    event Bid(address indexed bidder, uint256 indexed listingId, uint256 indexed amount);
    event FixedPriceSaleClose(uint256 indexed collectionId, uint256 indexed listingId, address sender, uint256[] serialNumbers); //uint256,uint256,address,uint256[]
    event AuctionClose(uint256 indexed collectionId, uint256 indexed listingId, address sender, uint256[] serialNumbers);
    event Offer(uint256 indexed offerId, address indexed sender, uint256 collectionId, uint256 seriesId);
    event OfferCancel(uint256 indexed offerId, address indexed sender, uint256 collectionId, uint256 seriesId);
    event OfferAccept(uint256 indexed offerId, uint256 indexed amount, address indexed sender, uint256 collectionId, uint256 seriesId); // uint256,uint256,address,uint256
    
    function registerMarketplace(address marketplaceAccount, uint256 entitlement) external returns (uint marketplaceId);
    function sellNftWithMarketplaceId(address collectionAddress, uint256[] calldata serialNumberIds, address buyer, address paymentAsset, uint256 fixedPrice, uint256 duration, uint32 marketplaceId) external returns (uint listingId);
    function sellNftWithoutMarketplace(address collectionAddress, uint256[] calldata serialNumberIds, address buyer, address paymentAsset, uint256 fixedPrice, uint256 duration) external returns (uint listingId);
    function updateFixedPrice(uint128 listingId, uint256 newPrice) external;
    function buy(uint128 listingId) external payable;
    function auctionNftWithMarketplaceId(address collectionAddress, uint256[] calldata serialNumberIds, address paymentAsset, uint256 reservePrice, uint256 duration, uint256 marketplaceId) external payable;
    function auctionNftWithoutMarketplace(address collectionAddress, uint256[] calldata serialNumberIds, address paymentAsset, uint256 reservePrice, uint256 duration) external payable;
    function bid(uint128 listingId, uint256 amount) external;
    function cancelSale(uint128 listingId) external;
    function makeSimpleOfferWithMarketplaceId(address collectionAddress, uint32 serialNumber, uint256 amount, address assetId, uint32 marketplaceId) external returns (uint offerId);
    function makeSimpleOfferWithoutMarketplace(address collectionAddress, uint32 serialNumber, uint256 amount, address assetId) external returns (uint offerId);
    function cancelOffer(uint64 offerId) external;
    function acceptOffer(uint64 offerId) external;

    // read
    function getMarketplaceAccount(uint32 marketplaceId) external view returns(address marketplaceAccount);
    function getListingFromId(uint128 listingId) external view returns (uint32 collectionId, uint32[] calldata serialNumbers, uint128 price, uint32 paymentAsset);
    function getOfferFromId(uint64 offerId) external view returns (uint32 collectionId, uint32 serialNumber, uint128 amount, address buyer);
}
```
