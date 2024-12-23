// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.17;
import "@openzeppelin/contracts/interfaces/IERC1155.sol";
import "@openzeppelin/contracts/token/ERC1155/extensions/IERC1155MetadataURI.sol";

contract ERC1155PrecompileCaller {
    address public precompile;

    constructor(address _precompile) {
        precompile = _precompile;
    }

    function balanceOfProxy(address who, uint256 id) public view returns (uint256) {
        return IERC1155(precompile).balanceOf(who, id);
    }

    function balanceOfBatchProxy(address[] memory owners, uint256[] memory ids) public view returns (uint256[] memory) {
        return IERC1155(precompile).balanceOfBatch(owners, ids);
    }

    function setApprovalForAllProxy(address operator, bool approved) external {
        IERC1155(precompile).setApprovalForAll(operator, approved);
    }

    function isApprovedForAllProxy(address owner, address operator) public view returns (bool) {
        return IERC1155(precompile).isApprovedForAll(owner, operator);
    }

    function safeTransferFromProxy(
        address from,
        address to,
        uint256 id,
        uint256 amount,
        bytes memory data
    ) external {
        IERC1155(precompile).safeTransferFrom(from, to, id, amount, data);
    }

    function safeBatchTransferFromProxy(
        address from,
        address to,
        uint256[] memory ids,
        uint256[] memory amounts,
        bytes memory data
    ) external {
        IERC1155(precompile).safeBatchTransferFrom(from, to, ids, amounts, data);
    }

    function uriProxy(uint256 id) public view returns (string memory) {
        return IERC1155MetadataURI(precompile).uri(id);
    }
}

contract ERC1155PrecompileERC165Validator {
    // Store ERC165 interface ID as constant
    bytes4 public constant ERC165_ID = type(IERC165).interfaceId;
    
    function calculateERC1155InterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("balanceOf(address,uint256)") ^
            keccak256("balanceOfBatch(address[],uint256[])") ^
            keccak256("setApprovalForAll(address,bool)") ^
            keccak256("isApprovedForAll(address,address)") ^
            keccak256("safeTransferFrom(address,address,uint256,uint256,bytes)") ^
            keccak256("safeBatchTransferFrom(address,address,uint256[],uint256[],bytes)")
        );
    }

    function calculateERC1155BurnableInterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("burn(address,uint256,uint256)") ^
            keccak256("burnBatch(address,uint256[],uint256[])")
        );
    }

    function calculateERC1155SupplyInterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("totalSupply(uint256)") ^
            keccak256("exists(uint256)")
        );
    }

    function calculateERC1155MetadataURIInterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("uri(uint256)")
        );
    }

    function calculateTRN1155InterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("createToken(bytes,uint128,uint128,address)") ^
            keccak256("mint(address,uint256,uint256)") ^
            keccak256("mintBatch(address,uint256[],uint256[])") ^
            keccak256("setMaxSupply(uint256,uint32)") ^
            keccak256("setBaseURI(bytes)") ^
            keccak256("togglePublicMint(uint256,bool)") ^
            keccak256("setMintFee(uint256,address,uint256)")
        );
    }

    function calculateOwnableInterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("owner()") ^
            keccak256("renounceOwnership()") ^
            keccak256("transferOwnership(address)")
        );
    }

    function validateContract(address contractAddress) public view returns (
        bool supportsERC165,
        bool supportsERC1155,
        bool supportsERC1155Burnable,
        bool supportsERC1155Supply,
        bool supportsERC1155MetadataURI,
        bool supportsTRN1155,
        bool supportsOwnable
    ) {
        IERC165 target = IERC165(contractAddress);
        
        try target.supportsInterface(ERC165_ID) returns (bool erc165Support) {
            supportsERC165 = erc165Support;
            
            if (erc165Support) {
                try target.supportsInterface(calculateERC1155InterfaceId()) returns (bool support) {
                    supportsERC1155 = support;
                } catch {}
                
                try target.supportsInterface(calculateERC1155BurnableInterfaceId()) returns (bool support) {
                    supportsERC1155Burnable = support;
                } catch {}

                try target.supportsInterface(calculateERC1155SupplyInterfaceId()) returns (bool support) {
                    supportsERC1155Supply = support;
                } catch {}

                try target.supportsInterface(calculateERC1155MetadataURIInterfaceId()) returns (bool support) {
                    supportsERC1155MetadataURI = support;
                } catch {}

                try target.supportsInterface(calculateTRN1155InterfaceId()) returns (bool support) {
                    supportsTRN1155 = support;
                } catch {}

                try target.supportsInterface(calculateOwnableInterfaceId()) returns (bool support) {
                    supportsOwnable = support;
                } catch {}
            }
        } catch {}
    }

    function getAllInterfaceIds() public pure returns (
        bytes4 erc165,
        bytes4 erc1155,
        bytes4 erc1155Burnable,
        bytes4 erc1155Supply,
        bytes4 erc1155MetadataURI,
        bytes4 trn1155,
        bytes4 ownable
    ) {
        return (
            ERC165_ID,
            calculateERC1155InterfaceId(),
            calculateERC1155BurnableInterfaceId(),
            calculateERC1155SupplyInterfaceId(),
            calculateERC1155MetadataURIInterfaceId(),
            calculateTRN1155InterfaceId(),
            calculateOwnableInterfaceId()
        );
    }
}
