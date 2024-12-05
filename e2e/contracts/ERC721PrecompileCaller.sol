// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.17;

import "@openzeppelin/contracts/interfaces/IERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/IERC721Metadata.sol";

contract ERC721PrecompileCaller {
    address public precompile;

    constructor(address _precompile) {
        precompile = _precompile;
    }

    function balanceOfProxy(address who) public view returns (uint256) {
        return IERC721(precompile).balanceOf(who);
    }

    function ownerOfProxy(uint256 tokenId) public view returns (address) {
        return IERC721(precompile).ownerOf(tokenId);
    }

    function getApprovedProxy(uint256 tokenId) public view returns (address) {
        return IERC721(precompile).getApproved(tokenId);
    }

    function isApprovedForAllProxy(address owner, address operator) public view returns (bool) {
        return IERC721(precompile).isApprovedForAll(owner, operator);
    }

    function nameProxy() public view returns (string memory) {
        return IERC721Metadata(precompile).name();
    }

    function symbolProxy() public view returns (string memory) {
        return IERC721Metadata(precompile).symbol();
    }

    function tokenURIProxy(uint256 tokenId) public view returns (string memory) {
        return IERC721Metadata(precompile).tokenURI(tokenId);
    }

    function transferFromProxy(
        address from,
        address to,
        uint256 tokenId
    ) external {
        IERC721(precompile).transferFrom(from, to, tokenId);
    }

    function safeTransferFromProxy(
        address from,
        address to,
        uint256 tokenId
    ) external {
        IERC721(precompile).safeTransferFrom(from, to, tokenId);
    }

    function setApprovalForAllProxy(
        address operator,
        bool approved
    ) external {
        IERC721(precompile).setApprovalForAll(operator, approved);
    }

    function approveProxy(
        address who,
        uint256 tokenId
    ) external {
        IERC721(precompile).approve(who, tokenId);
    }
}

contract ERC721PrecompileERC165Validator {
    // Store interface IDs as constants after calculation
    bytes4 public constant ERC165_ID = type(IERC165).interfaceId;
    
    // Calculate interface IDs based on function selectors
    function calculateERC721InterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("balanceOf(address)") ^
            keccak256("ownerOf(uint256)") ^
            keccak256("safeTransferFrom(address,address,uint256)") ^
            keccak256("transferFrom(address,address,uint256)") ^
            keccak256("approve(address,uint256)") ^
            keccak256("getApproved(uint256)") ^
            keccak256("setApprovalForAll(address,bool)") ^
            keccak256("isApprovedForAll(address,address)") ^
            keccak256("safeTransferFrom(address,address,uint256,bytes)")
        );
    }

    function calculateERC721MetadataInterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("name()") ^
            keccak256("symbol()") ^
            keccak256("tokenURI(uint256)")
        );
    }

    function calculateERC721BurnableInterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("burn(uint256)")
        );
    }

    function calculateTRN721InterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("totalSupply()") ^
            keccak256("mint(address,uint32)") ^
            keccak256("setMaxSupply(uint32)") ^
            keccak256("setBaseURI(bytes)") ^
            keccak256("ownedTokens(address,uint16,uint32)") ^
            keccak256("togglePublicMint(bool)") ^
            keccak256("setMintFee(address,uint256)")
        );
    }

    function calculateOwnableInterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("owner()") ^
            keccak256("renounceOwnership()") ^
            keccak256("transferOwnership(address)")
        );
    }

    // Validation functions
    function validateContract(address contractAddress) public view returns (
        bool supportsERC165,
        bool supportsERC721,
        bool supportsERC721Metadata,
        bool supportsERC721Burnable,
        bool supportsTRN721,
        bool supportsOwnable
    ) {
        IERC165 target = IERC165(contractAddress);
        
        // First check ERC165 support
        try target.supportsInterface(ERC165_ID) returns (bool erc165Support) {
            supportsERC165 = erc165Support;
            
            if (erc165Support) {
                // Only check other interfaces if ERC165 is supported
                try target.supportsInterface(calculateERC721InterfaceId()) returns (bool support) {
                    supportsERC721 = support;
                } catch {}
                
                try target.supportsInterface(calculateERC721MetadataInterfaceId()) returns (bool support) {
                    supportsERC721Metadata = support;
                } catch {}
                
                try target.supportsInterface(calculateERC721BurnableInterfaceId()) returns (bool support) {
                    supportsERC721Burnable = support;
                } catch {}
                
                try target.supportsInterface(calculateTRN721InterfaceId()) returns (bool support) {
                    supportsTRN721 = support;
                } catch {}
                
                try target.supportsInterface(calculateOwnableInterfaceId()) returns (bool support) {
                    supportsOwnable = support;
                } catch {}
            }
        } catch {}
    }

    // Get all interface IDs at once
    function getAllInterfaceIds() public pure returns (
        bytes4 erc165,
        bytes4 erc721,
        bytes4 erc721Metadata,
        bytes4 erc721Burnable,
        bytes4 trn721,
        bytes4 ownable
    ) {
        return (
            ERC165_ID,
            calculateERC721InterfaceId(),
            calculateERC721MetadataInterfaceId(),
            calculateERC721BurnableInterfaceId(),
            calculateTRN721InterfaceId(),
            calculateOwnableInterfaceId()
        );
    }
}
