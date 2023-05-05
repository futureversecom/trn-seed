# TRN ERC721 Precompile supported interfaces

Precompile address: `0x00000000000000000000000000000000000006b9`

```solidity
interface TRNNFT is IERC165 {
    event InitializeCollection(address indexed collectionOwner, address precompileAddress);

    function initializeCollection(address owner, bytes calldata name, uint32 maxIssuance, bytes calldata metadataPath, address[] calldata royaltyAddresses, uint32[] calldata royaltyEntitlements) external returns (address, uint32);
}
```
