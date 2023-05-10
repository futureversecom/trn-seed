# TRN FuturePass Precompile supported interfaces

Precompile address: `0xffffffff[16 byte futurepass index]`

```solidity
interface FuturePass {
    event FuturepassDelegateRegistered(address indexed futurepass, address indexed delegate, uint8 proxyType);
    event FuturepassDelegateUnregistered(address indexed futurepass, address delegate);
    
    function delegateType(address delegate) external view returns (uint8);

    function registerDelegate(address delegate, uint8 proxyType) external;
    function unregisterDelegate(address delegate) external;
    function proxyCall(uint8 callType, address callTo, uint256 value, bytes memory callData) external payable;
}
```
