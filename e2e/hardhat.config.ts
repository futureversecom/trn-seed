import "@nomicfoundation/hardhat-toolbox";
import * as dotenv from "dotenv";
import { utils } from "ethers";
import { HardhatUserConfig, task } from "hardhat/config";

dotenv.config();

// This is a sample Hardhat task. To learn how to create your own go to
// https://hardhat.org/guides/create-task.html
task("accounts", "Prints the list of accounts", async (taskArgs, hre) => {
  const accounts = await hre.ethers.getSigners();

  for (const account of accounts) {
    console.log(account.address);
  }
});

// You need to export an object to set up your config
// Go to https://hardhat.org/config/ to learn more

const config: HardhatUserConfig = {
  solidity: "0.8.17",
  networks: {
    hardhat: {
      chainId: 1337,
      gasPrice: utils.parseUnits("100", "gwei").toNumber(),
    },
    seed: {
      url: "http://localhost:9933",
      accounts: [
        `0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133`, // Alith
        `0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf`, // Bob
      ],
      chainId: 7672,
    },
  },
  etherscan: {
    apiKey: process.env.ETHERSCAN_API_KEY,
  },
};

export default config;
