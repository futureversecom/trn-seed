// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.17;

import "@openzeppelin/contracts/interfaces/IERC20.sol";
import "@openzeppelin/contracts/utils/introspection/IERC165.sol";

// Calls Root network ERC20 precompile
contract ERC20PrecompileCaller {
    // derived XRP token address on testnets (AssetId 2)
    // cccccccc (prefix) + 00000002 (assetId) + padding
    // run through web3.utils.toChecksumAddress(..)
    address public xrpPrecompile = 0xCCCCcCCc00000002000000000000000000000000;

    receive() external payable {}

    // transfer XRP from caller using the Root precompile address w ERC20 abi
    function takeXRP(uint256 amount) external {
        IERC20(xrpPrecompile).transferFrom(msg.sender, address(this), amount);
    }

    // Test sending various XRP amounts via the EVM.
    // destination should have 0 balance to start.
    function sendXRPAmounts(address payable destination) public payable {
        assert(address(destination).balance == 0);
        uint64[6] memory amounts_18 = [1000000500000000000 wei, 1000000000000000001 wei, 1000001000000000000 wei, 1000000000000000000 wei, 1 wei, 0 wei];
        uint24[6] memory amounts_6 = [1000001, 1000001, 1000001, 1000000, 1, 0];
        uint256 total;

        for(uint i; i < 5; i++) {
            (bool sent, ) = destination.call{value: uint256(amounts_18[i])}("");
            require(sent, "Failed to send XRP");
            total += (uint256(amounts_6[i]) * uint256(1e12));
            require(total == address(destination).balance, "unexpected balance");
        }
    }
}

contract ERC20PrecompileERC165Validator {
    // Store ERC165 interface ID as constant
    bytes4 public constant ERC165_ID = type(IERC165).interfaceId;
    
    function calculateERC20InterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("totalSupply()") ^
            keccak256("balanceOf(address)") ^
            keccak256("transfer(address,uint256)") ^
            keccak256("allowance(address,address)") ^
            keccak256("approve(address,uint256)") ^
            keccak256("transferFrom(address,address,uint256)")
        );
    }

    function calculateERC20MetadataInterfaceId() public pure returns (bytes4) {
        return bytes4(
            keccak256("name()") ^
            keccak256("symbol()") ^
            keccak256("decimals()")
        );
    }

    function validateContract(address contractAddress) public view returns (
        bool supportsERC165,
        bool supportsERC20,
        bool supportsERC20Metadata
    ) {
        IERC165 target = IERC165(contractAddress);
        
        try target.supportsInterface(ERC165_ID) returns (bool erc165Support) {
            supportsERC165 = erc165Support;
            
            if (erc165Support) {
                try target.supportsInterface(calculateERC20InterfaceId()) returns (bool support) {
                    supportsERC20 = support;
                } catch {}
                
                try target.supportsInterface(calculateERC20MetadataInterfaceId()) returns (bool support) {
                    supportsERC20Metadata = support;
                } catch {}
            }
        } catch {}
    }

    function getAllInterfaceIds() public pure returns (
        bytes4 erc165,
        bytes4 erc20,
        bytes4 erc20Metadata
    ) {
        return (
            ERC165_ID,
            calculateERC20InterfaceId(),
            calculateERC20MetadataInterfaceId()
        );
    }
}
