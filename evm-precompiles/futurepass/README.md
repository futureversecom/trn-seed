# TRN FuturePass Precompile supported interfaces

Precompile address: `0x000000000000000000000000000000000000FFFF`

```solidity
interface FuturePass {
    event FuturepassDelegateRegistered(address indexed futurepass, address indexed delegate, uint8 proxyType);
    event FuturepassDelegateUnregistered(address indexed futurepass, address delegate);
    
    function isDelegate(address delegate) external view returns (bool);
    function delegateType(address delegate) external view returns (uint8);

    function registerDelegate(address delegate, uint8 proxyType) external;
    function unregisterDelegate(address delegate) external;
    function proxyCall(address callTo, uint8 callType, bytes memory callData) external payable;
}
```
