# TRN FuturePass Precompile supported interfaces

```solidity
interface FuturePass {
    event FuturepassCreated(address indexed futurepass, address owner);
    event FuturepassDelegateRegistered(address indexed futurepass, address delegate);
    event FuturepassDelegateUnregistered(address indexed futurepass, address delegate);
    
    function create(address owner) external returns (address);
    function registerDelegate(address futurepass, address delegate) external;
    function unregisterDelegate(address futurepass, address delegate) external;
    function proxyCall(address real, address callTo, bytes memory callData) external payable;
    function isDelegate(address futurepass, address delegate) external returns (bool);
}
```
