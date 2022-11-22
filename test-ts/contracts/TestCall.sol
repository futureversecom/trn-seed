pragma solidity ^0.8.14;

interface ITestCaller {
  function set(uint256 _val) external;
  function get() external view returns (uint256);
}

contract TestCall is ITestCaller {
  uint256 internal val;

  function set(uint256 _val) public {
    val = _val;
  }
  function get() public view returns (uint256) {
    return val;
  }

  receive() external payable {}
}

contract TestCallProxy {
  ITestCaller public implementation;

  constructor(ITestCaller _implementation) {
    implementation = _implementation;
  }

  function set(uint256 _val) public {
    implementation.set(_val);
    // (bool success, ) = address(implementation).staticcall(abi.encodeWithSignature("set(uint256)", _val));
    // require(success, "failed staticcall");
  }
  function get() public view returns (uint256) {
    return implementation.get();
  }

  function setWithAddress(ITestCaller _tester, uint256 _val) public {
    _tester.set(_val);
    // (bool success, ) = address(_tester).staticcall(abi.encodeWithSignature("set(uint256)", _val));
    // require(success, "failed staticcall");
  }
  function getWithAddress(ITestCaller _tester) public view returns (uint256) {
    return _tester.get();
  }
}
