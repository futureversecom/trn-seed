import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { IKeyringPair } from "@polkadot/types/types";

const Utils = {
  fromSeed: (seed: string, ss58Format = 42) => {
    const keyring = new Keyring({ type: "sr25519", ss58Format });
    return keyring.addFromUri(seed);
  },
};

export interface IChainProperties {
  ss58Format: number;
  tokenDecimals: number[];
  tokenSymbol: string[];
}

class Chains {
  helper: ChainHelper;

  constructor(helper: ChainHelper) {
    this.helper = helper;
  }

  getChainProperties = (): IChainProperties => {
    const properties = (this.helper.getApi() as any).registry.getChainProperties().toJSON();
    return {
      ss58Format: properties.ss58Format.toJSON(),
      tokenDecimals: properties.tokenDecimals.toJSON(),
      tokenSymbol: properties.tokenSymbol.toJSON(),
    };
  };
}

interface IChainHelper {
  getApi: () => ApiPromise;
  connect: (wsEndpoint: string) => Promise<void>;
  disconnect: () => Promise<void>;
}

class ChainHelper implements IChainHelper {
  api: ApiPromise | null = null;
  utils: typeof Utils;
  chain: Chains;

  constructor() {
    this.utils = Utils;
    this.chain = new Chains(this);
  }

  public getApi = (): ApiPromise => {
    if (this.api === null) throw new Error("API is not connected");
    return this.api;
  };

  public connect = async (wsEndpoint: string): Promise<void> => {
    const wsProvider = new WsProvider(wsEndpoint);
    this.api = await ApiPromise.create({
      provider: wsProvider,
    });
    await this.api.isReadyOrError;
  };

  public disconnect = async (): Promise<void> => {
    if (this.api === null) return;
    await this.api.disconnect();
    this.api = null;
  };
}

type PrivateKey = (seed: string) => Promise<IKeyringPair>;

export const usingPlaygrounds = async (
  url: string,
  code: (helper: IChainHelper, privateKey: PrivateKey) => Promise<void>,
) => {
  const helper = new ChainHelper();

  try {
    await helper.connect(url);
    const ss58Format = helper.chain.getChainProperties().ss58Format;

    const privateKey = async (seed: string) => {
      return helper.utils.fromSeed(seed, ss58Format);
    };

    await code(helper, privateKey);
  } finally {
    await helper.disconnect();
  }
};
