# TRN ERC721 Precompile supported interfaces

```solidity
interface TRNNFT is IERC165 {
    event InitializeCollection(address indexed collectionOwner, address precompileAddress);
    function initializeCollection(address owner, bytes name, uint32 maxIssuance, bytes metadataPath, address[] royaltyAddresses, uint32[] royaltyEntitlements) returns (address, uint32);
}
```
