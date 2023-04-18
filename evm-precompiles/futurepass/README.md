# TRN FuturePass Precompile supported interfaces

Precompile address: `0x000000000000000000000000000000000000FFFF`

```solidity
interface FuturePass {
    event FuturepassCreated(address indexed futurepass, address owner);
    event FuturepassDelegateRegistered(address indexed futurepass, address delegate);
    event FuturepassDelegateUnregistered(address indexed futurepass, address delegate);
    
    function futurepassOf(address owner) external view returns (address);
    function create(address owner) external returns (address);
    function registerDelegate(address futurepass, address delegate, uint8 proxyType) external;
    function unregisterDelegate(address futurepass, address delegate, uint8 proxyType) external;
    function proxyCall(address futurepass, address callTo, uint8 proxyType, bytes memory callData, uint8 callType) external payable;
    function isDelegate(address futurepass, address delegate, uint8 proxyType) external returns (bool);
}
```
