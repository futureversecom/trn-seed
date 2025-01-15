# TRN ERC20 Precompile supported interfaces

Precompile address spec: `0xCCCCCCCC[4-byte-token-id]000000000000000000000000`

```solidity
interface IERC165 {
    function supportsInterface(bytes4 interfaceId) external view returns (bool);
}
```

```solidity
interface IERC20 is IERC165 {
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    function totalSupply() external view returns (uint256);
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
    function allowance(address owner, address spender) external view returns (uint256);
    function approve(address spender, uint256 amount) external returns (bool);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}
```

```solidity
interface IERC20Metadata is IERC20 {
    function name() external view returns (string memory);
    function symbol() external view returns (string memory);
    function decimals() external view returns (uint8);
}
```
