# TRN Marketplace Precompile supported interfaces

Precompile address: `0x00000000000000000000000000000000000006CD`

```solidity
interface IMarketplace {
    event MarketplaceRegister(address indexed sender, uint256 indexed marketplaceId, address marketplaceAccount);
    event FixedPriceSaleListNFT(address indexed seller, uint256 indexed listingId, uint256 indexed fixedPrice, uint256[] serialNumbers, address collectionAddress, uint128 marketplaceId);
    event FixedPriceSaleListSFT(address indexed seller, uint256 indexed listingId, uint256 indexed fixedPrice, uint256[] serialNumbers, address collectionAddress, uint128 marketplaceId, uint256[] quantities);
    event FixedPriceSaleUpdate(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed newPrice, address sender, uint256[] serialNumbers, uint128 marketplaceId);
    event FixedPriceSaleComplete(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed fixedPrice, address sender, uint256[] serialNumbers, uint128 marketplaceId);
    event AuctionOpenNFT(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed reservePrice, address sender, uint256[] serialNumbers, uint128 marketplaceId);
    event AuctionOpenSFT(uint256 indexed collectionId, uint256 indexed listingId, uint256 indexed reservePrice, address sender, uint256[] serialNumbers, uint128 marketplaceId);
    event Bid(address indexed bidder, uint256 indexed listingId, uint256 indexed amount, uint128 marketplaceId);
    event FixedPriceSaleClose(uint256 indexed collectionId, uint256 indexed listingId, address sender, uint256[] serialNumbers, uint128 marketplaceId);
    event AuctionClose(uint256 indexed collectionId, uint256 indexed listingId, address sender, uint256[] serialNumbers, uint128 marketplaceId);
    event Offer(uint256 indexed offerId, address indexed sender, uint256 collectionId, uint256 seriesId, uint128 marketplaceId);
    event OfferCancel(uint256 indexed offerId, address indexed sender, uint256 collectionId, uint256 seriesId, uint128 marketplaceId);
    event OfferAccept(uint256 indexed offerId, uint256 indexed amount, address indexed sender, uint256 collectionId, uint256 seriesId, uint128 marketplaceId);

    function registerMarketplace(address marketplaceAccount, uint256 entitlement) external returns (uint marketplaceId);
    // sellNftWithMarketplaceId is deprecated for sellNFT
    function sellNftWithMarketplaceId(address collectionAddress, uint32[] calldata serialNumberIds, address buyer, address paymentAsset, uint128 fixedPrice, uint128 duration, uint32 marketplaceId) external returns (uint listingId);
    function sellNft(address collectionAddress, uint32[] calldata serialNumberIds, address buyer, address paymentAsset, uint128 fixedPrice, uint128 duration, uint32 marketplaceId) external returns (uint listingId);
    function sellSft(address collectionAddress, uint32[] calldata serialNumberIds, uint128[] calldata quantities, address buyer, address paymentAsset, uint128 fixedPrice, uint128 duration, uint32 marketplaceId) external returns (uint listingId);
    function updateFixedPrice(uint128 listingId, uint128 newPrice) external;
    function buy(uint128 listingId) external;

    // auctionNftWithMarketplaceId is deprecated for auctionNft
    function auctionNftWithMarketplaceId(address collectionAddress, uint32[] calldata serialNumberIds, address paymentAsset, uint128 reservePrice, uint128 duration, uint32 marketplaceId) external;
    function auctionNft(address collectionAddress, uint32[] calldata serialNumberIds, address paymentAsset, uint128 reservePrice, uint128 duration, uint32 marketplaceId) external;
    function auctionSft(address collectionAddress, uint32[] calldata serialNumberIds, uint128[] calldata quantities, address paymentAsset, uint128 reservePrice, uint128 duration, uint32 marketplaceId) external;
    function bid(uint128 listingId, uint128 amount) external;


    function cancelSale(uint128 listingId) external;

    // makeSimpleOfferWithMarketplaceId is deprecated for makeSimpleOffer
    function makeSimpleOfferWithMarketplaceId(address collectionAddress, uint32 serialNumber, uint128 amount, address assetId, uint32 marketplaceId) external returns (uint offerId);
    function makeSimpleOffer(address collectionAddress, uint32 serialNumber, uint128 amount, address assetId, uint32 marketplaceId) external returns (uint offerId);
    function cancelOffer(uint64 offerId) external;
    function acceptOffer(uint64 offerId) external;

    // read
    function getMarketplaceAccount(uint32 marketplaceId) external view returns(address marketplaceAccount);
    function getListingFromId(uint128 listingId) external view returns (bytes memory listingType, uint32 collectionId, uint32[] calldata serialNumbers, uint128 reservePrice, uint32 paymentAsset);
    function getOfferFromId(uint64 offerId) external view returns (uint32 collectionId, uint32 serialNumber, uint128 amount, address buyer);
}
```
