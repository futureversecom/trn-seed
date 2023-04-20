# TRN FuturePass Precompile supported interfaces

Precompile address: `0x000000000000000000000000000000000000FFFF`

```solidity
interface FuturePass {
    event FuturepassCreated(address indexed futurepass, address owner);
    event FuturepassDelegateRegistered(address indexed futurepass, address indexed delegate, uint8 proxyType);
    event FuturepassDelegateUnregistered(address indexed futurepass, address delegate);
    
    function futurepassOf(address owner) external view returns (address);
    function isDelegate(address futurepass, address delegate) external returns (bool);
    function delegateType(address futurepass, address delegate) external returns (uint8);
    function create(address owner) external returns (address);
    function registerDelegate(address futurepass, address delegate, uint8 proxyType) external;
    function unregisterDelegate(address futurepass, address delegate) external;
    function proxyCall(address futurepass, address callTo, uint8 callType, bytes memory callData) external payable;
}
```
