// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.14;
import "@openzeppelin/contracts/interfaces/IERC20.sol";

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