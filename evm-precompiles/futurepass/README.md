# TRN FuturePass Precompile supported interfaces

Precompile address spec: `0xFFFFFFFF[16-byte-futurepass-index]`

```solidity
interface FuturePass {
    event FuturepassDelegateRegistered(address indexed futurepass, address indexed delegate, uint8 proxyType);
    event FuturepassDelegateUnregistered(address indexed futurepass, address delegate);
    event Executed(uint8 indexed callType, address indexed target, uint256 indexed value, bytes4 data);
    event ContractCreated(uint8 indexed callType, address indexed contractAddress, uint256 indexed value, bytes32 salt);
    
    function delegateType(address delegate) external view returns (uint8);

    function registerDelegateWithSignature(address delegate, uint8 proxyType, uint32 deadline, bytes memory signature) external;
    function unregisterDelegate(address delegate) external;
    function proxyCall(uint8 callType, address callTo, uint256 value, bytes memory callData) external payable;
}
```

```solidity
interface Ownable is IERC165 {
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    function owner() external view returns (address);
    function renounceOwnership() external;
    function transferOwnership(address owner) external;
}
```
