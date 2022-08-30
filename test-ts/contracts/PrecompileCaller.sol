// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

// Calls Root network ERC20 precompile
contract PrecompileCaller {
    // derived XRP token address on testnets (AssetId 2)
    // cccccccc (prefix) + 00000002 (assetId) + padding
    // run through web3.utils.toChecksumAddress(..)
    address root = 0xCCCCcCCc00000002000000000000000000000000;

    receive() external payable {}

    function balanceOfProxy(address who) public view returns (uint256) {
        (bool success, bytes memory returnData) = root.staticcall(abi.encodeWithSignature("balanceOf(address)", who));
        assembly {
            if eq(success, 0) {
                revert(add(returnData, 0x20), returndatasize())
            }
        }
        return abi.decode(returnData, (uint256));
    }

    // transfer XRP from caller using the Root precompile address w ERC20 abi
    function takeXRP(uint256 amount) external {
        (bool success, bytes memory returnData) = root.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, address(this), amount));
        assembly {
            if eq(success, 0) {
                revert(add(returnData, 0x20), returndatasize())
            }
        }
    }

    // Test sending various XRP amounts via the EVM.
    // destination should have 0 balance to start.
    function sendXRPAmounts(address payable destination) public payable {
        assert(address(destination).balance == 0);
        uint64[8] memory amounts_18 = [1 ether, 1000000500000000000 wei, 1000000000000000001 wei, 1000001000000000000 wei, 1000000000000000000 wei, 999 wei, 1 wei, 0 wei];
	    uint24[8] memory amounts_6 = [1000000, 1000001, 1000001, 1000001, 1000000, 1, 1, 0];
        uint256 total;

        for(uint i; i < 8; i++) {
            (bool sent, bytes memory _data) = destination.call{value: uint256(amounts_18[i])}("");
            require(sent, "Failed to send XRP");
            total += (uint256(amounts_6[i]) * uint256(1e12));
            require(total == address(destination).balance, "unexpected balance");
        }
    }
}