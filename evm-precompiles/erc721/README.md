# TRN ERC721 Precompile supported interfaces

Precompile address spec: `0xAAAAAAAA[4-byte-collection-id]000000000000000000000000`

```solidity
interface IERC721 is IERC165 {
    event Transfer(address indexed from, address indexed to, uint256 indexed tokenId);
    event Approval(address indexed owner, address indexed approved, uint256 indexed tokenId);
    event ApprovalForAll(address indexed owner, address indexed operator, bool approved);

    function balanceOf(address owner) external view returns (uint256 balance);
    function ownerOf(uint256 tokenId) external view returns (address owner);
    function safeTransferFrom(address from, address to, uint256 tokenId) external;
    function transferFrom(address from, address to, uint256 tokenId) external;
    function approve(address to, uint256 tokenId) external;
    function getApproved(uint256 tokenId) external view returns (address operator);
    function setApprovalForAll(address operator, bool _approved) external;
    function isApprovedForAll(address owner, address operator) external view returns (bool);
    function safeTransferFrom(address from,address to,uint256 tokenId,bytes calldata data) external;
}
```

```solidity
interface IERC721Metadata is IERC721 {
    function name() external view returns (string memory);
    function symbol() external view returns (string memory);
    function tokenURI(uint256 tokenId) external view returns (string memory);
}
```

```solidity
interface TRN721 is IERC165 {
    event MaxSupplyUpdated(uint32 maxSupply);
    event BaseURIUpdated(string baseURI);

    function totalSupply() external view returns (uint256);
    function mint(address owner, uint32 quantity) external;
    function setMaxSupply(uint32 maxSupply) external;
    function setBaseURI(bytes calldata baseURI) external;
    function ownedTokens(address who, uint16 limit, uint32 cursor) external view returns (uint32, uint32, uint32[] memory);
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
