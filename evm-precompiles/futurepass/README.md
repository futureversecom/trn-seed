# TRN FutturePass Precompile supported interfaces

```solidity
interface TRNFuturePass {
    event InitializeCollection(address indexed collectionOwner, address precompileAddress);
    function initializeCollection(address owner, bytes name, uint32 maxIssuance, bytes metadataPath, address[] royaltyAddresses, uint32[] royaltyEntitlements) returns (address, uint32);
    /// Defines the proxy permission types.
    enum ProxyType {
        Any,
        NonTransfer,
        Governance,
        Staking,
        CancelProxy,
        Balances,
        AuthorMapping,
        IdentityJudgement
    }
    
    /// 
    function create(address owner) external returns address;
    function register(address futurepass, address delegate) external;
    function unregister(address futurepass, address delegate) external;
    function proxy(address real, address callTo, bytes memory callData) external payable;
}
```
