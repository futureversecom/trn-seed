# TRN FutturePass Precompile supported interfaces

```solidity
interface TRNFuturePass {
    event FuturepassCreated(address indexed futurepass, address owner);
    event FuturepassDelegateRegistered(address indexed futurepass, address delegate);
    event FuturepassDelegateUnregistered(address indexed futurepass, address delegate);
    
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
    function create(address owner) external returns (address);
    function register(address futurepass, address delegate) external;
    function unregister(address futurepass, address delegate) external;
    function proxy(address real, address callTo, bytes memory callData) external payable;
}
```
