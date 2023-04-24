import { JsonRpcProvider, Provider } from "@ethersproject/providers";
import { expect } from "chai";
import { BigNumber, Contract, ContractFactory, Wallet, utils } from "ethers";
import web3 from "web3";

import PrecompileCaller from "../artifacts/contracts/Erc20PrecompileCaller.sol/ERC20PrecompileCaller.json";
import { BOB_PRIVATE_KEY, ERC20_ABI, NodeProcess, startNode } from "../common";

const xrpTokenAddress = web3.utils.toChecksumAddress("0xCCCCCCCC00000002000000000000000000000000");

describe("ERC20 Precompile", function () {
  let node: NodeProcess;

  let seedSigner: Wallet;
  let xrpToken: Contract;
  let precompileCaller: Contract;
  let jsonProvider: Provider;

  // Setup api instance
  before(async () => {
    node = await startNode();

    await node.wait(); // wait for the node to be ready

    // Setup JSON RPC
    jsonProvider = new JsonRpcProvider(`http://localhost:${node.httpPort}`);
    seedSigner = new Wallet(BOB_PRIVATE_KEY).connect(jsonProvider); // 'development' seed
    xrpToken = new Contract(xrpTokenAddress, ERC20_ABI, seedSigner);

    const factory = new ContractFactory(PrecompileCaller.abi, PrecompileCaller.bytecode, seedSigner);
    precompileCaller = await factory.deploy();
  });

  after(async () => await node.stop());

  it("name, symbol, decimals", async () => {
    expect(await xrpToken.decimals()).to.equal(6);

    expect(await xrpToken.name()).to.equal("XRP");

    expect(await xrpToken.symbol()).to.equal("XRP");
  });

  it("XRP transfer, balanceOf", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const transferAmount = 12345;
    const startingAmount = await xrpToken.balanceOf(seedSigner.address);

    await expect(xrpToken.transfer(receiverAddress, transferAmount))
      .to.emit(xrpToken, "Transfer")
      .withArgs(seedSigner.address, receiverAddress, transferAmount);

    expect(await xrpToken.balanceOf(receiverAddress)).to.be.equal(transferAmount);
    // Account should be decremented by the sent amount + fees
    expect(await xrpToken.balanceOf(seedSigner.address)).to.be.lessThan(startingAmount - transferAmount);
  });

  it("XRP transfer amounts via EVM", async () => {
    // fund the contract with some XRP
    const endowment = utils.parseEther("8");
    const endowTx = await seedSigner.sendTransaction({
      to: precompileCaller.address,
      value: endowment,
      gasLimit: 500000,
    });
    await endowTx.wait();
    expect(await jsonProvider.getBalance(precompileCaller.address)).to.be.equal(endowment);
    const receiverAddress = await Wallet.createRandom().getAddress();
    const tx = await precompileCaller.sendXRPAmounts(receiverAddress);
    await tx.wait();
  });

  it("approve and transferFrom", async () => {
    const approvedAmount = 12345;
    await expect(xrpToken.approve(precompileCaller.address, approvedAmount))
      .to.emit(xrpToken, "Approval")
      .withArgs(seedSigner.address, precompileCaller.address, approvedAmount);

    expect(await xrpToken.allowance(seedSigner.address, precompileCaller.address)).to.equal(approvedAmount);

    await expect(precompileCaller.takeXRP(approvedAmount))
      .to.emit(xrpToken, "Transfer")
      .withArgs(seedSigner.address, precompileCaller.address, approvedAmount);
  });

  it("XRP transfer amounts via transaction", async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    // pairs of (input amount, actual transferred amount)
    // shows the behaviour of the xrp scaling rules
    const payments = [
      [utils.parseEther("1.000000"), utils.parseEther("1.000000")],
      // transfer smallest unit of xrp
      [utils.parseEther("0.000001"), utils.parseEther("0.000001")],
      // transfer 1.234567 xrp
      [utils.parseEther("1.234567"), utils.parseEther("1.234567")],
      // transfer < the smallest unit of xrp 0.000001, rounds up
      [utils.parseEther("0.00000099"), utils.parseEther("0.000001")],
      // transfer amounts with some part < the smallest unit of xrp
      [utils.parseEther("1.0000005"), utils.parseEther("1.000001")],
      [utils.parseEther("1.00000050000001"), utils.parseEther("1.000001")],
      [utils.parseEther("1.0000009999"), utils.parseEther("1.000001")],
    ];
    let total: BigNumber = BigNumber.from(0);

    for (const [payment, expected] of payments) {
      const tx = await seedSigner.sendTransaction({
        to: receiverAddress,
        value: payment,
      });
      await tx.wait();
      const balance = await jsonProvider.getBalance(receiverAddress);
      total = total.add(expected);
      expect(balance).to.be.equal(total.toString());
    }
  });
});
