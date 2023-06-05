# TRN ERC1155 Precompile supported interfaces

Precompile address spec: `0xBBBBBBBB[4-byte-collection-id]000000000000000000000000`

```solidity
interface IERC1155 is IERC165 {
    event TransferSingle(address indexed operator, address indexed from, address indexed to, uint256 id, uint256 value);
    event TransferBatch(address indexed operator, address indexed from, address indexed to, uint256[] ids, uint256[] values);
    event ApprovalForAll(address indexed account, address indexed operator, bool approved);

    function balanceOf(address owner, uint256 id) external view returns (uint256);
    function balanceOfBatch(address[] owners, uint256[] ids) external view returns (uint256[] memory);
    function setApprovalForAll(address operator, bool approved) external;
    function isApprovedForAll(address account, address operator) external view returns (bool);
    function safeTransferFrom(address from, address to, uint256 id, uint256 amount, bytes calldata data) external;
    function safeBatchTransferFrom(address from, address to, uint256[] calldata ids, uint256[] calldata amounts, bytes calldata data) external;
}
```

```solidity
interface IERC1155Burnable is IERC1155 {
    function burn(address account, uint256 id, uint256 value) external;
    function burnBatch(address account, uint256[] ids, uint256[] values) external;
}
```

```solidity
interface IERC1155Supply is IERC1155 {
    function totalSupply(uint256 id) external view returns (uint256);
    function exists(uint256 id) external view returns (bool);
}
```

```solidity
interface IERC1155MetadataURI is IERC1155 {
    function uri(uint256 id) external view returns (string memory);
}
```

```solidity
interface TRN1155 is IERC165 {
    event TokenCreated(uint32 serialNumber);
    event MaxSupplyUpdated(uint128 maxSupply);
    event BaseURIUpdated(string baseURI);

    function createToken(bytes name, uint128 maxIssuance) external returns (uint32);
    function mint(address owner, uint256 id, uint256 amount) external;
    function mintBatch(address owner, uint256[] ids, uint256[] amounts) external;
    function setMaxSupply(uint256 id, uint32 maxSupply) external;
    function setBaseURI(bytes baseURI) external;
}
```

```solidity
interface Ownable is IERC165 {
    event OwnershipTransferred(address indexed previousOwner, address newOwner);

    function owner() external view returns (address);
    function renounceOwnership() external;
    function transferOwnership(address owner) external;
}
```
