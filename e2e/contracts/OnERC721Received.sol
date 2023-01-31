// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.17;

contract OnERC721ReceivedSucceeds {
    function onERC721Received(
        address,
        address,
        uint256,
        bytes memory
    ) public pure returns (bytes4) {
        return this.onERC721Received.selector;
    }
}

contract OnERC721ReceivedFails {
    function onERC721Received(
        address,
        address,
        uint256,
        bytes memory
    ) public pure returns (bytes4) {
        return bytes4("");
    }
}
