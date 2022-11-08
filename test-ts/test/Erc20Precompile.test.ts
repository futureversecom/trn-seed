import { expect } from "chai";
import { Contract, ContractFactory, Wallet, utils, BigNumber } from 'ethers';
import web3 from 'web3';
import { JsonRpcProvider, Provider } from "@ethersproject/providers";
import PrecompileCaller from '../artifacts/contracts/Erc20PrecompileCaller.sol/ERC20PrecompileCaller.json';
import { ERC20_ABI } from '../utils';

const xrpTokenAddress = web3.utils.toChecksumAddress('0xCCCCCCCC00000002000000000000000000000000');

describe('ERC20 Precompile', function () {
  let seedSigner: Wallet;
  let xrpToken: Contract;
  let precompileCaller: Contract;
  let jsonProvider: Provider;

  // Setup api instance
  before(async () => {
    // Setup providers for jsonRPCs and WS
    jsonProvider = new JsonRpcProvider(`http://localhost:9933`);

    seedSigner = new Wallet('0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf').connect(jsonProvider); // 'development' seed
    xrpToken = new Contract(xrpTokenAddress, ERC20_ABI, seedSigner);

    let factory = new ContractFactory(PrecompileCaller.abi, PrecompileCaller.bytecode, seedSigner);
    precompileCaller = await factory.deploy();
  });

  it('name, symbol, decimals', async () => {
    expect(
        await xrpToken.decimals()
    ).to.equal(6);

    expect(
        await xrpToken.name()
    ).to.equal("ROOT");

    expect(
        await xrpToken.symbol()
    ).to.equal("ROOT");
  });

  it('XRP transfer, balanceOf', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    const transferAmount = 12345;
    const startingAmount = await xrpToken.balanceOf(seedSigner.address);

    await expect(
        xrpToken.transfer(receiverAddress, transferAmount)
    ).to.emit(xrpToken, 'Transfer').withArgs(seedSigner.address, receiverAddress, transferAmount);

    expect(await xrpToken.balanceOf(receiverAddress)).to.be.equal(transferAmount);
    // Account should be decremented by the sent amount + fees
    expect(await xrpToken.balanceOf(seedSigner.address)).to.be.lessThan(startingAmount - transferAmount);
  })

  it('XRP transfer amounts via EVM', async () => {
    // fund the contract with some XRP
    let endowment = utils.parseEther('8');
    let endowTx = await seedSigner.sendTransaction(
        {
          to: precompileCaller.address,
          value: endowment,
          gasLimit: 500000,
        }
    );
    await endowTx.wait();
    expect(await jsonProvider.getBalance(precompileCaller.address)).to.be.equal(endowment);
    const receiverAddress = await Wallet.createRandom().getAddress();
    let tx = await precompileCaller.sendXRPAmounts(receiverAddress);
    await tx.wait();
  })

  it('approve and transferFrom', async () => {
    let approvedAmount = 12345;
    await expect(
        xrpToken.approve(precompileCaller.address, approvedAmount)
    ).to.emit(xrpToken, 'Approval').withArgs(seedSigner.address, precompileCaller.address, approvedAmount);

    expect(
        await xrpToken.allowance(seedSigner.address, precompileCaller.address)
    ).to.equal(approvedAmount);

    await expect(
        precompileCaller.takeXRP(approvedAmount)
    ).to.emit(xrpToken, 'Transfer').withArgs(seedSigner.address, precompileCaller.address, approvedAmount);

  })

  it('XRP transfer amounts via transaction', async () => {
    const receiverAddress = await Wallet.createRandom().getAddress();
    // pairs of (input amount, actual transferred amount)
    // shows the behaviour of the xrp scaling rules
    let payments = [
      [utils.parseEther('1.000000'), utils.parseEther('1.000000')],
      // transfer smallest unit of xrp
      [utils.parseEther('0.000001'), utils.parseEther('0.000001')],
      // transfer 1.234567 xrp
      [utils.parseEther('1.234567'), utils.parseEther('1.234567')],
      // transfer < the smallest unit of xrp 0.000001, rounds up
      [utils.parseEther('0.00000099'), utils.parseEther('0.000001')],
      // transfer amounts with some part < the smallest unit of xrp
      [utils.parseEther('1.0000005'), utils.parseEther('1.000001')],
      [utils.parseEther('1.00000050000001'), utils.parseEther('1.000001')],
      [utils.parseEther('1.0000009999'), utils.parseEther('1.000001')],
    ];
    let total: BigNumber = BigNumber.from(0);

    for (const [payment, expected] of payments) {
      let tx = await seedSigner.sendTransaction(
          {
            to: receiverAddress,
            value: payment,
          }
      );
      await tx.wait();
      let balance = await jsonProvider.getBalance(receiverAddress);
      total = total.add(expected);
      expect(balance).to.be.equal(total.toString());
    }
  })
});
