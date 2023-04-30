# TRN FuturePass Registrar Precompile supported interfaces

Precompile address: `0x000000000000000000000000000000000000FFFF`

```solidity
interface FuturePassRegistrar {
    event FuturepassCreated(address indexed futurepass, address owner);
    
    function futurepassOf(address owner) external view returns (address);
    function create(address owner) external returns (address);
}
```
